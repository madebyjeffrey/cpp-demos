//! Viewport overlays, focus scopes and presentation lifetimes.
use crate::{Error, Host, Kind, Node, NodeId, Point, Rect, Size, View};
use std::{cell::Cell, rc::Rc};
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum OverlayPlacement {
    #[default]
    Below,
    Above,
    Left,
    Right,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum OverlayFocus {
    #[default]
    Content,
    Preserve,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OverlayOptions {
    pub placement: OverlayPlacement,
    pub focus: OverlayFocus,
    /// None keeps the body's intrinsic width.
    pub width: Option<f64>,
    pub max_height: f64,
    pub gap: f64,
    pub margin: f64,
    pub dismiss_on_outside: bool,
    pub dismiss_on_escape: bool,
    pub dismiss_on_deactivate: bool,
}
impl Default for OverlayOptions {
    fn default() -> Self {
        Self {
            placement: OverlayPlacement::Below,
            focus: OverlayFocus::Content,
            width: None,
            max_height: 480.,
            gap: 6.,
            margin: 0.,
            dismiss_on_outside: true,
            dismiss_on_escape: true,
            dismiss_on_deactivate: true,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DismissReason {
    Programmatic,
    Action,
    Escape,
    Outside,
    OwnerRemoved,
    Deactivated,
    Unmounted,
}
pub(crate) type DismissCallback = Rc<dyn Fn(DismissReason)>;
#[derive(Clone)]
pub(crate) struct OverlayData {
    pub options: OverlayOptions,
    pub point: Option<Point>,
    pub close: DismissCallback,
    pub presented: Option<Rc<dyn Fn() -> bool>>,
}
impl OverlayData {
    pub fn new(close: impl Fn(DismissReason) + 'static) -> Self {
        Self {
            options: Default::default(),
            point: None,
            close: Rc::new(close),
            presented: None,
        }
    }
}
pub(crate) struct Scope {
    pub id: NodeId,
    pub previous: Option<NodeId>,
    pub data: OverlayData,
    pub closed: Cell<bool>,
}
/// Flip to the opposite side when it has more space, then clamp inside the
/// viewport margin. Used by popovers, context menus and tooltips in logical points.
pub fn overlay_bounds(
    anchor: Rect,
    size: Size,
    viewport: Rect,
    placement: OverlayPlacement,
    gap: f64,
    margin: f64,
) -> Rect {
    let margin = margin
        .max(0.)
        .min(viewport.width / 2.)
        .min(viewport.height / 2.);
    let v = viewport.inset(margin);
    let w = size.width.max(0.).min(v.width);
    let h = size.height.max(0.).min(v.height);
    let below = v.y + v.height - (anchor.y + anchor.height + gap);
    let above = anchor.y - gap - v.y;
    let right = v.x + v.width - (anchor.x + anchor.width + gap);
    let left = anchor.x - gap - v.x;
    let placement = match placement {
        OverlayPlacement::Below if h > below && above > below => OverlayPlacement::Above,
        OverlayPlacement::Above if h > above && below > above => OverlayPlacement::Below,
        OverlayPlacement::Left if w > left && right > left => OverlayPlacement::Right,
        OverlayPlacement::Right if w > right && left > right => OverlayPlacement::Left,
        p => p,
    };
    let (x, y) = match placement {
        OverlayPlacement::Below => (anchor.x, anchor.y + anchor.height + gap),
        OverlayPlacement::Above => (anchor.x, anchor.y - h - gap),
        OverlayPlacement::Left => (anchor.x - w - gap, anchor.y),
        OverlayPlacement::Right => (anchor.x + anchor.width + gap, anchor.y),
    };
    Rect::new(
        x.clamp(v.x, (v.x + v.width - w).max(v.x)),
        y.clamp(v.y, (v.y + v.height - h).max(v.y)),
        w,
        h,
    )
}
pub(crate) fn validate(options: OverlayOptions) -> Result<(), Error> {
    if [options.max_height, options.gap, options.margin]
        .iter()
        .any(|n| !n.is_finite() || *n < 0.)
        || options.width.is_some_and(|w| !w.is_finite() || w < 0.)
    {
        return Err(Error(
            "Overlay sizes, gap and margin must be finite and nonnegative".into(),
        ));
    }
    Ok(())
}
impl View {
    pub fn overlay_options(mut self, options: OverlayOptions) -> Self {
        if let Some(data) = Rc::make_mut(&mut self.0).overlay.as_mut() {
            data.options = options;
        }
        self
    }
    /// Attach a menu to any owner, including a container or noninteractive label.
    /// The description owns its overlay; the application only retains the model.
    pub fn context_menu(mut self, menu: &crate::Menu, items: Vec<crate::MenuAction>) -> Self {
        let key = self
            .0
            .key
            .clone()
            .unwrap_or_else(|| format!("$context-{}", menu.identity()));
        let mut overlay = menu.view(&key, items);
        Rc::make_mut(&mut overlay.0).overlay_parent = true;
        let d = Rc::make_mut(&mut self.0);
        d.context_menu = Some(menu.clone());
        d.children.push(overlay);
        self
    }
}
impl Host {
    pub(crate) fn scope_nodes(&self) -> Vec<Node> {
        let mut nodes: Vec<_> = self
            .0
            .mounted
            .borrow()
            .nodes
            .iter()
            .filter(|n| {
                n.kind == Kind::Modal
                    && !n.hidden
                    && !n.layout_hidden
                    && !self.0.closed_overlays.borrow().contains(&n.id)
            })
            .cloned()
            .collect();
        nodes.sort_by_key(|n| n.style.z_index);
        nodes
    }
    pub(crate) fn within(nodes: &[Node], mut id: NodeId, scope: NodeId) -> bool {
        for _ in 0..nodes.len() {
            if id == scope {
                return true;
            }
            let Some(n) = nodes.iter().find(|n| n.id == id) else {
                return false;
            };
            // Sibling portals belong to the scope of their anchor.
            if n.kind == Kind::Modal
                && n.overlay_owner.is_none()
                && let Some(key) = &n.anchor
                && let Some(owner) = nodes.iter().find(|n| n.key.as_ref() == Some(key))
            {
                id = owner.id;
                continue;
            }
            let Some(parent) = n.parent else { return false };
            id = parent;
        }
        false
    }
    pub(crate) fn dismiss_overlay(&self, id: NodeId, reason: DismissReason) -> Result<bool, Error> {
        let nodes = self.0.mounted.borrow().nodes.clone();
        let mut callbacks = {
            let scopes = self.0.scopes.borrow();
            scopes
                .iter()
                .rev()
                .filter(|s| s.id == id || Self::within(&nodes, s.id, id))
                .filter(|s| !s.closed.replace(true))
                .map(|s| {
                    self.0.closed_overlays.borrow_mut().insert(s.id);
                    (
                        s.data.close.clone(),
                        if s.id == id {
                            reason
                        } else {
                            DismissReason::OwnerRemoved
                        },
                    )
                })
                .collect::<Vec<_>>()
        };
        if !self.0.scopes.borrow().iter().any(|s| s.id == id)
            && !self.0.closed_overlays.borrow().contains(&id)
        {
            let data = self
                .0
                .mounted
                .borrow()
                .descriptions
                .get(&id)
                .and_then(|d| d.overlay.clone());
            if let Some(data) = data {
                self.0.closed_overlays.borrow_mut().insert(id);
                callbacks.push((data.close, reason));
            }
        }
        let handled = !callbacks.is_empty();
        for (close, reason) in callbacks {
            close(reason);
        }
        self.redraw();
        Ok(handled)
    }
    pub fn dismiss_overlays(&self, reason: DismissReason) {
        let ids: Vec<_> = self.0.scopes.borrow().iter().rev().map(|s| s.id).collect();
        for id in ids {
            let _ = self.dismiss_overlay(id, reason);
        }
        self.dismiss_tooltip(reason, true);
    }
    pub(crate) fn reconcile_overlays(&self) -> Result<(), Error> {
        let nodes = self.0.mounted.borrow().nodes.clone();
        self.0
            .closed_overlays
            .borrow_mut()
            .retain(|id| nodes.iter().any(|n| n.id == *id));
        let current = self.scope_nodes();
        let focus = self.focused();
        let mut removed = vec![];
        let mut restore = vec![];
        {
            let mut scopes = self.0.scopes.borrow_mut();
            let mut i = 0;
            while i < scopes.len() {
                if !current.iter().any(|n| n.id == scopes[i].id) {
                    let scope = scopes.remove(i);
                    restore.push(scope.previous);
                    removed.push(scope);
                } else {
                    i += 1;
                }
            }
        }
        // Closing the parent also closes anchored descendants, even if they
        // were described as sibling portals.
        let mut pending = vec![];
        for n in &current {
            let d = self.0.mounted.borrow().descriptions[&n.id].clone();
            let data = d
                .overlay
                .clone()
                .unwrap_or_else(|| OverlayData::new(|_| {}));
            let owner = n
                .overlay_owner
                .and_then(|id| nodes.iter().find(|n| n.id == id))
                .or_else(|| {
                    n.anchor
                        .as_ref()
                        .and_then(|key| nodes.iter().find(|o| o.key.as_ref() == Some(key)))
                });
            let valid = n.anchor.is_none()
                || owner.is_some_and(|o| {
                    o.enabled
                        && !o.hidden
                        && !o.layout_hidden
                        && o.clip.width > 0.
                        && o.clip.height > 0.
                        && !self
                            .0
                            .closed_overlays
                            .borrow()
                            .iter()
                            .any(|closed| Self::within(&nodes, o.id, *closed))
                });
            let owner_lost = !valid
                || owner.is_some_and(|o| removed.iter().any(|s| Self::within(&nodes, o.id, s.id)));
            let is_new = !self.0.scopes.borrow().iter().any(|s| s.id == n.id);
            if is_new {
                let parent = current.iter().take_while(|p| p.id != n.id).last();
                let previous = focus
                    .filter(|id| {
                        nodes
                            .iter()
                            .any(|n| n.id == *id && n.enabled && !n.hidden && !n.layout_hidden)
                            && parent.is_none_or(|p| Self::within(&nodes, *id, p.id))
                    })
                    .or_else(|| owner.map(|n| n.id))
                    .or_else(|| restore.iter().rev().copied().flatten().next());
                self.0.scopes.borrow_mut().push(Scope {
                    id: n.id,
                    previous,
                    data: data.clone(),
                    closed: Cell::new(false),
                });
            } else {
                let old = {
                    let mut scopes = self.0.scopes.borrow_mut();
                    scopes
                        .iter_mut()
                        .find(|s| s.id == n.id)
                        .map(|s| std::mem::replace(&mut s.data, data))
                };
                drop(old);
            }
            if owner_lost {
                pending.push((n.id, DismissReason::OwnerRemoved));
            } else if !self.0.interaction.borrow().active
                && d.overlay
                    .as_ref()
                    .is_some_and(|d| d.options.dismiss_on_deactivate)
            {
                pending.push((n.id, DismissReason::Deactivated));
            }
        }
        for scope in removed.iter().rev() {
            if !scope.closed.replace(true) {
                (scope.data.close)(
                    if scope
                        .data
                        .presented
                        .as_ref()
                        .is_some_and(|presented| presented())
                    {
                        DismissReason::OwnerRemoved
                    } else {
                        DismissReason::Programmatic
                    },
                );
            }
        }
        for (id, reason) in pending {
            self.dismiss_overlay(id, reason)?;
        }
        let active = self.active_modal();
        let previous = std::mem::replace(&mut self.0.interaction.borrow_mut().modal, active);
        let changed = previous != active;
        if self.0.interaction.borrow().active {
            let restoring = !restore.is_empty();
            for id in restore.into_iter().rev().flatten() {
                let visible = self
                    .0
                    .mounted
                    .borrow()
                    .nodes
                    .iter()
                    .any(|n| n.id == id && n.clip.width > 0. && n.clip.height > 0.);
                if visible && self.focus(id)? {
                    return Ok(());
                }
            }
            if restoring && active.is_none() {
                let target = self.snapshot()?.into_iter().find(|n| {
                    n.enabled
                        && !n.hidden
                        && n.clip.width > 0.
                        && n.clip.height > 0.
                        && crate::host::focusable(n.kind)
                });
                if let Some(n) = target {
                    self.focus(n.id)?;
                }
            }
            if changed && let Some(id) = active {
                let preserve = self
                    .0
                    .mounted
                    .borrow()
                    .descriptions
                    .get(&id)
                    .and_then(|d| d.overlay.as_ref())
                    .is_some_and(|d| d.options.focus == OverlayFocus::Preserve);
                if !preserve {
                    self.focus_next(false)?;
                }
            }
        }
        Ok(())
    }
    pub fn show_context_menu(&self, id: NodeId, point: Option<Point>) -> Result<bool, Error> {
        self.refresh()?;
        let target = {
            let m = self.0.mounted.borrow();
            let mut current = Some(id);
            let mut result = None;
            while let Some(id) = current {
                let Some(n) = m.nodes.iter().find(|n| n.id == id) else {
                    break;
                };
                if !n.enabled || n.hidden || n.layout_hidden || !self.in_scope(id) {
                    break;
                }
                if let Some(menu) = m.descriptions[&id].context_menu.clone() {
                    result = Some(menu);
                    break;
                }
                current = n.parent;
            }
            result
        };
        if let Some(menu) = target {
            if let Some(point) = point {
                menu.show_at(point);
            } else {
                menu.show();
            }
            self.0.interaction.borrow_mut().swallow_up = true;
            self.redraw();
            Ok(true)
        } else {
            Ok(false)
        }
    }
    pub(crate) fn context_pointer(&self, event: crate::Pointer) -> Result<bool, Error> {
        if event.kind != crate::PointerKind::Down
            || !(event.button == 1 || (event.button == 0 && event.modifiers.control))
        {
            return Ok(false);
        }
        let target = self
            .scene()?
            .into_iter()
            .rev()
            .find(|n| {
                n.kind != Kind::Modal
                    && n.enabled
                    && !n.hidden
                    && n.bounds.contains(event.position)
                    && n.clip.contains(event.position)
                    && self.in_scope(n.id)
            })
            .map(|n| n.id);
        if let Some(id) = target {
            return self.show_context_menu(id, Some(event.position));
        }
        Ok(false)
    }
    pub(crate) fn overlay_pointer(&self, event: crate::Pointer) -> Result<bool, Error> {
        if event.kind == crate::PointerKind::Up && self.0.interaction.borrow_mut().swallow_up {
            self.0.interaction.borrow_mut().swallow_up = false;
            return Ok(true);
        }
        if event.kind != crate::PointerKind::Down {
            return Ok(false);
        }
        self.dismiss_tooltip(DismissReason::Outside, true);
        let Some(id) = self.active_modal() else {
            return Ok(false);
        };
        let (inside, dismiss) = {
            let m = self.0.mounted.borrow();
            (
                m.nodes.iter().any(|n| {
                    n.parent == Some(id)
                        && n.bounds.contains(event.position)
                        && n.clip.contains(event.position)
                }),
                m.descriptions
                    .get(&id)
                    .and_then(|d| d.overlay.as_ref())
                    .is_none_or(|d| d.options.dismiss_on_outside),
            )
        };
        if !inside {
            if dismiss {
                self.dismiss_overlay(id, DismissReason::Outside)?;
            }
            self.0.interaction.borrow_mut().swallow_up = true;
            return Ok(true);
        }
        Ok(false)
    }
}
