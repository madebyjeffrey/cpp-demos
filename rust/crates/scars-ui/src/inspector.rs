//! A live mounted hierarchy with owned diagnostics and stable outline navigation.
mod editor;
mod overrides;
mod snapshot;
use crate::state::{Observer, Tracking};
use crate::{Error, Host, Node, NodeId, Point, State, TreeModel, TreeNode, TreeOptions, View};
pub use overrides::{StyleOverride, StyleProperty};
pub use snapshot::{Inspection, InspectionCollection, InspectionFlexLine, InspectionOverlay};
use std::{
    cell::{Cell, RefCell},
    collections::{HashMap, HashSet},
    rc::Rc,
};
type Shape = Vec<(NodeId, Option<NodeId>, String)>;
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum InspectionHighlight {
    #[default]
    Selection,
    BoxModel,
    Flex,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum InspectorPage {
    #[default]
    Details,
    Layout,
    Style,
}
struct Data {
    attached: Cell<u64>,
    nodes: State<Vec<Inspection>>,
    tree: State<Option<TreeModel>>,
    shape: RefCell<Shape>,
    last_node: Cell<Option<NodeId>>,
    selection_key: RefCell<Option<String>>,
    capturing: Cell<bool>,
    observer: Rc<Observer>,
    page: State<String>,
    editors: RefCell<HashMap<(NodeId, StyleProperty), crate::Draft<String>>>,
}
#[derive(Clone)]
pub struct Inspector {
    pub picking: State<bool>,
    pub selected: State<Option<Inspection>>,
    pub highlight: State<InspectionHighlight>,
    pub(crate) overrides: State<HashMap<NodeId, StyleOverride>>,
    data: Rc<Data>,
}
impl Inspector {
    pub fn new() -> Self {
        let selected = State::new(None);
        let data = Rc::new(Data {
            attached: Cell::new(0),
            nodes: State::new(vec![]),
            tree: State::new(None),
            shape: RefCell::new(vec![]),
            last_node: Cell::new(None),
            selection_key: RefCell::new(None),
            capturing: Cell::new(false),
            observer: Observer::new(),
            page: State::new("details".into()),
            editors: RefCell::new(HashMap::new()),
        });
        let weak = Rc::downgrade(&data);
        let selection = selected.clone();
        data.observer.set_wake(Rc::new(move || {
            let Some(data) = weak.upgrade() else { return };
            data.observer.dirty.set(false);
            let Some(tree) = data.tree.get() else { return };
            let key = tree.selected().first().cloned();
            if data.capturing.get() || key == *data.selection_key.borrow() {
                return;
            }
            let target = data.nodes.with(|nodes| {
                nodes
                    .iter()
                    .find(|n| Some(n.id.to_string()) == key)
                    .cloned()
            });
            data.last_node.set(target.as_ref().map(|n| n.id));
            *data.selection_key.borrow_mut() = key;
            selection.set(target);
        }));
        Self {
            picking: State::new(false),
            selected,
            highlight: State::new(InspectionHighlight::Selection),
            overrides: State::new(HashMap::new()),
            data,
        }
    }
    /// Last laid-out source hierarchy, excluding the inspector's own controls.
    pub fn nodes(&self) -> Vec<Inspection> {
        self.data.nodes.get()
    }
    pub(crate) fn picking_for(&self, source: u64) -> bool {
        (self.data.attached.get() == 0 || self.data.attached.get() == source) && self.picking.get()
    }
    /// Select a mounted node and reveal its ancestors in the hierarchy.
    pub fn select(&self, id: NodeId) -> bool {
        let Some(info) = self.nodes().into_iter().find(|n| n.id == id) else {
            return false;
        };
        crate::state::batch(|| {
            if let Some(tree) = self.data.tree.get() {
                let key = id.to_string();
                let _ = tree.reveal(&key);
                tree.select(&key, false);
            }
            self.selected.set(Some(info));
        });
        true
    }
    pub fn view(&self) -> View {
        let mut children = vec![
            View::label(format!(
                "Inspector · {} mounted nodes",
                self.data.nodes.with(Vec::len)
            ))
            .bold(true),
            View::checkbox("Pick a control · Escape exits", self.picking.binding()),
        ];
        children.push(self.data.tree.get().map_or_else(
            || {
                View::label(if self.data.attached.get() == 0 {
                    "No source attached"
                } else {
                    "Hierarchy available after layout"
                })
                .height(180)
            },
            |tree| {
                tree.view()
                    .height(180)
                    .accessible_label("Mounted hierarchy")
            },
        ));
        if let Some(info) = self.selected.get() {
            let mut lines = vec![
                format!(
                    "{:?} #{} · {}",
                    info.kind,
                    info.id,
                    info.key.as_deref().unwrap_or("unkeyed")
                ),
                format!(
                    "Bounds: {:.1} × {:.1} at {:.1}, {:.1}",
                    info.bounds.width, info.bounds.height, info.bounds.x, info.bounds.y
                ),
                format!(
                    "Clip: {:.1} × {:.1} at {:.1}, {:.1}",
                    info.clip.width, info.clip.height, info.clip.x, info.clip.y
                ),
                format!(
                    "Text: {:.1} × {:.1}; baseline {:.1}",
                    info.text_bounds.width, info.text_bounds.height, info.baseline
                ),
                format!(
                    "Enabled {}; hidden {}; readonly {}",
                    info.enabled, info.hidden, info.read_only
                ),
                format!(
                    "Focused {}; pressed {}; in scope {}",
                    info.focused, info.pressed, info.in_scope
                ),
            ];
            if !info.label.is_empty() {
                lines.push(format!("Label: {}", info.label));
            }
            if !info.value.is_empty() {
                lines.push(format!("Value: {}", info.value));
            }
            if let Some(selected) = info.selected {
                lines.push(format!("Selected: {selected}"));
            }
            if let Some(s) = info.text_selection {
                lines.push(format!(
                    "Text selection: {}..{}; marked {}",
                    s.anchor, s.caret, info.marked
                ));
            }
            if let Some(v) = &info.validation {
                lines.push(format!(
                    "Validation: {:?}; dirty {}\n{}",
                    v.status,
                    v.dirty,
                    v.message.as_deref().unwrap_or("")
                ));
            }
            if let Some(s) = info.scroll {
                lines.push(format!(
                    "Scroll: {:?}; offset {:.1}, {:.1}; extent {:.1} × {:.1}",
                    s.axes, s.offset.x, s.offset.y, s.content.width, s.content.height
                ));
            }
            if let Some(o) = &info.overlay {
                lines.push(format!(
                    "Overlay: active {}; owner {:?}; anchor {:?}\n{:?}",
                    o.active, o.owner, o.anchor, o.options
                ));
            }
            if let Some(c) = &info.collection {
                lines.push(format!("Collection: {} known, {} rows, {} columns\nWindow {:?}; {} realized, {} pinned\n{} selected; cursor {:?}\nOffset {:.1}; viewport {:.1}; row height {:.1}",c.total,c.rows,c.columns,c.window,c.realized,c.pinned,c.selected,c.cursor,c.offset,c.viewport,c.row_height));
            }
            lines.push(format!(
                "Measured: {:.1} × {:.1}; content: {:.1} × {:.1}",
                info.measured.width,
                info.measured.height,
                info.content_bounds.width,
                info.content_bounds.height
            ));
            lines.push(format!(
                "Resolved text: {:.1}pt, color #{:08X}, bold {}",
                info.text_style.size, info.text_style.color, info.text_style.bold
            ));
            lines.push(format!("{} explicit overrides", info.overridden.len()));
            if let Some(error) = &info.override_error {
                lines.push(format!("Override suspended: {error}"));
            }
            lines.extend(info.details.clone());
            let (read, write) = (self.highlight.clone(), self.highlight.clone());
            children.push(
                View::segmented(
                    crate::Binding::new(
                        move || {
                            match read.get() {
                                InspectionHighlight::Selection => "outline",
                                InspectionHighlight::BoxModel => "box",
                                InspectionHighlight::Flex => "flex",
                            }
                            .into()
                        },
                        move |value: String| {
                            write.set(match value.as_str() {
                                "box" => InspectionHighlight::BoxModel,
                                "flex" => InspectionHighlight::Flex,
                                _ => InspectionHighlight::Selection,
                            })
                        },
                    ),
                    vec![
                        crate::OptionItem::new("outline", "Outline"),
                        crate::OptionItem::new("box", "Box"),
                        crate::OptionItem::new("flex", "Flex"),
                    ],
                )
                .accessible_label("Inspector highlight"),
            );
            for (index, line) in info.flex_lines.iter().enumerate() {
                lines.push(format!(
                    "Flex line {}: {} items; gap {:.1}; free {:.1}",
                    index + 1,
                    line.items.len(),
                    line.gap,
                    line.free
                ));
            }
            children.push(
                crate::View::segmented(
                    self.data.page.binding(),
                    vec![
                        crate::OptionItem::new("details", "Details"),
                        crate::OptionItem::new("layout", "Layout"),
                        crate::OptionItem::new("style", "Style"),
                    ],
                )
                .accessible_label("Inspector page"),
            );
            let content = if self.data.page.get() == "details" {
                View::column(
                    lines
                        .into_iter()
                        .map(|s| View::label(s).wrap(true))
                        .collect(),
                )
                .gap(8.)
            } else {
                self.property_view(&info)
            };
            children.push(
                View::scroll(vec![content])
                    .grow(1.)
                    .basis(0)
                    .min_height(0.)
                    .accessible_label("Selected control details"),
            );
            let (reset, all) = (self.clone(), self.clone());
            let id = info.id;
            children.push(View::row(vec![
                View::button("Reset node", move || reset.reset_override(id)),
                View::button("Reset all", move || all.reset_all_overrides()),
            ]));
        } else {
            children
                .push(View::label("Browse the hierarchy or pick a rendered control.").wrap(true));
        }
        let mut view = View::panel(children).padding(16.).gap(8.).min_height(0.);
        Rc::make_mut(&mut view.0).inspector_panel = true;
        view
    }
    pub(crate) fn retire(&self, source: u64) {
        if self.data.attached.get() != source {
            return;
        }
        self.data.attached.set(0);
        self.data
            .observer
            .epoch
            .set(self.data.observer.epoch.get().wrapping_add(1));
        self.data.observer.dirty.set(false);
        self.data.shape.borrow_mut().clear();
        self.data.last_node.set(None);
        *self.data.selection_key.borrow_mut() = None;
        crate::state::batch(|| {
            self.data.nodes.set(vec![]);
            self.data.tree.set(None);
            self.selected.set(None);
            self.overrides.set(HashMap::new());
            self.picking.set(false);
            self.highlight.set(InspectionHighlight::Selection);
        });
        self.data.editors.borrow_mut().clear();
    }
    fn observe_tree(&self, tree: &TreeModel) {
        let tracking = Tracking::begin();
        let _ = tree.selected();
        let deps = tracking.finish();
        let epoch = self.data.observer.epoch.get().wrapping_add(1);
        for dep in deps {
            dep.subscribe(&self.data.observer, epoch);
        }
        self.data.observer.epoch.set(epoch);
        self.data.observer.dirty.set(false);
    }
    fn capture(&self, nodes: Vec<Inspection>) -> Result<(), Error> {
        if self.data.attached.get() == 0 && !self.picking.get() && self.selected.get().is_none() {
            return Ok(());
        }
        crate::state::batch(|| {
            if self.data.capturing.replace(true) {
                return Ok(());
            }
            struct Reset<'a>(&'a Cell<bool>);
            impl Drop for Reset<'_> {
                fn drop(&mut self) {
                    self.0.set(false)
                }
            }
            let _reset = Reset(&self.data.capturing);
            let tree = if let Some(tree) = self.data.tree.get() {
                tree
            } else {
                let tree = TreeModel::with_options(
                    vec![],
                    TreeOptions {
                        row_height: 28.,
                        visible_rows: 9,
                        indentation: 14.,
                        ..Default::default()
                    },
                )?;
                self.data.tree.set(Some(tree.clone()));
                self.observe_tree(&tree);
                tree
            };
            let cursor = tree.selected().first().cloned();
            let shape: Shape = nodes
                .iter()
                .map(|n| (n.id, n.parent, n.outline_label()))
                .collect();
            let changed = *self.data.shape.borrow() != shape;
            let initial = self.data.shape.borrow().is_empty();
            let live: HashSet<_> = nodes.iter().map(|n| n.id).collect();
            self.overrides
                .update(|m| m.retain(|id, _| live.contains(id)));
            self.data
                .editors
                .borrow_mut()
                .retain(|(id, _), _| live.contains(id));
            self.data.nodes.set(nodes.clone());
            if changed {
                *self.data.shape.borrow_mut() = shape;
                tree.reconcile_mounted(outline(&nodes))?;
                if initial {
                    for n in nodes.iter().filter(|n| n.parent.is_none()) {
                        tree.expand(&n.id.to_string())?;
                    }
                }
            }
            let requested = self.selected.get().map(|n| n.id);
            let chosen = if requested != self.data.last_node.get() {
                requested
            } else if cursor != *self.data.selection_key.borrow() {
                cursor.as_ref().and_then(|key| {
                    nodes
                        .iter()
                        .find(|n| n.id.to_string() == *key)
                        .map(|n| n.id)
                })
            } else {
                requested
            };
            let target = nodes.into_iter().find(|n| Some(n.id) == chosen);
            let version = self.selected.version();
            if chosen != self.data.last_node.get()
                || target
                    .as_ref()
                    .is_some_and(|n| !tree.selected().contains(&n.id.to_string()))
                || target.is_none() && tree.selection_count() != 0
            {
                if let Some(info) = &target {
                    tree.reveal(&info.id.to_string())?;
                    tree.select(&info.id.to_string(), false);
                } else {
                    tree.clear_selection();
                }
            }
            if self.selected.version() != version {
                return Ok(());
            }
            self.data.last_node.set(target.as_ref().map(|n| n.id));
            *self.data.selection_key.borrow_mut() = tree.selected().first().cloned();
            self.selected.set(target);
            Ok(())
        })
    }
}
fn outline(nodes: &[Inspection]) -> Vec<TreeNode> {
    let mut children: HashMap<Option<NodeId>, Vec<&Inspection>> = HashMap::new();
    for n in nodes {
        children.entry(n.parent).or_default().push(n);
    }
    fn build(
        parent: Option<NodeId>,
        children: &HashMap<Option<NodeId>, Vec<&Inspection>>,
    ) -> Vec<TreeNode> {
        children
            .get(&parent)
            .into_iter()
            .flatten()
            .map(|n| {
                TreeNode::new(n.id.to_string(), n.outline_label())
                    .children(build(Some(n.id), children))
            })
            .collect()
    }
    build(None, &children)
}
impl Default for Inspector {
    fn default() -> Self {
        Self::new()
    }
}
impl Host {
    pub fn set_inspector(&self, inspector: Inspector) {
        let previous_source = inspector.data.attached.get();
        if previous_source != 0 && previous_source != self.0.inspection_id {
            inspector.retire(previous_source);
        }
        inspector.data.attached.set(self.0.inspection_id);
        let old = self.0.inspector.replace(inspector.clone());
        if !Rc::ptr_eq(&old.data, &inspector.data) {
            let id = self.0.inspection_id;
            crate::state::cleanup(move || old.retire(id));
        }
        self.0.observer.dirty.set(true);
        self.redraw();
    }
    pub fn inspector(&self) -> Inspector {
        self.0.inspector.borrow().clone()
    }
    fn inspector_node(&self, id: NodeId) -> bool {
        let m = self.0.mounted.borrow();
        let mut current = Some(id);
        while let Some(id) = current {
            if m.descriptions.get(&id).is_some_and(|d| d.inspector_panel) {
                return true;
            }
            current = m.nodes.iter().find(|n| n.id == id).and_then(|n| n.parent);
        }
        false
    }
    pub(crate) fn inspector_hit(&self, point: Point) -> Result<bool, Error> {
        Ok(self
            .scene()?
            .iter()
            .rev()
            .find(|n| {
                !n.hidden && !n.layout_hidden && n.bounds.contains(point) && n.clip.contains(point)
            })
            .is_some_and(|n| self.inspector_node(n.id)))
    }
    /// Current inspected source node; another host's selection never paints here.
    pub fn inspection(&self) -> Option<Inspection> {
        let i = self.inspector();
        if i.data.attached.get() != 0 && i.data.attached.get() != self.0.inspection_id {
            None
        } else {
            i.selected.get()
        }
    }
    fn inspect_node(&self, n: &Node, nodes: &[Node]) -> Inspection {
        let mut info = Inspection::from_node(n, nodes);
        if let Some(d) = self.0.mounted.borrow().descriptions.get(&n.id) {
            info.source_style = d.style;
            if let Some(patch) = self.inspector().overrides.with(|m| m.get(&n.id).copied()) {
                info.overridden = patch.properties();
                info.override_error = crate::host::validate_style(patch.apply(d.style))
                    .err()
                    .map(|e| e.0);
            }
        }
        info.in_scope = self.in_scope(n.id);
        if let Some(o) = &mut info.overlay {
            o.active = self.active_modal() == Some(n.id);
        }
        if let Some(editor) = self.0.editors.borrow().get(&n.id) {
            info.text_selection = Some(editor.selection);
            info.marked = editor.marked.is_some();
        }
        info
    }
    pub(crate) fn capture_inspector(&self) -> Result<(), Error> {
        let inspector = self.inspector();
        if inspector.data.attached.get() == 0
            && !inspector.picking.get()
            && inspector.selected.get().is_none()
        {
            return Ok(());
        }
        let owner = inspector.data.attached.get();
        if owner != 0 && owner != self.0.inspection_id {
            return Ok(());
        }
        inspector.data.attached.set(self.0.inspection_id);
        let nodes = self.snapshot_mounted();
        let mut excluded = HashSet::new();
        {
            let m = self.0.mounted.borrow();
            for n in &nodes {
                if m.descriptions[&n.id].inspector_panel
                    || n.parent.is_some_and(|p| excluded.contains(&p))
                {
                    excluded.insert(n.id);
                }
            }
        }
        let info = crate::state::read_only(|| {
            nodes
                .iter()
                .filter(|n| !excluded.contains(&n.id))
                .map(|n| self.inspect_node(n, &nodes))
                .collect()
        });
        inspector.capture(info)
    }
    pub fn pick(&self, point: Point) -> Result<Option<Inspection>, Error> {
        if self.inspector_hit(point)? {
            return Ok(None);
        }
        let nodes = self.scene()?;
        Ok(crate::state::read_only(|| {
            nodes
                .iter()
                .rev()
                .find(|n| {
                    !n.hidden
                        && !n.layout_hidden
                        && n.bounds.contains(point)
                        && n.clip.contains(point)
                        && self.in_scope(n.id)
                        && !self.inspector_node(n.id)
                })
                .map(|n| self.inspect_node(n, &nodes))
        }))
    }
}
