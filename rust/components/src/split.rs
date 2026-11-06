//! Two retained panes and one adjustable divider. Geometry is in logical points.
use crate::{Binding, Error, Host, Kind, NodeId, Orientation, Point, Rect, State, View};
use std::rc::Rc;
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SplitPane {
    #[default]
    None,
    First,
    Second,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SplitOptions {
    pub orientation: Orientation,
    pub first_min: f64,
    pub second_min: f64,
    pub divider_width: f64,
    pub keyboard_step: f64,
}
impl Default for SplitOptions {
    fn default() -> Self {
        Self {
            orientation: Orientation::Horizontal,
            first_min: 100.,
            second_min: 100.,
            divider_width: 12.,
            keyboard_step: 10.,
        }
    }
}
impl SplitOptions {
    pub(crate) fn validate(self) -> Result<(), Error> {
        if ![
            self.first_min,
            self.second_min,
            self.divider_width,
            self.keyboard_step,
        ]
        .into_iter()
        .all(f64::is_finite)
            || self.first_min < 0.
            || self.second_min < 0.
            || self.divider_width <= 0.
            || self.keyboard_step <= 0.
        {
            return Err(Error(
                "Invalid split limits or divider/keyboard size".into(),
            ));
        }
        Ok(())
    }
    pub(crate) fn geometry(self, bounds: Rect, ratio: f64, collapsed: SplitPane) -> SplitInfo {
        let h = self.orientation == Orientation::Horizontal;
        let main = if h { bounds.width } else { bounds.height };
        let divider_width = self.divider_width.min(main);
        let available = (main - divider_width).max(0.);
        let (min, max) =
            if self.first_min <= available && self.second_min <= available - self.first_min {
                (self.first_min, available - self.second_min)
            } else {
                let largest = self.first_min.max(self.second_min);
                let a = self.first_min / largest;
                let b = self.second_min / largest;
                let position = available * a / (a + b);
                (position, position)
            };
        let first = match collapsed {
            SplitPane::First => 0.,
            SplitPane::Second => available,
            SplitPane::None => (ratio * available).clamp(min, max),
        };
        let rect = |start, extent| {
            if h {
                Rect::new(bounds.x + start, bounds.y, extent, bounds.height)
            } else {
                Rect::new(bounds.x, bounds.y + start, bounds.width, extent)
            }
        };
        SplitInfo {
            orientation: self.orientation,
            ratio,
            effective_ratio: if available > 0. {
                first / available
            } else {
                0.5
            },
            collapsed,
            available,
            limits: (min, max),
            keyboard_step: self.keyboard_step,
            first: rect(0., first),
            divider: rect(first, divider_width),
            second: rect(first + divider_width, (available - first).max(0.)),
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SplitInfo {
    pub orientation: Orientation,
    pub ratio: f64,
    pub effective_ratio: f64,
    pub collapsed: SplitPane,
    pub available: f64,
    pub limits: (f64, f64),
    pub keyboard_step: f64,
    pub first: Rect,
    pub divider: Rect,
    pub second: Rect,
}
#[derive(Clone)]
pub(crate) struct SplitData {
    pub ratio: Binding<f64>,
    pub collapsed: Option<Binding<SplitPane>>,
    pub options: SplitOptions,
}
/// Store a model for application-controlled collapse and restoration.
#[derive(Clone)]
pub struct Split {
    data: SplitData,
}
impl Split {
    pub fn new(ratio: f64) -> Self {
        assert!(ratio.is_finite(), "Split ratio must be finite");
        Self::bound(State::new(ratio.clamp(0., 1.)).binding())
    }
    pub fn bound(ratio: Binding<f64>) -> Self {
        Self {
            data: SplitData {
                ratio,
                collapsed: Some(State::new(SplitPane::None).binding()),
                options: SplitOptions::default(),
            },
        }
    }
    pub fn options(mut self, options: SplitOptions) -> Self {
        self.data.options = options;
        self
    }
    pub fn orientation(mut self, orientation: Orientation) -> Self {
        self.data.options.orientation = orientation;
        self
    }
    pub fn limits(mut self, first: f64, second: f64) -> Self {
        self.data.options.first_min = first;
        self.data.options.second_min = second;
        self
    }
    pub fn divider_width(mut self, width: f64) -> Self {
        self.data.options.divider_width = width;
        self
    }
    pub fn keyboard_step(mut self, step: f64) -> Self {
        self.data.options.keyboard_step = step;
        self
    }
    pub fn collapsed(mut self, collapsed: Binding<SplitPane>) -> Self {
        self.data.collapsed = Some(collapsed);
        self
    }
    pub fn collapsed_pane(&self) -> SplitPane {
        self.data
            .collapsed
            .as_ref()
            .map_or(SplitPane::None, Binding::get)
    }
    pub fn ratio(&self) -> f64 {
        self.data.ratio.get()
    }
    pub fn set_ratio(&self, ratio: f64) -> bool {
        if !ratio.is_finite() {
            return false;
        }
        let old = self.ratio();
        self.data.ratio.set(ratio.clamp(0., 1.));
        old != self.ratio()
    }
    pub fn collapse(&self, pane: SplitPane) {
        if let Some(collapsed) = &self.data.collapsed {
            collapsed.set(pane);
        }
    }
    pub fn restore(&self) {
        self.collapse(SplitPane::None)
    }
    pub fn view(&self, first: View, second: View) -> View {
        description(self.data.clone(), first, second)
    }
}
fn description(data: SplitData, first: View, second: View) -> View {
    let divider = View::new(Kind::SplitDivider, String::new(), vec![])
        .key("$divider")
        .accessible_label("Split divider");
    let mut view = View::new(
        Kind::Split,
        String::new(),
        vec![
            View::layer(vec![first]).key("$first"),
            divider,
            View::layer(vec![second]).key("$second"),
        ],
    );
    Rc::make_mut(&mut view.0).split = Some(data);
    view
}
/// Existing call sites keep their ratio state; collapse state belongs to mounted identity.
pub fn split_view(ratio: &State<f64>, first: View, second: View) -> View {
    description(
        SplitData {
            ratio: ratio.binding(),
            collapsed: None,
            options: SplitOptions::default(),
        },
        first,
        second,
    )
}
pub fn split(split: &Split, first: View, second: View) -> View {
    split.view(first, second)
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PointerCursor {
    Arrow,
    Text,
    ResizeHorizontal,
    ResizeVertical,
}
pub(crate) struct SplitDrag {
    pub divider: NodeId,
    pub orientation: Orientation,
    pub offset: f64,
    pub ratio: f64,
    pub collapsed: SplitPane,
    pub last_ratio: f64,
}
impl Host {
    fn split_owner(&self, id: NodeId) -> Option<NodeId> {
        let m = self.0.mounted.borrow();
        let n = m.nodes.iter().find(|n| n.id == id)?;
        if n.kind == Kind::Split {
            Some(id)
        } else if n.kind == Kind::SplitDivider {
            n.parent
        } else {
            None
        }
    }
    fn split_data(&self, id: NodeId) -> Option<(NodeId, SplitData)> {
        let owner = self.split_owner(id)?;
        let m = self.0.mounted.borrow();
        m.nodes.iter().find(|n| {
            n.id == owner && n.enabled && !n.hidden && !n.layout_hidden && self.in_scope(owner)
        })?;
        Some((owner, m.descriptions.get(&owner)?.split.clone()?))
    }
    pub fn split_info(&self, id: NodeId) -> Result<Option<SplitInfo>, Error> {
        self.refresh()?;
        let owner = self.split_owner(id);
        Ok(self
            .0
            .mounted
            .borrow()
            .nodes
            .iter()
            .find(|n| Some(n.id) == owner)
            .and_then(|n| n.split))
    }
    pub fn set_split_ratio(&self, id: NodeId, ratio: f64) -> Result<bool, Error> {
        self.refresh()?;
        if !ratio.is_finite() {
            return Ok(false);
        }
        let Some((_, data)) = self.split_data(id) else {
            return Ok(false);
        };
        let old = data.ratio.get();
        data.ratio.set(ratio.clamp(0., 1.));
        let changed = data.ratio.get() != old;
        if changed {
            self.redraw();
        }
        Ok(changed)
    }
    pub fn collapse_split(&self, id: NodeId, pane: SplitPane) -> Result<bool, Error> {
        self.refresh()?;
        let Some((owner, data)) = self.split_data(id) else {
            return Ok(false);
        };
        let old = data.collapsed.as_ref().map_or_else(
            || {
                self.0
                    .splits
                    .borrow()
                    .get(&owner)
                    .copied()
                    .unwrap_or_default()
            },
            Binding::get,
        );
        let accepted = if let Some(binding) = data.collapsed {
            binding.set(pane);
            binding.get()
        } else {
            self.0.splits.borrow_mut().insert(owner, pane);
            pane
        };
        if old != accepted {
            let mut i = self.0.interaction.borrow_mut();
            i.split_drag = None;
            i.pressed = None;
            drop(i);
            self.redraw();
        }
        Ok(old != accepted)
    }
    pub(crate) fn split_offset(&self, id: NodeId, position: f64) -> Result<bool, Error> {
        if !position.is_finite() {
            return Ok(false);
        }
        let Some(info) = self.split_info(id)? else {
            return Ok(false);
        };
        if info.available <= 0. {
            return Ok(false);
        }
        let position = position.clamp(info.limits.0, info.limits.1);
        let current = if info.orientation == Orientation::Horizontal {
            info.first.width
        } else {
            info.first.height
        };
        if position == current && info.collapsed == SplitPane::None {
            return Ok(false);
        }
        let reopened = self.collapse_split(id, SplitPane::None)?;
        Ok(self.set_split_ratio(id, position / info.available)? || reopened)
    }
    pub fn resize_split(&self, id: NodeId, delta: f64) -> Result<bool, Error> {
        let Some(info) = self.split_info(id)? else {
            return Ok(false);
        };
        self.split_offset(
            id,
            (if info.orientation == Orientation::Horizontal {
                info.first.width
            } else {
                info.first.height
            }) + delta,
        )
    }
    pub(crate) fn toggle_split(&self, id: NodeId) -> Result<bool, Error> {
        let Some(info) = self.split_info(id)? else {
            return Ok(false);
        };
        self.collapse_split(
            id,
            if info.collapsed != SplitPane::None {
                SplitPane::None
            } else if info.effective_ratio <= 0.5 {
                SplitPane::First
            } else {
                SplitPane::Second
            },
        )
    }
    pub fn pointer_cursor(&self, point: Point) -> Result<PointerCursor, Error> {
        self.refresh()?;
        if let Some(drag) = &self.0.interaction.borrow().split_drag {
            return Ok(if drag.orientation == Orientation::Horizontal {
                PointerCursor::ResizeHorizontal
            } else {
                PointerCursor::ResizeVertical
            });
        }
        if self
            .0
            .interaction
            .borrow()
            .table_drag
            .as_ref()
            .is_some_and(|d| d.resize)
        {
            return Ok(PointerCursor::ResizeHorizontal);
        }
        let hit = self.hit_test(point)?;
        let m = self.0.mounted.borrow();
        Ok(match m.nodes.iter().find(|n| Some(n.id) == hit) {
            Some(n) if n.table_resize => PointerCursor::ResizeHorizontal,
            Some(n) if n.kind == Kind::TextField => PointerCursor::Text,
            Some(n) if n.kind == Kind::SplitDivider => {
                if n.split
                    .is_some_and(|s| s.orientation == Orientation::Vertical)
                {
                    PointerCursor::ResizeVertical
                } else {
                    PointerCursor::ResizeHorizontal
                }
            }
            _ => PointerCursor::Arrow,
        })
    }
    pub(crate) fn split_pointer(&self, event: crate::Pointer, clicks: u32) -> Result<bool, Error> {
        use crate::PointerKind::*;
        let drag = self
            .0
            .interaction
            .borrow()
            .split_drag
            .as_ref()
            .map(|d| (d.divider, d.orientation, d.offset));
        if let Some((id, orientation, offset)) = drag {
            if matches!(event.kind, Up | Cancel) {
                let mut i = self.0.interaction.borrow_mut();
                i.split_drag = None;
                i.pressed = None;
                drop(i);
                self.redraw();
                return Ok(true);
            }
            if event.kind == Move
                && let Some(info) = self.split_info(id)?
            {
                let start = if orientation == Orientation::Horizontal {
                    info.first.x
                } else {
                    info.first.y
                };
                let p = if orientation == Orientation::Horizontal {
                    event.position.x
                } else {
                    event.position.y
                };
                // Resizing must not end this capture when reopening an already expanded split.
                let saved = self.0.interaction.borrow_mut().split_drag.take();
                self.split_offset(id, p - start - offset)?;
                let accepted = self.split_data(id).map(|(_, data)| data.ratio.get());
                {
                    let mut i = self.0.interaction.borrow_mut();
                    if saved.is_some() {
                        i.pressed = Some(id);
                    }
                    i.split_drag = saved;
                    if let Some(drag) = &mut i.split_drag
                        && let Some(ratio) = accepted
                    {
                        drag.last_ratio = ratio;
                    }
                }
                self.redraw();
            }
            return Ok(true);
        }
        if event.kind != Down || event.button != 0 || !self.0.interaction.borrow().active {
            return Ok(false);
        }
        let Some(id) = self.hit_test(event.position)? else {
            return Ok(false);
        };
        let node = self
            .0
            .mounted
            .borrow()
            .nodes
            .iter()
            .find(|n| n.id == id && n.kind == Kind::SplitDivider)
            .cloned();
        let Some(node) = node else { return Ok(false) };
        let Some(info) = node.split else {
            return Ok(false);
        };
        self.focus(id)?;
        if clicks >= 2 {
            self.toggle_split(id)?;
        } else {
            let offset = if info.orientation == Orientation::Horizontal {
                event.position.x - info.divider.x
            } else {
                event.position.y - info.divider.y
            };
            let mut i = self.0.interaction.borrow_mut();
            i.pointer = event.position;
            i.pressed = Some(id);
            i.split_drag = Some(SplitDrag {
                divider: id,
                orientation: info.orientation,
                offset,
                ratio: info.ratio,
                collapsed: info.collapsed,
                last_ratio: info.ratio,
            });
        }
        self.redraw();
        Ok(true)
    }
    pub(crate) fn split_key(&self, id: NodeId, event: crate::KeyEvent) -> Result<bool, Error> {
        let Some(info) = self.split_info(id)? else {
            return Ok(false);
        };
        use crate::Key::*;
        match event.key {
            Enter | Space if !event.repeat => self.toggle_split(id),
            Escape => {
                let drag = self.0.interaction.borrow_mut().split_drag.take();
                if let Some(drag) = drag {
                    self.0.interaction.borrow_mut().pressed = None;
                    self.collapse_split(id, drag.collapsed)?;
                    self.set_split_ratio(id, drag.ratio)?;
                    return Ok(true);
                }
                self.collapse_split(id, SplitPane::None)
            }
            Home => self.split_offset(id, info.limits.0),
            End => self.split_offset(id, info.limits.1),
            Left | Right if info.orientation == Orientation::Horizontal => self.resize_split(
                id,
                info.keyboard_step
                    * if event.modifiers.shift { 10. } else { 1. }
                    * if event.key == Left { -1. } else { 1. },
            ),
            Up | Down if info.orientation == Orientation::Vertical => self.resize_split(
                id,
                info.keyboard_step
                    * if event.modifiers.shift { 10. } else { 1. }
                    * if event.key == Up { -1. } else { 1. },
            ),
            _ => Ok(false),
        }
    }
    pub(crate) fn reconcile_split_capture(&self) {
        let (focus, drag) = {
            let i = self.0.interaction.borrow();
            (
                i.focus,
                i.split_drag
                    .as_ref()
                    .map(|d| (d.divider, d.orientation, d.collapsed, d.last_ratio)),
            )
        };
        let nodes = self.0.mounted.borrow().nodes.clone();
        let invalid = drag.is_some_and(|(id, orientation, collapsed, last_ratio)| {
            !nodes.iter().any(|n| {
                n.id == id
                    && n.enabled
                    && !n.hidden
                    && !n.layout_hidden
                    && n.split.is_some_and(|s| {
                        (s.collapsed == SplitPane::None || s.collapsed == collapsed)
                            && s.orientation == orientation
                            && s.ratio == last_ratio
                    })
            })
        });
        if invalid {
            let mut i = self.0.interaction.borrow_mut();
            i.split_drag = None;
            i.pressed = None;
        }
        for n in nodes.iter().filter(|n| n.layout_hidden) {
            if let Some(editor) = self.0.editors.borrow_mut().get_mut(&n.id) {
                editor.unmark(false);
            }
        }
        if let Some(mut id) =
            focus.filter(|id| nodes.iter().any(|n| n.id == *id && n.layout_hidden))
        {
            while let Some(parent) = nodes.iter().find(|n| n.id == id).and_then(|n| n.parent) {
                id = parent;
                if nodes.iter().any(|n| {
                    n.id == id
                        && n.kind == Kind::Split
                        && n.split.is_some_and(|s| s.collapsed != SplitPane::None)
                }) {
                    if let Some(divider) = nodes
                        .iter()
                        .find(|n| n.parent == Some(id) && n.kind == Kind::SplitDivider)
                    {
                        let _ = self.focus(divider.id);
                    }
                    break;
                }
            }
        }
    }
}
