use crate::{DismissReason, Error, Host, NodeId, OverlayPlacement, Rect, Size, TextStyle, View};
use std::{
    rc::Rc,
    time::{Duration, Instant},
};
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TooltipOptions {
    pub delay: Duration,
    pub max_width: f64,
    pub gap: f64,
    pub margin: f64,
    pub placement: OverlayPlacement,
}
impl Default for TooltipOptions {
    fn default() -> Self {
        Self {
            delay: Duration::from_millis(600),
            max_width: 280.,
            gap: 8.,
            margin: 8.,
            placement: OverlayPlacement::Below,
        }
    }
}
#[derive(Clone)]
pub(crate) struct TooltipData {
    pub text: String,
    pub options: TooltipOptions,
    pub close: Option<crate::overlay::DismissCallback>,
}
pub(crate) struct TooltipState {
    pub owner: Option<NodeId>,
    pub since: Instant,
    pub visible: bool,
    pub suppressed: bool,
    pub data: Option<TooltipData>,
}
impl Default for TooltipState {
    fn default() -> Self {
        Self {
            owner: None,
            since: Instant::now(),
            visible: false,
            suppressed: false,
            data: None,
        }
    }
}
#[derive(Clone, Debug)]
pub struct TooltipPresentation {
    pub owner: NodeId,
    pub text: String,
    pub bounds: Rect,
    pub text_bounds: Rect,
    pub style: TextStyle,
}
impl View {
    pub fn tooltip_options(mut self, options: TooltipOptions) -> Self {
        Rc::make_mut(&mut self.0).tooltip_options = options;
        self
    }
    pub fn on_tooltip_dismiss(mut self, callback: impl Fn(DismissReason) + 'static) -> Self {
        Rc::make_mut(&mut self.0).tooltip_close = Some(Rc::new(callback));
        self
    }
}
impl Host {
    pub(crate) fn dismiss_tooltip(&self, reason: DismissReason, suppress: bool) -> bool {
        let callback = {
            let mut t = self.0.tooltip.borrow_mut();
            let visible = std::mem::replace(&mut t.visible, false);
            t.suppressed = suppress;
            if visible {
                t.data.as_ref().and_then(|d| d.close.clone())
            } else {
                None
            }
        };
        if let Some(close) = callback {
            close(reason);
        }
        self.redraw();
        true
    }
    /// Advance presentation time without installing an animation or polling loop.
    pub fn advance_overlays_at(&self, now: Instant) {
        let _ = self.refresh();
        let (hover, focus, active) = {
            let i = self.0.interaction.borrow();
            (i.tooltip_hover, i.focus, i.active)
        };
        let candidate = if active {
            [hover, focus].into_iter().flatten().find_map(|id| {
                let m = self.0.mounted.borrow();
                let n = m.nodes.iter().find(|n| {
                    n.id == id
                        && n.enabled
                        && !n.hidden
                        && !n.layout_hidden
                        && n.clip.width > 0.
                        && n.clip.height > 0.
                        && self.in_scope(id)
                })?;
                let d = m.descriptions.get(&n.id)?;
                (!d.tooltip.is_empty()).then(|| {
                    (
                        id,
                        TooltipData {
                            text: d.tooltip.clone(),
                            options: d.tooltip_options,
                            close: d.tooltip_close.clone(),
                        },
                    )
                })
            })
        } else {
            None
        };
        let old = self.0.tooltip.borrow().owner;
        let id = candidate.as_ref().map(|c| c.0);
        if old != id {
            let old_valid = old.is_some_and(|id| {
                self.0.mounted.borrow().nodes.iter().any(|n| {
                    n.id == id
                        && n.enabled
                        && !n.hidden
                        && !n.layout_hidden
                        && n.clip.width > 0.
                        && n.clip.height > 0.
                        && self.in_scope(id)
                })
            });
            self.dismiss_tooltip(
                if !active {
                    DismissReason::Deactivated
                } else if old_valid {
                    DismissReason::Outside
                } else {
                    DismissReason::OwnerRemoved
                },
                false,
            );
            let old = {
                let mut t = self.0.tooltip.borrow_mut();
                t.owner = id;
                t.since = now;
                std::mem::replace(&mut t.data, candidate.map(|c| c.1))
            };
            drop(old);
        } else if let Some((_, data)) = candidate {
            let old = self.0.tooltip.borrow_mut().data.replace(data);
            drop(old);
        }
        let show = {
            let mut t = self.0.tooltip.borrow_mut();
            let show = !t.visible
                && !t.suppressed
                && t.data
                    .as_ref()
                    .is_some_and(|d| now.saturating_duration_since(t.since) >= d.options.delay);
            if show {
                t.visible = true;
            }
            show
        };
        if show {
            self.redraw();
        }
    }
    pub(crate) fn tooltip_delay(&self) -> Option<Duration> {
        self.advance_overlays_at(Instant::now());
        let t = self.0.tooltip.borrow();
        (!t.visible && !t.suppressed)
            .then(|| {
                t.data
                    .as_ref()
                    .map(|d| d.options.delay.saturating_sub(t.since.elapsed()))
            })
            .flatten()
    }
    /// Compatibility query; the point is now the anchored presentation origin.
    pub fn tooltip(&self) -> Option<(String, crate::Point)> {
        self.advance_overlays_at(Instant::now());
        let t = self.0.tooltip.borrow();
        if !t.visible {
            return None;
        }
        let id = t.owner?;
        let text = t.data.as_ref()?.text.clone();
        drop(t);
        let n = self
            .0
            .mounted
            .borrow()
            .nodes
            .iter()
            .find(|n| n.id == id)?
            .bounds;
        Some((
            text,
            crate::Point {
                x: n.x,
                y: n.y + n.height,
            },
        ))
    }
    pub fn tooltip_presentation(
        &self,
        measure: &dyn crate::Measure,
    ) -> Result<Option<TooltipPresentation>, Error> {
        self.advance_overlays_at(Instant::now());
        let (id, data) = {
            let t = self.0.tooltip.borrow();
            if !t.visible {
                return Ok(None);
            }
            let Some(id) = t.owner else { return Ok(None) };
            let Some(data) = t.data.clone() else {
                return Ok(None);
            };
            (id, data)
        };
        let Some(anchor) = self
            .0
            .mounted
            .borrow()
            .nodes
            .iter()
            .find(|n| n.id == id)
            .map(|n| n.bounds)
        else {
            return Ok(None);
        };
        let v = self.0.viewport.get();
        let margin = data.options.margin.min(v.width / 2.).min(v.height / 2.);
        let width = (data.options.max_width.min(v.width - 2. * margin) - 16.).max(0.);
        let style = TextStyle {
            size: 13.,
            color: self.theme().text,
            wrap: true,
            ..TextStyle::default()
        };
        let layout = measure.text(&data.text, style, width);
        let bounds = crate::overlay_bounds(
            anchor,
            Size::new(
                (layout.size.width + 16.).min(data.options.max_width),
                layout.size.height + 12.,
            ),
            Rect::new(0., 0., v.width, v.height),
            data.options.placement,
            data.options.gap,
            data.options.margin,
        );
        Ok(Some(TooltipPresentation {
            owner: id,
            text: data.text,
            bounds,
            text_bounds: Rect::new(
                bounds.x + 8.,
                bounds.y + 6.,
                (bounds.width - 16.).max(0.),
                (bounds.height - 12.).max(0.),
            ),
            style,
        }))
    }
}
