//! Scroll geometry and transport in logical points. Scrollbars overlay content.
use crate::{Error, Host, NodeId, Orientation, Point, Rect, Size, View};
use std::rc::Rc;
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ScrollAxes {
    #[default]
    Vertical,
    Horizontal,
    Both,
}
impl ScrollAxes {
    pub(crate) fn x(self) -> bool {
        self != Self::Vertical
    }
    pub(crate) fn y(self) -> bool {
        self != Self::Horizontal
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Scrollbar {
    pub orientation: Orientation,
    pub track: Rect,
    pub thumb: Rect,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScrollInfo {
    pub axes: ScrollAxes,
    pub viewport: Rect,
    pub content: Size,
    pub offset: Point,
    pub max: Point,
    pub scrollbars: bool,
}
impl ScrollInfo {
    pub(crate) fn clamp(self, p: Point) -> Point {
        Point {
            x: p.x.clamp(0., self.max.x),
            y: p.y.clamp(0., self.max.y),
        }
    }
    pub fn bars(self) -> impl Iterator<Item = Scrollbar> {
        let mut bars = [None, None];
        if self.scrollbars {
            for (slot, horizontal, max, extent, offset) in [
                (0, true, self.max.x, self.content.width, self.offset.x),
                (1, false, self.max.y, self.content.height, self.offset.y),
            ] {
                if max <= 0. {
                    continue;
                }
                let v = self.viewport;
                let track = if horizontal {
                    Rect::new(
                        v.x + 2.,
                        v.y + (v.height - 10.).max(0.),
                        (v.width - 4. - if self.max.y > 0. { 10. } else { 0. }).max(0.),
                        8_f64.min(v.height),
                    )
                } else {
                    Rect::new(
                        v.x + (v.width - 10.).max(0.),
                        v.y + 2.,
                        8_f64.min(v.width),
                        (v.height - 4. - if self.max.x > 0. { 10. } else { 0. }).max(0.),
                    )
                };
                let length = if horizontal {
                    track.width
                } else {
                    track.height
                };
                let visible = if horizontal { v.width } else { v.height };
                let thumb_len = (length * visible / extent).max(24.).min(length);
                let start = (length - thumb_len) * offset / max;
                let thumb = if horizontal {
                    Rect::new(track.x + start, track.y, thumb_len, track.height)
                } else {
                    Rect::new(track.x, track.y + start, track.width, thumb_len)
                };
                bars[slot] = Some(Scrollbar {
                    orientation: if horizontal {
                        Orientation::Horizontal
                    } else {
                        Orientation::Vertical
                    },
                    track,
                    thumb,
                });
            }
        }
        bars.into_iter().flatten()
    }
}
impl View {
    pub fn scroll_axes(mut self, axes: ScrollAxes) -> Self {
        Rc::make_mut(&mut self.0).scroll_axes = axes;
        self
    }
    pub fn scrollbars(mut self, visible: bool) -> Self {
        Rc::make_mut(&mut self.0).scrollbars = visible;
        self
    }
}
pub(crate) struct ScrollDrag {
    pub id: NodeId,
    pub orientation: Orientation,
    pub pointer: f64,
    pub offset: f64,
}
impl Host {
    /// Current offset plus geometry from the latest layout.
    pub fn scroll_info(&self, id: NodeId) -> Result<Option<ScrollInfo>, Error> {
        self.refresh()?;
        Ok(self
            .0
            .mounted
            .borrow()
            .nodes
            .iter()
            .find(|n| n.id == id)
            .and_then(|n| n.scroll)
            .map(|mut info| {
                info.offset = self.0.scroll.borrow().get(&id).copied().unwrap_or_default();
                info
            }))
    }
    /// Absolute offset; disabled axes and content edges clamp automatically.
    pub fn scroll_to(&self, id: NodeId, offset: Point) -> Result<bool, Error> {
        if !offset.x.is_finite() || !offset.y.is_finite() {
            return Ok(false);
        }
        let Some(info) = self.scroll_info(id)? else {
            return Ok(false);
        };
        let next = info.clamp(offset);
        if next == info.offset {
            return Ok(false);
        }
        self.0.scroll.borrow_mut().insert(id, next);
        if let Some(model) = self
            .0
            .mounted
            .borrow()
            .nodes
            .iter()
            .find(|n| n.id == id)
            .and_then(|n| n.collection.clone())
        {
            model.set_viewport(next.y, info.viewport.height);
        }
        self.redraw();
        Ok(true)
    }
    pub fn scroll_by(&self, id: NodeId, delta: Point) -> Result<bool, Error> {
        let Some(info) = self.scroll_info(id)? else {
            return Ok(false);
        };
        self.scroll_to(
            id,
            Point {
                x: info.offset.x + delta.x,
                y: info.offset.y + delta.y,
            },
        )
    }
    /// Vertical shorthand. Positive deltas move toward the end of the content.
    pub fn scroll(&self, point: Point, delta: f64) -> Result<bool, Error> {
        self.scroll_at(point, Point { x: 0., y: delta })
    }
    /// Route each axis through the deepest scroll container and its ancestors.
    /// Any unused delta at an edge continues outward, preserving diagonal motion.
    pub fn scroll_at(&self, point: Point, mut delta: Point) -> Result<bool, Error> {
        if !delta.x.is_finite() || !delta.y.is_finite() {
            return Ok(false);
        }
        let nodes = self.scene()?;
        let mut current = nodes
            .iter()
            .rev()
            .find(|n| n.enabled && !n.hidden && n.clip.contains(point) && self.in_scope(n.id))
            .map(|n| n.id);
        let mut changed = false;
        while let Some(id) = current {
            let Some(node) = nodes.iter().find(|n| n.id == id) else {
                break;
            };
            if node.enabled
                && !node.hidden
                && let Some(info) = self.scroll_info(id)?
            {
                let next = info.clamp(Point {
                    x: info.offset.x + delta.x,
                    y: info.offset.y + delta.y,
                });
                delta.x -= next.x - info.offset.x;
                delta.y -= next.y - info.offset.y;
                changed |= self.scroll_to(id, next)?;
            }
            current = node.parent;
        }
        Ok(changed)
    }
    /// Reveal a window-space rectangle through every containing scroll viewport.
    pub fn reveal_rect(&self, id: NodeId, mut rect: Rect) -> bool {
        if ![rect.x, rect.y, rect.width, rect.height]
            .into_iter()
            .all(f64::is_finite)
            || rect.width < 0.
            || rect.height < 0.
        {
            return false;
        }
        fn movement(start: f64, size: f64, edge: f64, visible: f64) -> f64 {
            if size > visible || start < edge {
                start - edge
            } else {
                (start + size - edge - visible).max(0.)
            }
        }
        let m = self.0.mounted.borrow();
        let mut current = m.nodes.iter().find(|n| n.id == id).and_then(|n| n.parent);
        let mut changed = false;
        while let Some(id) = current {
            let Some(n) = m.nodes.iter().find(|n| n.id == id) else {
                break;
            };
            if let Some(info) = n.scroll {
                let current = self.0.scroll.borrow().get(&id).copied().unwrap_or_default();
                // Geometry belongs to the last layout; account for pending offsets first.
                rect.x -= current.x - info.offset.x;
                rect.y -= current.y - info.offset.y;
                let next = info.clamp(Point {
                    x: current.x
                        + movement(rect.x, rect.width, info.viewport.x, info.viewport.width),
                    y: current.y
                        + movement(rect.y, rect.height, info.viewport.y, info.viewport.height),
                });
                rect.x -= next.x - current.x;
                rect.y -= next.y - current.y;
                changed |= next != current;
                self.0.scroll.borrow_mut().insert(id, next);
            }
            current = n.parent;
        }
        drop(m);
        if changed {
            self.redraw();
        }
        changed
    }
    pub(crate) fn scroll_key(
        &self,
        id: Option<NodeId>,
        event: crate::KeyEvent,
    ) -> Result<bool, Error> {
        use crate::Key::*;
        let nodes = self.snapshot()?;
        let mut current = id.or_else(|| {
            nodes
                .iter()
                .find(|n| n.scroll.is_some() && n.enabled && !n.hidden && self.in_scope(n.id))
                .map(|n| n.id)
        });
        while let Some(id) = current {
            let Some(n) = nodes.iter().find(|n| n.id == id) else {
                break;
            };
            if n.enabled
                && !n.hidden
                && let Some(info) = self.scroll_info(id)?
            {
                let horizontal = event.modifiers.shift || info.axes == ScrollAxes::Horizontal;
                let amount = if horizontal {
                    info.viewport.width
                } else {
                    info.viewport.height
                } * 0.9;
                let mut next = info.offset;
                match event.key {
                    Left => next.x -= 40.,
                    Right => next.x += 40.,
                    Up => next.y -= 40.,
                    Down => next.y += 40.,
                    PageUp | PageDown => {
                        let delta = if event.key == PageUp { -amount } else { amount };
                        if horizontal {
                            next.x += delta
                        } else {
                            next.y += delta
                        }
                    }
                    Home | End => {
                        let end = event.key == End;
                        if horizontal {
                            next.x = if end { info.max.x } else { 0. }
                        } else {
                            next.y = if end { info.max.y } else { 0. }
                        }
                    }
                    _ => return Ok(false),
                }
                if self.scroll_to(id, next)? {
                    return Ok(true);
                }
            }
            current = n.parent;
        }
        Ok(false)
    }
    fn drag_target(&self) -> Option<(NodeId, crate::Selection)> {
        let i = self.0.interaction.borrow();
        if !i.active {
            return None;
        }
        let id = i.pressed?;
        let m = self.0.mounted.borrow();
        let n = m.nodes.iter().find(|n| {
            n.id == id
                && n.kind == crate::Kind::TextField
                && n.enabled
                && !n.hidden
                && !n.layout_hidden
        })?;
        if n.clip.inset(4.).contains(i.pointer) {
            return None;
        }
        let point = i.pointer;
        drop(m);
        drop(i);
        let current = self.0.editors.borrow().get(&id)?.selection;
        let selection = self.drag_selection(id, point)?;
        (selection != current).then_some((id, selection))
    }
    pub(crate) fn update_drag_deadline(&self, now: std::time::Instant) {
        let scrolling = self.drag_target().is_some();
        let mut i = self.0.interaction.borrow_mut();
        if scrolling {
            i.drag_tick
                .get_or_insert(now + std::time::Duration::from_millis(16));
        } else {
            i.drag_tick = None;
        }
    }
    pub(crate) fn drag_delay(&self) -> Option<std::time::Duration> {
        self.0
            .interaction
            .borrow()
            .drag_tick
            .map(|at| at.saturating_duration_since(std::time::Instant::now()))
    }
    pub(crate) fn run_drag_due(&self, now: std::time::Instant) {
        if self.refresh().is_err() {
            self.0.interaction.borrow_mut().drag_tick = None;
            return;
        }
        let due = self
            .0
            .interaction
            .borrow()
            .drag_tick
            .is_some_and(|at| at <= now);
        if !due {
            return;
        }
        self.0.interaction.borrow_mut().drag_tick = None;
        if let Some((id, selection)) = self.drag_target() {
            let point = self.0.interaction.borrow().pointer;
            let _ = self.select_pointer_text(id, point, selection);
        }
    }
    pub(crate) fn reconcile_capture(&self) {
        let m = self.0.mounted.borrow();
        let valid = |id| {
            m.nodes.iter().any(|n| {
                n.id == id && n.enabled && !n.hidden && !n.layout_hidden && self.in_scope(id)
            })
        };
        let mut i = self.0.interaction.borrow_mut();
        if !i.active || i.scroll_drag.as_ref().is_some_and(|d| !valid(d.id)) {
            i.scroll_drag = None;
        }
        if !i.active || i.pressed.is_some_and(|id| !valid(id)) {
            i.pressed = None;
        }
        if i.pressed.is_none() {
            i.drag_tick = None;
        }
        if i.hovered.is_some_and(|id| !valid(id)) {
            i.hovered = None;
        }
        if i.focus.is_some_and(|id| {
            !m.nodes
                .iter()
                .any(|n| n.id == id && n.enabled && !n.hidden && !n.layout_hidden)
        }) {
            i.focus = None;
        }
    }
    pub(crate) fn scrollbar_pointer(&self, event: crate::Pointer) -> Result<bool, Error> {
        use crate::PointerKind::*;
        if event.kind == Cancel {
            self.0.interaction.borrow_mut().scroll_drag = None;
            return Ok(false);
        }
        let drag = self
            .0
            .interaction
            .borrow()
            .scroll_drag
            .as_ref()
            .map(|d| (d.id, d.orientation, d.pointer, d.offset));
        if let Some(drag) = drag {
            if event.kind == Up {
                self.0.interaction.borrow_mut().scroll_drag = None;
                return Ok(true);
            }
            if event.kind == Move {
                let Some(info) = self.scroll_info(drag.0)? else {
                    return Ok(false);
                };
                if let Some(bar) = info.bars().find(|b| b.orientation == drag.1) {
                    let h = drag.1 == Orientation::Horizontal;
                    let (p, travel, max) = if h {
                        (
                            event.position.x,
                            bar.track.width - bar.thumb.width,
                            info.max.x,
                        )
                    } else {
                        (
                            event.position.y,
                            bar.track.height - bar.thumb.height,
                            info.max.y,
                        )
                    };
                    let value = drag.3 + (p - drag.2) * max / travel.max(1.);
                    self.scroll_to(
                        drag.0,
                        if h {
                            Point {
                                x: value,
                                y: info.offset.y,
                            }
                        } else {
                            Point {
                                x: info.offset.x,
                                y: value,
                            }
                        },
                    )?;
                }
            }
            return Ok(true);
        }
        if event.kind != Down || event.button != 0 {
            return Ok(false);
        }
        // First locate the topmost containing scroll subtree, so a covered bar is inert.
        let nodes = self.scene()?;
        let mut current = nodes
            .iter()
            .rev()
            .find(|n| n.enabled && !n.hidden && n.clip.contains(event.position))
            .map(|n| n.id);
        while let Some(id) = current {
            let Some(n) = nodes.iter().find(|n| n.id == id) else {
                break;
            };
            if self.in_scope(id)
                && let Some(info) = self.scroll_info(id)?
            {
                for bar in info.bars() {
                    if bar.track.contains(event.position) && n.clip.contains(event.position) {
                        let h = bar.orientation == Orientation::Horizontal;
                        let p = if h {
                            event.position.x
                        } else {
                            event.position.y
                        };
                        let offset = if h { info.offset.x } else { info.offset.y };
                        if bar.thumb.contains(event.position) {
                            self.0.interaction.borrow_mut().scroll_drag = Some(ScrollDrag {
                                id,
                                orientation: bar.orientation,
                                pointer: p,
                                offset,
                            });
                        } else {
                            let start = if h { bar.thumb.x } else { bar.thumb.y };
                            let page = if h {
                                info.viewport.width
                            } else {
                                info.viewport.height
                            } * 0.9;
                            self.scroll_by(
                                id,
                                if h {
                                    Point {
                                        x: if p < start { -page } else { page },
                                        y: 0.,
                                    }
                                } else {
                                    Point {
                                        x: 0.,
                                        y: if p < start { -page } else { page },
                                    }
                                },
                            )?;
                        }
                        return Ok(true);
                    }
                }
            }
            current = n.parent;
        }
        Ok(false)
    }
}
