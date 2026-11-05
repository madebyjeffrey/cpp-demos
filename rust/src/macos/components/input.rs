use super::*;
use ui::{TextCommand, text};
pub(super) struct Client {
    pub owner: Weak<Mounted>,
}
fn range(text: &str, range: TextRange) -> Option<std::ops::Range<usize>> {
    let end = range.start.checked_add(range.length)?;
    Some(ui::text::byte_offset(text, range.start)?..ui::text::byte_offset(text, end)?)
}
fn native(text: &str, range: std::ops::Range<usize>) -> Option<TextRange> {
    let start = text::utf16_offset(text, range.start)?;
    Some(TextRange {
        start,
        length: text::utf16_offset(text, range.end)? - start,
    })
}
impl TextInputClient for Client {
    fn state(&self) -> Option<TextInputState> {
        self.owner
            .upgrade()?
            .run(|m, _| {
                Ok(m.host.text_input()?.and_then(|i| {
                    Some(TextInputState {
                        text: i.text.clone(),
                        selection: native(&i.text, i.selection.range())?,
                        marked: i.marked.and_then(|r| native(&i.text, r)),
                        editable: i.editable,
                        multiline: i.multiline,
                        can_undo: i.can_undo,
                        can_redo: i.can_redo,
                        secure: i.secure,
                        active: i.active,
                    })
                }))
            })
            .flatten()
    }
    fn insert(&self, value: String, replacement: Option<TextRange>) {
        if let Some(m) = self.owner.upgrade() {
            m.run(|m, _| {
                let Some(input) = m.host.text_input()? else {
                    return Ok(());
                };
                let replacement = match replacement {
                    Some(r) => {
                        let Some(r) = range(&input.text, r) else {
                            return Ok(());
                        };
                        Some(r)
                    }
                    None => None,
                };
                m.host.insert_text(&value, replacement)?;
                Ok(())
            });
        }
    }
    fn mark(&self, value: String, selection: TextRange, replacement: Option<TextRange>) {
        if let Some(m) = self.owner.upgrade() {
            m.run(|m, _| {
                let Some(input) = m.host.text_input()? else {
                    return Ok(());
                };
                let Some(selected) = range(&value, selection) else {
                    return Ok(());
                };
                let replacement = match replacement {
                    Some(r) => {
                        let Some(r) = range(&input.text, r) else {
                            return Ok(());
                        };
                        Some(r)
                    }
                    None => None,
                };
                m.host.mark_text(&value, selected, replacement)?;
                Ok(())
            });
        }
    }
    fn unmark(&self, commit: bool) {
        if let Some(m) = self.owner.upgrade() {
            m.run(|m, _| m.host.unmark_text(commit).map(|_| ()));
        }
    }
    fn command(&self, selector: &str) -> bool {
        self.owner
            .upgrade()
            .and_then(|m| m.run(|m, _| command(&m.host, selector)))
            .unwrap_or(false)
    }
    fn rectangle(&self, r: TextRange) -> Rect {
        self.owner
            .upgrade()
            .and_then(|m| {
                m.run(|m, _| {
                    let Some(i) = m.host.text_input()? else {
                        return Ok(Rect::default());
                    };
                    let Some(r) = range(&i.text, r) else {
                        return Ok(Rect::default());
                    };
                    let r = m
                        .host
                        .text_rectangles(i.id, r)
                        .first()
                        .copied()
                        .unwrap_or_default();
                    Ok(Rect {
                        origin: Point::new(r.x, r.y),
                        size: Size::new(r.width, r.height),
                    })
                })
            })
            .unwrap_or_default()
    }
    fn index(&self, p: Point) -> usize {
        self.owner
            .upgrade()
            .and_then(|m| {
                m.run(|m, _| {
                    let Some(i) = m.host.text_input()? else {
                        return Ok(0);
                    };
                    Ok(text::utf16_offset(
                        &i.text,
                        m.host.text_index(i.id, ui::Point { x: p.x, y: p.y }),
                    )
                    .unwrap_or(0))
                })
            })
            .unwrap_or(0)
    }
    fn intercept_key(&self, key: &KeyEvent) -> bool {
        if key.down && key.modifiers.command {
            let command = match key.key {
                Key::C => "copy:",
                Key::X => "cut:",
                Key::V => "paste:",
                Key::A => "selectAll:",
                Key::Z if key.modifiers.shift => "redo:",
                Key::Z => "undo:",
                _ => return false,
            };
            return self.command(command);
        }
        false
    }
    fn deactivate(&self) {
        if let Some(m) = self.owner.upgrade() {
            m.run(|m, _| {
                m.host.set_active(false);
                Ok(())
            });
        }
    }
}
fn command(host: &ui::Host, selector: &str) -> std::result::Result<bool, ui::Error> {
    if matches!(selector, "copy:" | "cut:") {
        if let Some(value) = host.selected_text()? {
            unsafe {
                let p = NSPasteboard::generalPasteboard();
                p.clearContents();
                p.setString_forType(&ns(&value), NSPasteboardTypeString);
            }
        }
        if selector == "cut:" && host.selected_text()?.is_some() {
            return host.insert_text("", None);
        }
        return Ok(true);
    }
    if selector == "paste:" {
        let value = unsafe {
            NSPasteboard::generalPasteboard()
                .stringForType(NSPasteboardTypeString)
                .map(|s| s.to_string())
                .unwrap_or_default()
        };
        return host.insert_text(&value, None);
    }
    let extend = selector.contains("AndModifySelection");
    let command = match selector
        .trim_end_matches(':')
        .trim_end_matches("AndModifySelection")
    {
        "moveLeft" | "moveBackward" => TextCommand::Left,
        "moveRight" | "moveForward" => TextCommand::Right,
        "moveWordLeft" | "moveWordBackward" => TextCommand::WordLeft,
        "moveWordRight" | "moveWordForward" => TextCommand::WordRight,
        "moveToBeginningOfLine" | "moveToBeginningOfDocument" | "moveToLeftEndOfLine" => {
            TextCommand::Start
        }
        "moveToEndOfLine" | "moveToEndOfDocument" | "moveToRightEndOfLine" => TextCommand::End,
        "deleteBackward" => TextCommand::Backspace,
        "deleteForward" => TextCommand::Delete,
        "selectAll" => TextCommand::SelectAll,
        "undo" => TextCommand::Undo,
        "redo" => TextCommand::Redo,
        "moveUp" => TextCommand::Up,
        "moveDown" => TextCommand::Down,
        "insertNewline" | "insertNewlineIgnoringFieldEditor" => TextCommand::Enter,
        "cancelOperation" => TextCommand::Escape,
        "insertTab" => TextCommand::Tab,
        "insertBacktab" => TextCommand::BackTab,
        _ => return Ok(false),
    };
    host.text_command(command, extend)
}
pub(super) fn event(m: &Mounted, event: &Event) -> std::result::Result<bool, ui::Error> {
    let modifiers = |v: Modifiers| ui::Modifiers {
        shift: v.shift,
        control: v.control,
        alt: v.alt,
        command: v.command,
    };
    match event {
        Event::Pointer(p) => {
            let kind = match p.kind {
                PointerKind::Down => ui::PointerKind::Down,
                PointerKind::Up => ui::PointerKind::Up,
                PointerKind::Move | PointerKind::Enter => ui::PointerKind::Move,
                PointerKind::Leave => ui::PointerKind::Leave,
                PointerKind::Scroll => {
                    return m.host.scroll_at(
                        ui::Point {
                            x: p.position.x,
                            y: p.position.y,
                        },
                        ui::Point {
                            x: -p.scroll_x,
                            y: -p.scroll_y,
                        },
                    );
                }
            };
            m.host.pointer(ui::Pointer {
                kind,
                position: ui::Point {
                    x: p.position.x,
                    y: p.position.y,
                },
                button: p.button,
                modifiers: modifiers(p.modifiers),
            })
        }
        Event::Key(k) => {
            let key = match k.key {
                Key::Tab => ui::Key::Tab,
                Key::Space => ui::Key::Space,
                Key::Enter => ui::Key::Enter,
                Key::Escape => ui::Key::Escape,
                Key::Left => ui::Key::Left,
                Key::Right => ui::Key::Right,
                Key::Up => ui::Key::Up,
                Key::Down => ui::Key::Down,
                Key::Home => ui::Key::Home,
                Key::End => ui::Key::End,
                Key::PageUp => ui::Key::PageUp,
                Key::PageDown => ui::Key::PageDown,
                Key::Backspace => ui::Key::Backspace,
                Key::DeleteForward => ui::Key::Delete,
                Key::A => ui::Key::A,
                _ => ui::Key::Unknown,
            };
            m.host.key(ui::KeyEvent {
                key,
                down: k.down,
                repeat: k.repeat,
                modifiers: modifiers(k.modifiers),
            })
        }
        Event::Focus(active) => {
            m.host.set_active(*active);
            Ok(false)
        }
        _ => Ok(false),
    }
}
