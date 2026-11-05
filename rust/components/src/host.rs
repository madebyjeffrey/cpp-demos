use crate::{
    Bitmap, ContentMode, Kind, Rect, Size, Style, TextLayout, TextStyle, Theme, Value, View,
    state::{Observer, Tracking},
    view::{Data, Description},
};
use std::{
    cell::{Cell, RefCell},
    collections::{HashMap, HashSet},
    fmt,
    rc::Rc,
    sync::atomic::{AtomicU64, Ordering},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct NodeId(u64);
impl fmt::Display for NodeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}
#[derive(Clone)]
pub struct Node {
    pub id: NodeId,
    pub parent: Option<NodeId>,
    pub(crate) slot: usize,
    pub depth: usize,
    pub kind: Kind,
    pub key: Option<String>,
    pub text: String,
    pub value: Value,
    pub bounds: Rect,
    pub clip: Rect,
    pub text_bounds: Rect,
    pub baseline: f64,
    pub spans: Option<Vec<crate::ParagraphSpan>>,
    pub(crate) rich: Option<Vec<crate::rich_text::Run>>,
    pub text_style: TextStyle,
    pub accent: u32,
    pub density: crate::Density,
    pub style: Style,
    pub enabled: bool,
    pub hidden: bool,
    pub(crate) layout_hidden: bool,
    pub read_only: bool,
    pub secure: bool,
    pub revealed: bool,
    pub field_kind: crate::TextFieldKind,
    pub field_action_width: f64,
    pub multiline: bool,
    pub anchor: Option<String>,
    pub scroll: Option<crate::ScrollInfo>,
    pub(crate) scroll_axes: crate::ScrollAxes,
    pub(crate) scrollbars: bool,
    pub collection: Option<crate::ListModel>,
    pub label: String,
    pub focused: bool,
    pub pressed: bool,
    pub hovered: bool,
    pub range: Option<(f64, f64, f64)>,
    pub image: Option<(Bitmap, ContentMode)>,
}
impl Node {
    pub fn edit_bounds(&self) -> Rect {
        self.bounds.inset(crate::Insets {
            top: 6.,
            bottom: 6.,
            left: 10.,
            right: 10. + self.field_action_width,
        })
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Error(pub String);
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}
impl std::error::Error for Error {}
#[derive(Clone, Hash, PartialEq, Eq)]
enum Segment {
    Key(String),
    Index(usize),
}
type Path = Vec<(Segment, Kind)>;
#[derive(Default)]
pub(crate) struct Mounted {
    identities: HashMap<Path, NodeId>,
    pub nodes: Vec<Node>,
    pub descriptions: HashMap<NodeId, Rc<Description>>,
    pub layouts: HashMap<NodeId, TextLayout>,
}
pub(crate) struct Inner {
    builder: Box<dyn Fn() -> View>,
    pub observer: Rc<Observer>,
    pub mounted: RefCell<Mounted>,
    building: Cell<bool>,
    generation: Cell<u64>,
    pub theme: Cell<Theme>,
    pub viewport: Cell<Size>,
    pub interaction: RefCell<crate::input::Interaction>,
    pub autofocus: Cell<Option<NodeId>>,
    pub inspector: RefCell<crate::Inspector>,
    pub scroll: RefCell<HashMap<NodeId, crate::Point>>,
    pub deadlines: RefCell<Vec<crate::deadlines::Deadline>>,
    pub fields: RefCell<HashMap<NodeId, crate::fields::FieldState>>,
    pub editors: RefCell<HashMap<NodeId, crate::editing::Editor>>,
}
/// Reactive descriptions, layout and input in logical points. The host is !Send/!Sync.
/// Call layout with a pure renderer before geometric input/semantic queries.
#[derive(Clone)]
pub struct Host(pub(crate) Rc<Inner>);
impl Host {
    pub fn new(builder: impl Fn() -> View + 'static) -> Self {
        Self(Rc::new(Inner {
            builder: Box::new(builder),
            observer: Observer::new(),
            mounted: RefCell::new(Mounted::default()),
            building: Cell::new(false),
            generation: Cell::new(0),
            theme: Cell::new(Theme::default()),
            viewport: Cell::new(Size::default()),
            interaction: RefCell::new(Default::default()),
            editors: RefCell::new(HashMap::new()),
            deadlines: RefCell::new(Vec::new()),
            fields: RefCell::new(HashMap::new()),
            scroll: RefCell::new(HashMap::new()),
            inspector: RefCell::new(crate::Inspector::new()),
            autofocus: Cell::new(None),
        }))
    }
    pub fn on_invalidate(&self, wake: impl Fn() + 'static) {
        self.0.observer.set_wake(Rc::new(wake));
    }
    pub(crate) fn redraw(&self) {
        self.0.observer.redraw();
    }
    pub fn generation(&self) -> u64 {
        self.0.generation.get()
    }
    pub fn theme(&self) -> Theme {
        self.0.theme.get()
    }
    pub fn set_theme(&self, theme: Theme) {
        if self.0.theme.replace(theme) != theme {
            self.redraw();
        }
    }
    pub fn refresh(&self) -> Result<bool, Error> {
        if self.0.building.replace(true) {
            return Err(Error("Host cannot rebuild during view construction".into()));
        }
        struct Guard<'a>(&'a Cell<bool>);
        impl Drop for Guard<'_> {
            fn drop(&mut self) {
                self.0.set(false)
            }
        }
        let _guard = Guard(&self.0.building);
        if !self.0.observer.dirty.get() {
            return Ok(false);
        }
        let tracking = Tracking::begin();
        let view = (self.0.builder)();
        let identities = self.0.mounted.borrow().identities.clone();
        let mut next = Mounted::default();
        mount(
            &view,
            Vec::new(),
            None,
            0,
            true,
            false,
            &identities,
            &mut next,
        )?;
        let inspector = self.0.inspector.borrow();
        let _ = inspector.overrides.get();
        let _ = inspector.selected.get();
        let _ = inspector.picking.get();
        drop(inspector);
        let deps = tracking.finish();
        let epoch = self.0.observer.epoch.get().wrapping_add(1);
        for dep in deps {
            dep.subscribe(&self.0.observer, epoch);
        }
        self.0.observer.epoch.set(epoch);
        // User getters and destructors never run under a mounted-tree borrow.
        let old = self.0.mounted.replace(next);
        if let Some(id) = {
            let m = self.0.mounted.borrow();
            m.nodes
                .iter()
                .rev()
                .find(|n| {
                    m.descriptions[&n.id].autofocus
                        && n.enabled
                        && !n.hidden
                        && !n.layout_hidden
                        && !old.descriptions.contains_key(&n.id)
                })
                .map(|n| n.id)
        } {
            self.0.autofocus.set(Some(id));
        }
        self.0
            .generation
            .set(self.0.generation.get().wrapping_add(1));
        self.0.observer.dirty.set(false);
        {
            let mounted = self.0.mounted.borrow();
            let mut editors = self.0.editors.borrow_mut();
            self.0
                .scroll
                .borrow_mut()
                .retain(|id, _| mounted.nodes.iter().any(|n| n.id == *id));
            editors.retain(|id, _| {
                mounted
                    .descriptions
                    .get(id)
                    .is_some_and(|d| d.kind == Kind::TextField)
            });
            for n in &mounted.nodes {
                if let Value::Text(value) = &n.value {
                    let editor = editors.entry(n.id).or_default();
                    let mut options = mounted.descriptions[&n.id].text_options;
                    options.multiline = n.multiline;
                    options.read_only = n.read_only;
                    editor.set_options(options);
                    editor.sync(value);
                }
            }
            let mut i = self.0.interaction.borrow_mut();
            if i.focus.is_some_and(|id| {
                !mounted.nodes.iter().any(|n| {
                    n.id == id
                        && n.enabled
                        && !n.hidden
                        && !n.layout_hidden
                        && (focusable(n.kind) || n.kind == Kind::Scroll)
                })
            }) {
                i.focus = None;
            }
            if i.pressed.is_some_and(|id| {
                !mounted
                    .nodes
                    .iter()
                    .any(|n| n.id == id && n.enabled && !n.hidden && !n.layout_hidden)
            }) {
                i.pressed = None;
            }
        }
        drop(old);
        self.reconcile_capture();
        self.reconcile_fields();
        Ok(true)
    }
    pub fn snapshot(&self) -> Result<Vec<Node>, Error> {
        self.refresh()?;
        let i = self.0.interaction.borrow();
        let editors = self.0.editors.borrow();
        let mut nodes = self.0.mounted.borrow().nodes.clone();
        for n in &mut nodes {
            n.hidden |= n.layout_hidden;
            n.revealed = self
                .0
                .fields
                .borrow()
                .get(&n.id)
                .is_some_and(|s| s.revealed);
            if n.key.as_deref() == Some("$field-action")
                && n.parent
                    .is_some_and(|p| self.0.fields.borrow().get(&p).is_some_and(|s| s.revealed))
            {
                n.text = "Hide".into();
                n.label = "Hide password".into();
            }
            n.focused = i.focus == Some(n.id) && i.active;
            n.pressed = i.pressed == Some(n.id);
            n.hovered = i.hovered == Some(n.id);
            if let Some(e) = editors.get(&n.id) {
                n.value = Value::Text(e.text.clone());
            }
        }
        Ok(nodes)
    }
    pub fn activate(&self, id: NodeId) -> Result<bool, Error> {
        self.refresh()?;
        let target = {
            let m = self.0.mounted.borrow();
            m.nodes
                .iter()
                .find(|n| {
                    n.id == id && n.enabled && !n.hidden && !n.layout_hidden && self.in_scope(n.id)
                })
                .and_then(|n| {
                    m.descriptions
                        .get(&id)
                        .map(|d| (d.clone(), n.value.clone()))
                })
        };
        let Some((d, value)) = target else {
            return Ok(false);
        };
        if let Some(action) = d.field_action {
            return self.field_action(id, action);
        }
        match (&d.data, value) {
            (Data::Bool(b), Value::Bool(v)) => b.set(
                if matches!(d.kind, Kind::Radio | Kind::Segment | Kind::CollectionRow) {
                    true
                } else {
                    !v
                },
            ),
            (Data::Check(b), Value::Check(v)) => b.set(if v == crate::CheckState::Checked {
                crate::CheckState::Unchecked
            } else {
                crate::CheckState::Checked
            }),
            (Data::Text(_), _) => {
                self.focus(id)?;
            }
            _ => {
                if let Some(action) = &d.action {
                    action()
                } else {
                    return Ok(false);
                }
            }
        }
        self.redraw();
        Ok(true)
    }
    pub fn set_number(&self, id: NodeId, value: f64) -> Result<bool, Error> {
        self.refresh()?;
        if !value.is_finite() {
            return Ok(false);
        }
        let data = {
            let m = self.0.mounted.borrow();
            m.nodes
                .iter()
                .find(|n| {
                    n.id == id
                        && n.enabled
                        && !n.hidden
                        && !n.layout_hidden
                        && !n.read_only
                        && self.in_scope(n.id)
                        && n.kind == Kind::Slider
                })
                .and_then(|_| m.descriptions.get(&id))
                .map(|d| d.data.clone())
        };
        let Some(Data::Number(binding, min, max, step)) = data else {
            return Ok(false);
        };
        let value = if step > 0. {
            min + ((value - min) / step).round() * step
        } else {
            value
        };
        binding.set(value.clamp(min, max));
        self.redraw();
        Ok(true)
    }
    pub fn needs_animation(&self) -> bool {
        self.0.interaction.borrow().active
            && self.0.mounted.borrow().nodes.iter().any(|n| {
                n.kind == Kind::Progress
                    && n.value == Value::None
                    && !n.hidden
                    && !n.layout_hidden
                    && n.enabled
            })
    }
}
pub(crate) fn focusable(kind: Kind) -> bool {
    matches!(
        kind,
        Kind::Button
            | Kind::CollectionRow
            | Kind::Toggle
            | Kind::CheckBox
            | Kind::Radio
            | Kind::Segment
            | Kind::Switch
            | Kind::Slider
            | Kind::TextField
    )
}
#[allow(clippy::too_many_arguments)]
fn mount(
    view: &View,
    path: Path,
    parent: Option<NodeId>,
    index: usize,
    enabled: bool,
    hidden: bool,
    old: &HashMap<Path, NodeId>,
    next: &mut Mounted,
) -> Result<(), Error> {
    static NEXT: AtomicU64 = AtomicU64::new(1);
    let d = &view.0;
    if !d.visible {
        return Ok(());
    }
    let segment = match &d.key {
        Some(k) if k.is_empty() => return Err(Error("Keys must not be empty".into())),
        Some(k) => Segment::Key(k.clone()),
        None => Segment::Index(index),
    };
    validate(d)?;
    let mut path = path;
    path.push((segment, d.kind));
    let id = old
        .get(&path)
        .copied()
        .unwrap_or_else(|| NodeId(NEXT.fetch_add(1, Ordering::Relaxed)));
    let value = d.data.value();
    if let Value::Number(v) = value
        && !v.is_finite()
    {
        return Err(Error("Numeric values must be finite".into()));
    }
    let range = if let Data::Number(_, a, b, s) = &d.data {
        Some((*a, *b, *s))
    } else {
        None
    };
    let field_children = crate::fields::children(d, &value);
    let image = if let Data::Image(b, m) = &d.data {
        Some((b.clone(), *m))
    } else {
        None
    };
    next.identities.insert(path.clone(), id);
    next.descriptions.insert(id, d.clone());
    next.nodes.push(Node {
        id,
        parent,
        slot: index,
        depth: path.len() - 1,
        kind: d.kind,
        key: d.key.clone(),
        text: d.text.clone(),
        value,
        bounds: Rect::default(),
        clip: Rect::default(),
        text_bounds: Rect::default(),
        baseline: 0.,
        spans: None,
        rich: d.rich.clone(),
        text_style: TextStyle {
            size: 15.,
            color: 0xff000000,
            bold: false,
            wrap: false,
            ..TextStyle::default()
        },
        accent: crate::Theme::default().accent,
        density: crate::Density::Standard,
        style: d.style,
        enabled: enabled && d.enabled,
        hidden: hidden || d.hidden,
        layout_hidden: false,
        read_only: d.read_only,
        secure: d.secure,
        revealed: false,
        field_kind: d.field.kind,
        field_action_width: field_children
            .last()
            .filter(|v| v.0.field_action.is_some())
            .map_or(0., |v| v.0.style.width.resolve(100.).unwrap_or(0.) + 6.),
        multiline: d.multiline,
        anchor: d.anchor.clone(),
        scroll: None,
        scroll_axes: d.scroll_axes,
        scrollbars: d.scrollbars,
        collection: d.collection.clone(),
        label: if d.accessible.is_empty() {
            d.text.clone()
        } else {
            d.accessible.clone()
        },
        focused: false,
        pressed: false,
        hovered: false,
        range,
        image,
    });
    let mut keys = HashSet::new();
    for (index, child) in field_children.iter().enumerate() {
        if let Some(key) = &child.0.key
            && !keys.insert(key)
        {
            return Err(Error(format!("Duplicate sibling key: {key}")));
        }
        mount(
            child,
            path.clone(),
            Some(id),
            index,
            enabled && d.enabled,
            hidden || d.hidden,
            old,
            next,
        )?;
    }
    Ok(())
}
pub(crate) fn validate(d: &Description) -> Result<(), Error> {
    if d.field.kind == crate::TextFieldKind::Search
        && std::time::Instant::now()
            .checked_add(d.field.delay)
            .is_none()
    {
        return Err(Error("Search debounce is too large".into()));
    }
    let s = d.style;
    if let Some(runs) = &d.rich {
        for run in runs {
            if let crate::rich_text::Run::Text(span) = run {
                span.style.validate()?;
            }
        }
    }
    if [
        s.padding.top,
        s.padding.right,
        s.padding.bottom,
        s.padding.left,
        s.margin.top,
        s.margin.right,
        s.margin.bottom,
        s.margin.left,
        s.gap,
        s.grow,
        s.shrink,
        s.min_width,
        s.min_height,
        s.radius,
    ]
    .iter()
    .any(|v| !v.is_finite() || *v < 0.)
        || s.max_width.is_nan()
        || s.max_height.is_nan()
        || s.max_width < s.min_width
        || s.max_height < s.min_height
        || !s.letter_spacing.is_finite()
        || s.line_height.is_some_and(|v| !v.is_finite() || v <= 0.)
        || s.font_size.is_some_and(|v| !v.is_finite() || v <= 0.)
    {
        return Err(Error("Invalid layout/style value".into()));
    }
    for length in [s.width, s.height, s.basis] {
        if let crate::Length::Points(v) | crate::Length::Percent(v) = length
            && (!v.is_finite() || v < 0.)
        {
            return Err(Error("Dimensions must be finite and nonnegative".into()));
        }
    }
    for length in [s.left, s.right, s.top, s.bottom] {
        if let crate::Length::Points(v) | crate::Length::Percent(v) = length
            && !v.is_finite()
        {
            return Err(Error("Offsets must be finite".into()));
        }
    }
    if let Data::Number(_, min, max, step) = &d.data
        && (!min.is_finite() || !max.is_finite() || !step.is_finite() || min > max || *step < 0.)
    {
        return Err(Error("Invalid numeric range".into()));
    }
    Ok(())
}
