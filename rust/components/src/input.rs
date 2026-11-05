use crate::{Error, Host, Kind, NodeId, Point, Selection, TextCommand, Value, host::focusable};
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
    pub hovered: Option<NodeId>,
    pub active: bool,
    pub text_anchor: usize,
}
impl Default for Interaction {
    fn default() -> Self {
        Self {
            focus: None,
            pressed: None,
            hovered: None,
            active: true,
            text_anchor: 0,
        }
    }
}
impl Host {
    pub fn focused(&self) -> Option<NodeId> {
        self.0.interaction.borrow().focus
    }
    pub fn set_active(&self, active: bool) {
        if !active {
            let _ = self.unmark_text(true);
        }
        let mut i = self.0.interaction.borrow_mut();
        i.active = active;
        if !active {
            i.pressed = None;
            i.hovered = None;
        }
        drop(i);
        self.redraw();
    }
    pub fn focus(&self, id: NodeId) -> Result<bool, Error> {
        self.unmark_text(true)?;
        self.refresh()?;
        if !self
            .0
            .mounted
            .borrow()
            .nodes
            .iter()
            .any(|n| n.id == id && n.enabled && !n.hidden && focusable(n.kind))
        {
            return Ok(false);
        }
        self.0.interaction.borrow_mut().focus = Some(id);
        self.redraw();
        Ok(true)
    }
    pub fn blur(&self) {
        let _ = self.unmark_text(true);
        let mut i = self.0.interaction.borrow_mut();
        i.focus = None;
        i.pressed = None;
        drop(i);
        self.redraw();
    }
    pub fn focus_next(&self, back: bool) -> Result<bool, Error> {
        self.refresh()?;
        let nodes = self.snapshot()?;
        let ids: Vec<_> = nodes
            .iter()
            .filter(|n| n.enabled && !n.hidden && focusable(n.kind))
            .filter(|n| {
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
                    && !n.hidden
                    && focusable(n.kind)
                    && n.bounds.contains(point)
                    && n.clip.contains(point)
            })
            .map(|n| n.id))
    }
    pub fn pointer(&self, event: Pointer) -> Result<bool, Error> {
        self.refresh()?;
        if event.kind == PointerKind::Cancel {
            self.0.interaction.borrow_mut().pressed = None;
            self.redraw();
            return Ok(true);
        }
        let hit = if event.kind == PointerKind::Leave {
            None
        } else {
            self.hit_test(event.position)?
        };
        self.0.interaction.borrow_mut().hovered = hit;
        let pressed = self.0.interaction.borrow().pressed;
        let mut handled = hit.is_some() || pressed.is_some();
        match event.kind {
            PointerKind::Down if event.button == 0 => {
                if let Some(id) = hit {
                    self.focus(id)?;
                    self.0.interaction.borrow_mut().pressed = Some(id);
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
                    }
                    if kind == Some(Kind::TextField) {
                        let caret = self.text_index(id, event.position);
                        let anchor = if event.modifiers.shift {
                            self.0
                                .editors
                                .borrow()
                                .get(&id)
                                .map_or(caret, |e| e.selection.anchor)
                        } else {
                            caret
                        };
                        self.0.interaction.borrow_mut().text_anchor = anchor;
                        self.select_text(id, Selection { anchor, caret })?;
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
                    } else if kind == Some(Kind::TextField) {
                        let caret = self.text_index(id, event.position);
                        let anchor = self.0.interaction.borrow().text_anchor;
                        self.select_text(id, Selection { anchor, caret })?;
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
                    if !matches!(kind, Some(Kind::Slider | Kind::TextField)) {
                        handled = self.activate(id)?;
                    }
                }
            }
            _ => (),
        }
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
        if event.key == Key::Tab {
            return self.focus_next(event.modifiers.shift);
        }
        let Some(id) = self.focused() else {
            return Ok(false);
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
        if node.kind == Kind::TextField {
            let command = match event.key {
                Key::Left if event.modifiers.command => TextCommand::Start,
                Key::Right if event.modifiers.command => TextCommand::End,
                Key::Left if event.modifiers.alt => TextCommand::WordLeft,
                Key::Right if event.modifiers.alt => TextCommand::WordRight,
                Key::Left => TextCommand::Left,
                Key::Right => TextCommand::Right,
                Key::Home => TextCommand::Start,
                Key::End => TextCommand::End,
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
                    n.parent == node.parent && n.kind == node.kind && n.enabled && !n.hidden
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
        if matches!(event.key, Key::Space | Key::Enter) && !event.repeat {
            return self.activate(id);
        }
        Ok(false)
    }
}
