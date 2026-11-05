use crate::{
    Error, Host, NodeId, Rect,
    text::{grapheme_boundaries, word_boundaries},
    view::Data,
};
use std::ops::Range;
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Selection {
    pub anchor: usize,
    pub caret: usize,
}
impl Selection {
    pub fn range(self) -> Range<usize> {
        self.anchor.min(self.caret)..self.anchor.max(self.caret)
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextCommand {
    Left,
    Right,
    WordLeft,
    WordRight,
    Start,
    End,
    Backspace,
    Delete,
    SelectAll,
    Enter,
    Escape,
    Tab,
    BackTab,
}
#[derive(Clone, Debug)]
pub struct TextInput {
    pub id: NodeId,
    pub text: String,
    pub selection: Selection,
    pub marked: Option<Range<usize>>,
    pub editable: bool,
    pub active: bool,
    pub bounds: Rect,
}
#[derive(Default)]
pub(crate) struct Editor {
    pub text: String,
    pub source: String,
    pub selection: Selection,
    pub marked: Option<Range<usize>>,
    base: Option<(String, Selection)>,
    pub scroll: f64,
}
fn floor(text: &str, index: usize) -> usize {
    grapheme_boundaries(text)
        .into_iter()
        .take_while(|b| *b <= index)
        .last()
        .unwrap_or(0)
}
fn ceil(text: &str, index: usize) -> usize {
    grapheme_boundaries(text)
        .into_iter()
        .find(|b| *b >= index)
        .unwrap_or(text.len())
}
impl Editor {
    pub fn sync(&mut self, value: &str) {
        if self.source != value {
            self.text = value.to_owned();
            self.source = value.to_owned();
            self.selection.anchor = floor(value, self.selection.anchor);
            self.selection.caret = floor(value, self.selection.caret);
            self.marked = None;
            self.base = None;
            self.scroll = 0.;
        }
    }
    fn replace(&mut self, value: &str, range: Range<usize>) -> Range<usize> {
        let start = floor(&self.text, range.start.min(self.text.len()));
        let end = if range.start == range.end {
            start
        } else {
            ceil(&self.text, range.end.min(self.text.len()))
        };
        self.text.replace_range(start..end, value);
        let end = start + value.len();
        self.selection = Selection {
            anchor: end,
            caret: end,
        };
        start..end
    }
    fn unmark(&mut self, commit: bool) {
        if !commit && let Some((text, selection)) = self.base.take() {
            self.text = text;
            self.selection = selection;
        }
        self.base = None;
        self.marked = None;
    }
}
impl Host {
    pub fn set_text(&self, id: NodeId, text: &str) -> Result<bool, Error> {
        self.refresh()?;
        let binding = {
            let m = self.0.mounted.borrow();
            m.nodes
                .iter()
                .find(|n| n.id == id && n.enabled && !n.hidden && !n.read_only)
                .and_then(|_| m.descriptions.get(&id))
                .and_then(|d| {
                    if let Data::Text(b) = &d.data {
                        Some(b.clone())
                    } else {
                        None
                    }
                })
        };
        let Some(binding) = binding else {
            return Ok(false);
        };
        binding.set(
            text.chars()
                .filter(|c| !matches!(c, '\n' | '\r' | '\0'))
                .collect(),
        );
        self.redraw();
        Ok(true)
    }
    pub fn text_input(&self) -> Result<Option<TextInput>, Error> {
        self.refresh()?;
        let i = self.0.interaction.borrow();
        let Some(id) = i.focus else { return Ok(None) };
        let m = self.0.mounted.borrow();
        let Some(node) = m
            .nodes
            .iter()
            .find(|n| n.id == id && n.enabled && !n.hidden)
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
        }))
    }
    pub fn select_text(&self, id: NodeId, selection: Selection) -> Result<bool, Error> {
        self.refresh()?;
        let mut editors = self.0.editors.borrow_mut();
        let Some(e) = editors.get_mut(&id) else {
            return Ok(false);
        };
        e.selection = Selection {
            anchor: floor(&e.text, selection.anchor),
            caret: floor(&e.text, selection.caret),
        };
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
        let text: String = text
            .chars()
            .filter(|c| *c != '\n' && *c != '\r' && *c != '\0')
            .collect();
        {
            let mut editors = self.0.editors.borrow_mut();
            let e = editors.get_mut(&input.id).unwrap();
            let range = replacement
                .or_else(|| e.marked.clone())
                .unwrap_or_else(|| e.selection.range());
            e.replace(&text, range);
            e.unmark(true);
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
        if text.contains(['\n', '\r', '\0']) {
            return Ok(false);
        }
        {
            let mut editors = self.0.editors.borrow_mut();
            let e = editors.get_mut(&input.id).unwrap();
            if e.base.is_none() {
                e.base = Some((e.text.clone(), e.selection));
            }
            let range = replacement
                .or_else(|| e.marked.clone())
                .unwrap_or_else(|| e.selection.range());
            let range = e.replace(text, range);
            e.selection = Selection {
                anchor: range.start + floor(text, selected.start),
                caret: range.start + floor(text, selected.end),
            };
            e.marked = Some(range);
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
        }
    }
    pub fn selected_text(&self) -> Result<Option<String>, Error> {
        Ok(self
            .text_input()?
            .map(|i| i.text[i.selection.range()].to_owned()))
    }
    pub fn text_command(&self, command: TextCommand, extend: bool) -> Result<bool, Error> {
        if command == TextCommand::Tab || command == TextCommand::BackTab {
            return self.focus_next(command == TextCommand::BackTab);
        }
        let Some(input) = self.text_input()?.filter(|i| i.active) else {
            return Ok(false);
        };
        if command == TextCommand::Escape {
            return self.unmark_text(false);
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
        self.unmark_text(true)?;
        let mut changed = false;
        {
            let mut editors = self.0.editors.borrow_mut();
            let Some(e) = editors.get_mut(&input.id) else {
                return Ok(false);
            };
            let selection = e.selection.range();
            let caret = e.selection.caret;
            let boundaries = grapheme_boundaries(&e.text);
            let previous = boundaries
                .iter()
                .copied()
                .take_while(|b| *b < caret)
                .last()
                .unwrap_or(0);
            let next = boundaries
                .iter()
                .copied()
                .find(|b| *b > caret)
                .unwrap_or(e.text.len());
            let target = match command {
                TextCommand::Start => 0,
                TextCommand::End => e.text.len(),
                TextCommand::Left if !extend && !selection.is_empty() => selection.start,
                TextCommand::Right if !extend && !selection.is_empty() => selection.end,
                TextCommand::Left => previous,
                TextCommand::Right => next,
                TextCommand::WordLeft => word_boundaries(&e.text)
                    .into_iter()
                    .take_while(|b| *b < caret)
                    .last()
                    .unwrap_or(0),
                TextCommand::WordRight => word_boundaries(&e.text)
                    .into_iter()
                    .find(|b| *b > caret)
                    .unwrap_or(e.text.len()),
                TextCommand::SelectAll => {
                    e.selection = Selection {
                        anchor: 0,
                        caret: e.text.len(),
                    };
                    e.text.len()
                }
                TextCommand::Backspace | TextCommand::Delete if input.editable => {
                    let range = if !selection.is_empty() {
                        selection
                    } else if command == TextCommand::Backspace {
                        previous..caret
                    } else {
                        caret..next
                    };
                    e.replace("", range);
                    changed = true;
                    e.selection.caret
                }
                _ => return Ok(false),
            };
            if command != TextCommand::SelectAll {
                e.selection.caret = floor(&e.text, target);
                if !extend {
                    e.selection.anchor = e.selection.caret;
                }
            }
        }
        if changed {
            self.publish_text(input.id);
        }
        self.redraw();
        Ok(true)
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
            (l.range_rects)(range)
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
