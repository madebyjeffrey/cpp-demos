use crate::{Error, Host, NodeId, Rect, Selection, TextBuffer, TextCommand, view::Data};
use std::ops::{Deref, DerefMut, Range};
#[derive(Clone, Debug)]
pub struct TextInput {
    pub id: NodeId,
    pub text: String,
    pub selection: Selection,
    pub marked: Option<Range<usize>>,
    pub editable: bool,
    pub active: bool,
    pub bounds: Rect,
    pub secure: bool,
    pub multiline: bool,
    pub can_undo: bool,
    pub can_redo: bool,
}
#[derive(Default)]
pub(crate) struct Editor {
    pub buffer: TextBuffer,
    pub source: String,
    pub scroll: f64,
    pub scroll_y: f64,
    pub reveal: bool,
}
impl Deref for Editor {
    type Target = TextBuffer;
    fn deref(&self) -> &TextBuffer {
        &self.buffer
    }
}
impl DerefMut for Editor {
    fn deref_mut(&mut self) -> &mut TextBuffer {
        &mut self.buffer
    }
}
impl Editor {
    pub fn sync(&mut self, value: &str) {
        if self.source != value {
            self.buffer.set_text(value);
            self.source = value.into();
            self.scroll = 0.;
            self.scroll_y = 0.;
            self.reveal = true;
        }
    }
}
impl Host {
    pub fn set_text(&self, id: NodeId, text: &str) -> Result<bool, Error> {
        self.refresh()?;
        if !self
            .0
            .mounted
            .borrow()
            .nodes
            .iter()
            .any(|n| n.id == id && n.enabled && !n.hidden && !n.layout_hidden && !n.read_only)
        {
            return Ok(false);
        }
        {
            let mut editors = self.0.editors.borrow_mut();
            let Some(e) = editors.get_mut(&id) else {
                return Ok(false);
            };
            let length = e.text.len();
            e.replace(0..length, text);
            e.reveal = true;
        }
        self.publish_text(id);
        self.redraw();
        Ok(true)
    }
    /// A discrete clipboard edit; unlike typing, it starts a separate undo group.
    pub fn paste_text(&self, text: &str) -> Result<bool, Error> {
        let Some(input) = self.text_input()?.filter(|i| i.editable && i.active) else {
            return Ok(false);
        };
        self.insert_text(text, Some(input.selection.range()))
    }
    pub fn text_input(&self) -> Result<Option<TextInput>, Error> {
        self.refresh()?;
        let i = self.0.interaction.borrow();
        let Some(id) = i.focus else { return Ok(None) };
        let m = self.0.mounted.borrow();
        let Some(node) = m
            .nodes
            .iter()
            .find(|n| n.id == id && n.enabled && !n.hidden && !n.layout_hidden)
        else {
            return Ok(None);
        };
        let editors = self.0.editors.borrow();
        Ok(editors.get(&id).map(|e| TextInput {
            id,
            text: e.text.clone(),
            selection: e.selection,
            marked: e.marked.clone(),
            editable: !node.read_only,
            active: i.active,
            bounds: node.bounds.inset(10.),
            secure: node.secure,
            multiline: node.multiline,
            can_undo: e.can_undo(),
            can_redo: e.can_redo(),
        }))
    }
    pub fn select_text(&self, id: NodeId, selection: Selection) -> Result<bool, Error> {
        self.refresh()?;
        let mut editors = self.0.editors.borrow_mut();
        let Some(e) = editors.get_mut(&id) else {
            return Ok(false);
        };
        e.reveal = true;
        e.select(selection);
        drop(editors);
        self.redraw();
        Ok(true)
    }
    pub fn insert_text(
        &self,
        text: &str,
        replacement: Option<Range<usize>>,
    ) -> Result<bool, Error> {
        let Some(input) = self.text_input()?.filter(|i| i.editable && i.active) else {
            return Ok(false);
        };
        {
            let mut editors = self.0.editors.borrow_mut();
            let e = editors.get_mut(&input.id).unwrap();
            if let Some(range) = replacement {
                e.replace(range, text);
            } else {
                e.insert(text);
            }
            e.reveal = true;
        }
        self.publish_text(input.id);
        self.redraw();
        Ok(true)
    }
    pub fn mark_text(
        &self,
        text: &str,
        selected: Range<usize>,
        replacement: Option<Range<usize>>,
    ) -> Result<bool, Error> {
        let Some(input) = self.text_input()?.filter(|i| i.editable && i.active) else {
            return Ok(false);
        };
        {
            let mut editors = self.0.editors.borrow_mut();
            let e = editors.get_mut(&input.id).unwrap();
            if !e.mark(text, selected, replacement) {
                return Ok(false);
            }
            e.reveal = true;
        }
        self.redraw();
        Ok(true)
    }
    pub fn unmark_text(&self, commit: bool) -> Result<bool, Error> {
        let Some(input) = self.text_input()? else {
            return Ok(false);
        };
        let changed = {
            let mut editors = self.0.editors.borrow_mut();
            let e = editors.get_mut(&input.id).unwrap();
            let changed = e.marked.is_some();
            e.unmark(commit);
            changed
        };
        if changed && commit {
            self.publish_text(input.id);
        }
        if changed {
            self.redraw();
        }
        Ok(changed)
    }
    fn publish_text(&self, id: NodeId) {
        let binding = {
            let m = self.0.mounted.borrow();
            m.descriptions.get(&id).and_then(|d| {
                if let Data::Text(b) = &d.data {
                    Some(b.clone())
                } else {
                    None
                }
            })
        };
        let text = self.0.editors.borrow().get(&id).map(|e| e.text.clone());
        if let (Some(binding), Some(text)) = (binding, text) {
            binding.set(text);
            let accepted = binding.get();
            if let Some(e) = self.0.editors.borrow_mut().get_mut(&id) {
                e.source = accepted.clone();
                if e.text != accepted {
                    e.buffer.set_text(accepted);
                }
            }
        }
    }
    pub fn selected_text(&self) -> Result<Option<String>, Error> {
        Ok(self
            .text_input()?
            .filter(|i| !i.secure)
            .map(|i| i.text[i.selection.range()].to_owned()))
    }
    pub fn text_command(&self, command: TextCommand, extend: bool) -> Result<bool, Error> {
        if command == TextCommand::Tab || command == TextCommand::BackTab {
            return self.focus_next(command == TextCommand::BackTab);
        }
        let Some(input) = self.text_input()?.filter(|i| i.active) else {
            return Ok(false);
        };
        if command == TextCommand::Enter && input.multiline {
            return self.insert_text("\n", None);
        }
        if command == TextCommand::Escape {
            if self.unmark_text(false)? {
                return Ok(true);
            }
            let action = self
                .0
                .mounted
                .borrow()
                .descriptions
                .get(&input.id)
                .and_then(|d| d.cancel.clone());
            if let Some(action) = action {
                action();
                return Ok(true);
            }
            return Ok(false);
        }
        if command == TextCommand::Enter {
            self.unmark_text(true)?;
            let action = self
                .0
                .mounted
                .borrow()
                .descriptions
                .get(&input.id)
                .and_then(|d| d.action.clone());
            if let Some(action) = action {
                action();
            }
            return Ok(true);
        }
        let handled = {
            let mut editors = self.0.editors.borrow_mut();
            let e = editors.get_mut(&input.id).unwrap();
            e.reveal = true;
            let m = self.0.mounted.borrow();
            if let Some(layout) = m.layouts.get(&input.id) {
                e.navigate(command, extend, layout, input.bounds.height)
            } else {
                e.command(command, extend)
            }
        };
        let changed = self
            .0
            .editors
            .borrow()
            .get(&input.id)
            .is_some_and(|e| e.committed_text() != e.source);
        if changed {
            self.publish_text(input.id);
        }
        if handled || changed {
            self.redraw();
        }
        Ok(handled || changed)
    }
    fn selection_range_at(
        &self,
        id: NodeId,
        point: crate::Point,
        clicks: u32,
    ) -> Option<Range<usize>> {
        let index = self.text_index(id, point);
        let m = self.0.mounted.borrow();
        let node = m.nodes.iter().find(|n| n.id == id)?;
        let editors = self.0.editors.borrow();
        let editor = editors.get(&id)?;
        if clicks > 1 && node.secure {
            return Some(0..editor.text.len());
        }
        if clicks >= 3 {
            if let Some(layout) = m.layouts.get(&id)
                && !layout.lines.is_empty()
            {
                let y = point.y - node.text_bounds.y;
                let line = layout
                    .lines
                    .iter()
                    .rev()
                    .find(|l| l.bounds.y <= y)
                    .unwrap_or(&layout.lines[0]);
                return Some(line.range.start..line.end_including_newline);
            }
            let start = editor.text[..index].rfind('\n').map_or(0, |i| i + 1);
            let end = editor.text[index..]
                .find('\n')
                .map_or(editor.text.len(), |i| index + i + 1);
            return Some(start..end);
        }
        Some(if clicks == 2 {
            editor.word_range(index)
        } else {
            index..index
        })
    }
    pub(crate) fn select_pointer_text(
        &self,
        id: NodeId,
        point: crate::Point,
        selection: Selection,
    ) -> Result<bool, Error> {
        let selected = self.select_text(id, selection)?;
        let upstream = {
            let m = self.0.mounted.borrow();
            m.nodes
                .iter()
                .find(|n| n.id == id)
                .and_then(|n| m.layouts.get(&id).map(|l| (l, point.y - n.text_bounds.y)))
                .is_some_and(|(layout, y)| {
                    layout.lines.iter().any(|line| {
                        line.range.end == selection.caret
                            && line.bounds.y <= y
                            && y < line.bounds.y + line.bounds.height
                    })
                })
        };
        if upstream && let Some(e) = self.0.editors.borrow_mut().get_mut(&id) {
            e.affinity = crate::CaretAffinity::Upstream;
        }
        Ok(selected)
    }
    pub(crate) fn begin_text_selection(
        &self,
        id: NodeId,
        point: crate::Point,
        extend: bool,
        clicks: u32,
    ) -> Result<(), Error> {
        let Some(range) = self.selection_range_at(id, point, clicks) else {
            return Ok(());
        };
        let anchor = if extend {
            self.0
                .editors
                .borrow()
                .get(&id)
                .map_or(range.start, |e| e.selection.anchor)
        } else {
            range.start
        };
        {
            let mut i = self.0.interaction.borrow_mut();
            i.text_anchor = anchor;
            i.text_origin = range.clone();
            i.text_clicks = clicks.max(1);
        }
        self.select_pointer_text(
            id,
            point,
            Selection {
                anchor,
                caret: range.end,
            },
        )?;
        Ok(())
    }
    pub(crate) fn drag_selection(&self, id: NodeId, point: crate::Point) -> Option<Selection> {
        let i = self.0.interaction.borrow();
        let (clicks, origin, anchor) = (i.text_clicks, i.text_origin.clone(), i.text_anchor);
        drop(i);
        let target = self.selection_range_at(id, point, clicks)?;
        Some(if clicks > 1 && target.start < origin.start {
            Selection {
                anchor: origin.end,
                caret: target.start,
            }
        } else {
            Selection {
                anchor,
                caret: target.end,
            }
        })
    }
    pub fn text_index(&self, id: NodeId, point: crate::Point) -> usize {
        let m = self.0.mounted.borrow();
        let Some(n) = m.nodes.iter().find(|n| n.id == id) else {
            return 0;
        };
        m.layouts.get(&id).map_or(0, |l| {
            (l.hit_test)(crate::Point {
                x: point.x - n.text_bounds.x,
                y: point.y - n.text_bounds.y,
            })
        })
    }
    pub fn text_rectangles(&self, id: NodeId, range: Range<usize>) -> Vec<Rect> {
        let m = self.0.mounted.borrow();
        let Some(n) = m.nodes.iter().find(|n| n.id == id) else {
            return Vec::new();
        };
        m.layouts.get(&id).map_or_else(Vec::new, |l| {
            let rectangles = if range.is_empty() {
                let affinity = self
                    .0
                    .editors
                    .borrow()
                    .get(&id)
                    .filter(|e| e.selection.caret == range.start)
                    .map_or(crate::CaretAffinity::Downstream, |e| e.affinity());
                vec![l.caret_rect(range.start, affinity)]
            } else {
                (l.range_rects)(range)
            };
            rectangles
                .into_iter()
                .map(|r| {
                    Rect::new(
                        r.x + n.text_bounds.x,
                        r.y + n.text_bounds.y,
                        r.width,
                        r.height,
                    )
                })
                .collect()
        })
    }
}
