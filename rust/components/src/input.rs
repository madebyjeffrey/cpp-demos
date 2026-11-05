use crate::{Error, Host, Kind, NodeId, Point, TextCommand, Value, host::focusable};
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Modifiers {
    pub shift: bool,
    pub control: bool,
    pub alt: bool,
    pub command: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PointerKind {
    Down,
    Move,
    Up,
    Leave,
    Cancel,
}
#[derive(Clone, Copy, Debug)]
pub struct Pointer {
    pub kind: PointerKind,
    pub position: Point,
    pub button: i32,
    pub modifiers: Modifiers,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Key {
    Tab,
    Space,
    Enter,
    Escape,
    Left,
    Right,
    Up,
    Down,
    Home,
    End,
    PageUp,
    PageDown,
    Backspace,
    Delete,
    A,
    Unknown,
}
#[derive(Clone, Copy, Debug)]
pub struct KeyEvent {
    pub key: Key,
    pub down: bool,
    pub repeat: bool,
    pub modifiers: Modifiers,
}
pub(crate) struct Interaction {
    pub focus: Option<NodeId>,
    pub pressed: Option<NodeId>,
    pub repeat: Option<(NodeId, std::time::Instant)>,
    pub scroll_drag: Option<crate::scroll::ScrollDrag>,
    pub drag_tick: Option<std::time::Instant>,
    pub hovered: Option<NodeId>,
    pub active: bool,
    pub text_anchor: usize,
    pub text_origin: std::ops::Range<usize>,
    pub text_clicks: u32,
    pub modal: Option<NodeId>,
    pub restore_focus: Option<NodeId>,
    pub hover_since: std::time::Instant,
    pub pointer: Point,
    pub pending_row: Option<(NodeId, String)>,
}
impl Default for Interaction {
    fn default() -> Self {
        Self {
            focus: None,
            pressed: None,
            repeat: None,
            scroll_drag: None,
            drag_tick: None,
            hovered: None,
            active: true,
            text_anchor: 0,
            text_origin: 0..0,
            text_clicks: 1,
            modal: None,
            restore_focus: None,
            hover_since: std::time::Instant::now(),
            pointer: Point::default(),
            pending_row: None,
        }
    }
}
impl Host {
    pub fn active_modal(&self) -> Option<NodeId> {
        self.0
            .mounted
            .borrow()
            .nodes
            .iter()
            .rev()
            .find(|n| n.kind == Kind::Modal && !n.hidden && !n.layout_hidden)
            .map(|n| n.id)
    }
    pub fn in_scope(&self, id: NodeId) -> bool {
        let Some(modal) = self.active_modal() else {
            return true;
        };
        let m = self.0.mounted.borrow();
        let mut current = Some(id);
        while let Some(id) = current {
            if id == modal {
                return true;
            }
            current = m.nodes.iter().find(|n| n.id == id).and_then(|n| n.parent);
        }
        false
    }
    pub(crate) fn reconcile_modal(&self) -> Result<(), Error> {
        let active = self.active_modal();
        let (changed, restore) = {
            let mut i = self.0.interaction.borrow_mut();
            if i.modal == active {
                (false, None)
            } else {
                let restore = if active.is_none() {
                    i.restore_focus.take()
                } else {
                    if i.modal.is_none() {
                        i.restore_focus = i.focus;
                    }
                    None
                };
                i.modal = active;
                (true, restore)
            }
        };
        if changed {
            if let Some(id) = restore {
                self.focus(id)?;
            } else if active.is_some() {
                self.focus_next(false)?;
            }
        }
        Ok(())
    }
    fn reveal(&self, id: NodeId) {
        let bounds = self
            .0
            .mounted
            .borrow()
            .nodes
            .iter()
            .find(|n| n.id == id)
            .map(|n| n.bounds);
        if let Some(bounds) = bounds {
            self.reveal_rect(id, bounds);
        }
    }
    pub(crate) fn tooltip_delay(&self) -> Option<std::time::Duration> {
        let i = self.0.interaction.borrow();
        let id = i.hovered?;
        let m = self.0.mounted.borrow();
        let d = m.descriptions.get(&id)?;
        if d.tooltip.is_empty() {
            return None;
        }
        let wait = std::time::Duration::from_millis(600).saturating_sub(i.hover_since.elapsed());
        (!wait.is_zero()).then_some(wait)
    }
    pub fn tooltip(&self) -> Option<(String, Point)> {
        let i = self.0.interaction.borrow();
        if i.hover_since.elapsed() < std::time::Duration::from_millis(600) {
            return None;
        }
        let id = i.hovered?;
        let m = self.0.mounted.borrow();
        let d = m.descriptions.get(&id)?;
        (!d.tooltip.is_empty() && self.in_scope(id)).then(|| (d.tooltip.clone(), i.pointer))
    }
    pub fn select_collection_item(
        &self,
        id: NodeId,
        index: usize,
        extend: bool,
    ) -> Result<bool, Error> {
        self.refresh()?;
        let model = self
            .0
            .mounted
            .borrow()
            .nodes
            .iter()
            .find(|n| {
                n.id == id && n.enabled && !n.hidden && !n.layout_hidden && self.in_scope(n.id)
            })
            .and_then(|n| n.collection.clone());
        let Some(model) = model else {
            return Ok(false);
        };
        let Some(item) = model.item(index) else {
            return Ok(false);
        };
        if !model.select(&item.key, extend) {
            return Ok(false);
        }
        model.reveal(index);
        self.0.interaction.borrow_mut().pending_row = Some((id, item.key));
        self.redraw();
        Ok(true)
    }
    pub(crate) fn focus_pending_row(&self) -> Result<(), Error> {
        let pending = self.0.interaction.borrow_mut().pending_row.take();
        if let Some((parent, key)) = pending {
            let id = self
                .0
                .mounted
                .borrow()
                .nodes
                .iter()
                .find(|n| n.parent == Some(parent) && n.key.as_ref() == Some(&key))
                .map(|n| n.id);
            if let Some(id) = id {
                self.focus(id)?;
            }
        }
        Ok(())
    }
    pub fn focused(&self) -> Option<NodeId> {
        self.0.interaction.borrow().focus
    }
    pub fn set_active(&self, active: bool) {
        let lost_focus = (!active && self.0.interaction.borrow().active)
            .then(|| self.focused())
            .flatten();
        if !active {
            let _ = self.unmark_text(true);
            self.cancel_field_work();
        }
        let mut i = self.0.interaction.borrow_mut();
        i.active = active;
        if !active {
            i.pressed = None;
            i.scroll_drag = None;
            i.drag_tick = None;
            i.repeat = None;
            i.hovered = None;
        }
        drop(i);
        self.blur_action(lost_focus);
        self.redraw();
    }
    pub fn focus(&self, id: NodeId) -> Result<bool, Error> {
        self.unmark_text(true)?;
        self.refresh()?;
        if !self.0.mounted.borrow().nodes.iter().any(|n| {
            n.id == id
                && n.enabled
                && !n.hidden
                && !n.layout_hidden
                && (focusable(n.kind) || n.kind == Kind::Scroll)
                && self.in_scope(n.id)
        }) {
            return Ok(false);
        }
        if let Some(previous) = self.focused().and_then(|id| self.field_owner(id))
            && Some(previous) != self.field_owner(id)
            && let Some(state) = self.0.fields.borrow_mut().get_mut(&previous)
        {
            state.revealed = false;
        }
        let previous = self.0.interaction.borrow_mut().focus.replace(id);
        if previous != Some(id) {
            let mut i = self.0.interaction.borrow_mut();
            i.pressed = None;
            i.scroll_drag = None;
            i.drag_tick = None;
            i.repeat = None;
        }
        if let Some(editor) = self.0.editors.borrow_mut().get_mut(&id) {
            editor.reveal = true;
        }
        self.reveal(id);
        if previous != Some(id) && self.0.interaction.borrow().active {
            self.blur_action(previous);
        }
        self.redraw();
        Ok(true)
    }
    pub fn blur(&self) {
        let _ = self.unmark_text(true);
        self.cancel_field_work();
        let mut i = self.0.interaction.borrow_mut();
        let previous = i.focus.take().filter(|_| i.active);
        i.pressed = None;
        i.scroll_drag = None;
        i.drag_tick = None;
        i.repeat = None;
        drop(i);
        self.blur_action(previous);
        self.redraw();
    }
    pub fn focus_next(&self, back: bool) -> Result<bool, Error> {
        self.refresh()?;
        let nodes = self.snapshot()?;
        let ids: Vec<_> = nodes
            .iter()
            .filter(|n| {
                n.enabled
                    && !n.hidden
                    && !n.layout_hidden
                    && focusable(n.kind)
                    && self.in_scope(n.id)
            })
            .filter(|n| {
                if n.parent.is_some_and(|p| {
                    nodes
                        .iter()
                        .any(|parent| parent.id == p && parent.kind == Kind::Stepper)
                }) {
                    return false;
                }
                if !matches!(n.kind, Kind::Radio | Kind::Segment) {
                    return true;
                }
                let group: Vec<_> = nodes
                    .iter()
                    .filter(|b| b.parent == n.parent && b.kind == n.kind && b.enabled && !b.hidden)
                    .collect();
                group
                    .iter()
                    .find(|b| b.value.selected())
                    .or_else(|| group.first())
                    .is_some_and(|b| b.id == n.id)
            })
            .map(|n| n.id)
            .collect();
        if ids.is_empty() {
            self.blur();
            return Ok(false);
        }
        let current = self
            .focused()
            .and_then(|id| ids.iter().position(|v| *v == id));
        let index = match (current, back) {
            (Some(i), true) => (i + ids.len() - 1) % ids.len(),
            (Some(i), false) => (i + 1) % ids.len(),
            (None, true) => ids.len() - 1,
            _ => 0,
        };
        self.focus(ids[index])
    }
    pub fn hit_test(&self, point: Point) -> Result<Option<NodeId>, Error> {
        Ok(self
            .scene()?
            .into_iter()
            .rev()
            .find(|n| {
                n.enabled
                    && self.in_scope(n.id)
                    && !n.hidden
                    && !n.layout_hidden
                    && focusable(n.kind)
                    && n.bounds.contains(point)
                    && n.clip.contains(point)
            })
            .map(|n| n.id))
    }
    pub fn pointer(&self, event: Pointer) -> Result<bool, Error> {
        self.pointer_click(event, 1)
    }
    /// Native click count enables word (double) and visual-line (triple) selection.
    pub fn pointer_click(&self, event: Pointer, clicks: u32) -> Result<bool, Error> {
        self.refresh()?;
        let inspector = self.inspector();
        if inspector.picking.get() && event.kind == PointerKind::Down && event.button == 0 {
            self.blur();
            inspector.selected.set(self.pick(event.position)?);
            self.redraw();
            return Ok(true);
        }
        if self.scrollbar_pointer(event)? {
            return Ok(true);
        }
        if event.kind == PointerKind::Cancel {
            self.0.interaction.borrow_mut().pressed = None;
            self.0.interaction.borrow_mut().drag_tick = None;
            self.0.interaction.borrow_mut().repeat = None;
            self.redraw();
            return Ok(true);
        }
        let hit = if event.kind == PointerKind::Leave {
            None
        } else {
            self.hit_test(event.position)?
        };
        {
            let mut i = self.0.interaction.borrow_mut();
            if i.hovered != hit {
                i.hover_since = std::time::Instant::now();
            }
            i.hovered = hit;
            i.pointer = event.position;
        }
        if event.kind == PointerKind::Down && event.button == 1 {
            let action = {
                let m = self.0.mounted.borrow();
                m.nodes
                    .iter()
                    .rev()
                    .filter(|n| {
                        n.enabled
                            && !n.hidden
                            && !n.layout_hidden
                            && self.in_scope(n.id)
                            && n.clip.contains(event.position)
                    })
                    .find_map(|n| m.descriptions.get(&n.id).and_then(|d| d.context.clone()))
            };
            if let Some(action) = action {
                action();
                return Ok(true);
            }
        }
        let pressed = self.0.interaction.borrow().pressed;
        let mut handled = hit.is_some() || pressed.is_some();
        match event.kind {
            PointerKind::Down if event.button == 0 => {
                if let Some(id) = hit {
                    let field_parent = {
                        let m = self.0.mounted.borrow();
                        m.descriptions
                            .get(&id)
                            .filter(|d| d.field_action.is_some())
                            .and_then(|_| {
                                m.nodes.iter().find(|n| n.id == id).and_then(|n| n.parent)
                            })
                    };
                    let step_parent = {
                        let m = self.0.mounted.borrow();
                        m.nodes
                            .iter()
                            .find(|n| n.id == id)
                            .and_then(|n| n.parent)
                            .filter(|p| {
                                m.nodes
                                    .iter()
                                    .any(|n| n.id == *p && n.kind == Kind::Stepper)
                            })
                    };
                    self.focus(field_parent.or(step_parent).unwrap_or(id))?;
                    self.0.interaction.borrow_mut().pressed = Some(id);
                    let kind = self
                        .0
                        .mounted
                        .borrow()
                        .nodes
                        .iter()
                        .find(|n| n.id == id)
                        .map(|n| n.kind);
                    if self.repeating(id).is_some() {
                        self.start_repeat(id)?;
                    }
                    if kind == Some(Kind::Slider) {
                        self.slider_pointer(id, event.position)?;
                    }
                    if kind == Some(Kind::TextField) {
                        self.begin_text_selection(
                            id,
                            event.position,
                            event.modifiers.shift,
                            clicks,
                        )?;
                    }
                } else if let Some(modal) = self.active_modal() {
                    let nodes = self.snapshot()?;
                    let inside = nodes
                        .iter()
                        .any(|n| n.parent == Some(modal) && n.bounds.contains(event.position));
                    if !inside {
                        self.activate(modal)?;
                    }
                } else {
                    self.blur();
                }
            }
            PointerKind::Move => {
                if let Some(id) = pressed {
                    let kind = self
                        .0
                        .mounted
                        .borrow()
                        .nodes
                        .iter()
                        .find(|n| n.id == id)
                        .map(|n| n.kind);
                    if kind == Some(Kind::Slider) {
                        self.slider_pointer(id, event.position)?;
                    } else if kind == Some(Kind::TextField)
                        && let Some(selection) = self.drag_selection(id, event.position)
                    {
                        self.select_pointer_text(id, event.position, selection)?;
                    }
                }
            }
            PointerKind::Up if event.button == 0 => {
                self.0.interaction.borrow_mut().pressed = None;
                if let Some(id) = pressed
                    && Some(id) == hit
                {
                    let kind = self
                        .0
                        .mounted
                        .borrow()
                        .nodes
                        .iter()
                        .find(|n| n.id == id)
                        .map(|n| n.kind);
                    if kind == Some(Kind::CollectionRow) {
                        let row = self.snapshot()?.into_iter().find(|n| n.id == id).unwrap();
                        let parent = row.parent.unwrap();
                        let model = self
                            .0
                            .mounted
                            .borrow()
                            .nodes
                            .iter()
                            .find(|n| n.id == parent)
                            .and_then(|n| n.collection.clone());
                        if let Some(model) = model
                            && let Some(index) =
                                row.key.as_deref().and_then(|key| model.index_of(key))
                        {
                            handled = self.select_collection_item(
                                parent,
                                index,
                                event.modifiers.shift || event.modifiers.command,
                            )?;
                        }
                    } else if !matches!(kind, Some(Kind::Slider | Kind::TextField))
                        && self.repeating(id).is_none()
                    {
                        handled = self.activate(id)?;
                    }
                }
            }
            _ => (),
        }
        self.update_drag_deadline(std::time::Instant::now());
        self.update_repeat();
        self.redraw();
        Ok(handled)
    }
    fn slider_pointer(&self, id: NodeId, p: Point) -> Result<bool, Error> {
        let info = self
            .0
            .mounted
            .borrow()
            .nodes
            .iter()
            .find(|n| n.id == id)
            .map(|n| (n.bounds, n.range));
        let Some((b, Some((min, max, _)))) = info else {
            return Ok(false);
        };
        let t = ((p.x - b.x - 12.) / (b.width - 24.).max(1.)).clamp(0., 1.);
        self.set_number(id, min + (max - min) * t)
    }
    pub fn key(&self, event: KeyEvent) -> Result<bool, Error> {
        if !event.down {
            return Ok(false);
        }
        self.refresh()?;
        let inspector = self.inspector();
        if event.key == Key::Escape && inspector.picking.get() {
            inspector.picking.set(false);
            self.redraw();
            return Ok(true);
        }
        if event.key == Key::Escape
            && let Some(id) = self.active_modal()
        {
            return self.activate(id);
        }
        if event.key == Key::Tab {
            return self.focus_next(event.modifiers.shift);
        }
        let Some(id) = self.focused() else {
            return self.scroll_key(None, event);
        };
        let node = self
            .0
            .mounted
            .borrow()
            .nodes
            .iter()
            .find(|n| n.id == id)
            .cloned();
        let Some(node) = node else { return Ok(false) };
        if node.kind == Kind::CollectionRow
            && matches!(
                event.key,
                Key::Up | Key::Down | Key::Home | Key::End | Key::PageUp | Key::PageDown
            )
        {
            let parent = node.parent.unwrap();
            let model = self
                .0
                .mounted
                .borrow()
                .nodes
                .iter()
                .find(|n| n.id == parent)
                .and_then(|n| n.collection.clone());
            if let Some(model) = model {
                let current = node
                    .key
                    .as_deref()
                    .and_then(|key| model.index_of(key))
                    .unwrap_or(0);
                let next = match event.key {
                    Key::Home => 0,
                    Key::End => model.len().saturating_sub(1),
                    Key::Up => current.saturating_sub(1),
                    Key::PageUp => current.saturating_sub(10),
                    Key::PageDown => (current + 10).min(model.len().saturating_sub(1)),
                    _ => (current + 1).min(model.len().saturating_sub(1)),
                };
                return self.select_collection_item(parent, next, event.modifiers.shift);
            }
        }
        if node.kind == Kind::Button
            && self.active_modal().is_some()
            && matches!(event.key, Key::Up | Key::Down)
        {
            return self.focus_next(event.key == Key::Up);
        }
        if node.kind == Kind::TextField {
            let command = match event.key {
                Key::Left if event.modifiers.command => TextCommand::LineStart,
                Key::Right if event.modifiers.command => TextCommand::LineEnd,
                Key::Left if event.modifiers.alt => TextCommand::WordLeft,
                Key::Right if event.modifiers.alt => TextCommand::WordRight,
                Key::Left => TextCommand::Left,
                Key::Right => TextCommand::Right,
                Key::Up if event.modifiers.command => TextCommand::Start,
                Key::Down if event.modifiers.command => TextCommand::End,
                Key::Up if event.modifiers.alt => TextCommand::ParagraphStart,
                Key::Down if event.modifiers.alt => TextCommand::ParagraphEnd,
                Key::Up => TextCommand::Up,
                Key::Down => TextCommand::Down,
                Key::Home if event.modifiers.control => TextCommand::Start,
                Key::End if event.modifiers.control => TextCommand::End,
                Key::Home => TextCommand::LineStart,
                Key::End => TextCommand::LineEnd,
                Key::PageUp => TextCommand::PageUp,
                Key::PageDown => TextCommand::PageDown,
                Key::Backspace if event.modifiers.command => TextCommand::DeleteLineBackward,
                Key::Delete if event.modifiers.command => TextCommand::DeleteLineForward,
                Key::Backspace if event.modifiers.alt => TextCommand::DeleteWordBackward,
                Key::Delete if event.modifiers.alt => TextCommand::DeleteWordForward,
                Key::Backspace => TextCommand::Backspace,
                Key::Delete => TextCommand::Delete,
                Key::Enter => TextCommand::Enter,
                Key::Escape => TextCommand::Escape,
                Key::A if event.modifiers.command => TextCommand::SelectAll,
                _ => return Ok(false),
            };
            return self.text_command(command, event.modifiers.shift);
        }
        if matches!(node.kind, Kind::Radio | Kind::Segment)
            && matches!(
                event.key,
                Key::Left | Key::Right | Key::Up | Key::Down | Key::Home | Key::End
            )
        {
            let ids: Vec<_> = self
                .snapshot()?
                .iter()
                .filter(|n| {
                    n.parent == node.parent
                        && n.kind == node.kind
                        && n.enabled
                        && !n.hidden
                        && !n.layout_hidden
                })
                .map(|n| n.id)
                .collect();
            let current = ids.iter().position(|v| *v == id).unwrap();
            let next = match event.key {
                Key::Home => 0,
                Key::End => ids.len() - 1,
                Key::Left | Key::Up => (current + ids.len() - 1) % ids.len(),
                _ => (current + 1) % ids.len(),
            };
            self.focus(ids[next])?;
            return self.activate(ids[next]);
        }
        if node.kind == Kind::Stepper
            && matches!(event.key, Key::Up | Key::Right | Key::Down | Key::Left)
        {
            return self.step_control(id, matches!(event.key, Key::Up | Key::Right));
        }
        if node.kind == Kind::Slider
            && let (Some((min, max, step)), Value::Number(value)) = (node.range, node.value)
        {
            let step = if step > 0. { step } else { (max - min) / 100. };
            let scale = if event.modifiers.shift { 10. } else { 1. };
            let value = match event.key {
                Key::Home => min,
                Key::End => max,
                Key::Left | Key::Down => value - step * scale,
                Key::Right | Key::Up => value + step * scale,
                Key::PageDown => value - step * 10.,
                Key::PageUp => value + step * 10.,
                _ => return Ok(false),
            };
            return self.set_number(id, value);
        }
        if matches!(event.key, Key::Space | Key::Enter)
            && (!event.repeat || self.repeating(id).is_some())
        {
            return self.activate(id);
        }
        self.scroll_key(Some(id), event)
    }
}
