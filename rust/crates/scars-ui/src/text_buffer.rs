//! Reusable UTF-8 editor state. Selection and edits snap to extended grapheme boundaries.
use crate::text::{grapheme_boundaries, word_boundaries};
use std::{
    ops::Range,
    time::{Duration, Instant},
};
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
    LineStart,
    LineEnd,
    ParagraphStart,
    ParagraphEnd,
    Backspace,
    Delete,
    DeleteWordBackward,
    DeleteWordForward,
    DeleteLineBackward,
    DeleteLineForward,
    DeleteParagraphBackward,
    DeleteParagraphForward,
    Transpose,
    SelectAll,
    Enter,
    Escape,
    Tab,
    BackTab,
    Undo,
    Redo,
    Up,
    Down,
    PageUp,
    PageDown,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TextOptions {
    pub multiline: bool,
    pub read_only: bool,
    /// Zero means unlimited; counts extended grapheme clusters, not bytes/code points.
    pub max_length: usize,
    pub undo: bool,
    pub history_limit: usize,
    pub typing_interval: Duration,
}
impl Default for TextOptions {
    fn default() -> Self {
        Self {
            multiline: true,
            read_only: false,
            max_length: 0,
            undo: true,
            history_limit: 100,
            typing_interval: Duration::from_millis(750),
        }
    }
}
#[derive(Clone)]
struct Snapshot {
    text: String,
    selection: Selection,
    affinity: crate::CaretAffinity,
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum Group {
    Typing,
    Backspace,
    Delete,
}
/// Owns committed text, composition, selection and bounded history. No window is required.
#[derive(Clone, Default)]
pub struct TextBuffer {
    pub(crate) text: String,
    pub(crate) selection: Selection,
    pub(crate) marked: Option<Range<usize>>,
    base: Option<Snapshot>,
    undo: Vec<Snapshot>,
    redo: Vec<Snapshot>,
    options: TextOptions,
    group: Option<(Group, Instant, usize)>,
    pub(crate) preferred_x: Option<f64>,
    pub(crate) affinity: crate::CaretAffinity,
}
pub(crate) fn floor(text: &str, index: usize) -> usize {
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
mod navigation;
impl TextBuffer {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            ..Self::default()
        }
    }
    pub fn text(&self) -> &str {
        &self.text
    }
    pub fn committed_text(&self) -> &str {
        self.base.as_ref().map_or(&self.text, |base| &base.text)
    }
    pub fn affinity(&self) -> crate::CaretAffinity {
        self.affinity
    }
    pub fn selection(&self) -> Selection {
        self.selection
    }
    pub fn word_range(&self, index: usize) -> Range<usize> {
        let index = if index >= self.text.len() && index > 0 {
            floor(&self.text, index - 1)
        } else {
            floor(&self.text, index)
        };
        let boundaries = word_boundaries(&self.text);
        boundaries
            .windows(2)
            .find(|b| b[0] <= index && index < b[1])
            .map_or(index..index, |b| b[0]..b[1])
    }
    pub fn marked_range(&self) -> Option<Range<usize>> {
        self.marked.clone()
    }
    pub fn options(&self) -> TextOptions {
        self.options
    }
    pub fn set_options(&mut self, options: TextOptions) {
        if options.read_only && !self.options.read_only {
            self.unmark(false);
        }
        self.options = options;
        if !options.undo {
            self.clear_history();
        }
        self.trim_history();
    }
    pub fn select(&mut self, selection: Selection) {
        self.affinity = crate::CaretAffinity::Downstream;
        self.selection = Selection {
            anchor: floor(&self.text, selection.anchor),
            caret: floor(&self.text, selection.caret),
        };
        self.break_group();
    }
    /// External replacement always resets composition/history, even when text is unchanged.
    pub fn set_text(&mut self, text: impl Into<String>) {
        self.text = text.into();
        self.base = None;
        self.marked = None;
        self.select(self.selection);
        self.clear_history();
    }
    pub fn clear_history(&mut self) {
        self.undo.clear();
        self.redo.clear();
        self.break_group();
    }
    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty() && self.options.undo && !self.options.read_only
    }
    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty() && self.options.undo && !self.options.read_only
    }
    pub fn history_len(&self) -> (usize, usize) {
        (self.undo.len(), self.redo.len())
    }
    fn snapshot(&self) -> Snapshot {
        Snapshot {
            text: self.text.clone(),
            selection: self.selection,
            affinity: self.affinity,
        }
    }
    fn restore(&mut self, s: Snapshot) {
        self.text = s.text;
        self.selection = s.selection;
        self.affinity = s.affinity;
        self.preferred_x = None;
    }
    fn trim_history(&mut self) {
        let n = self.options.history_limit;
        if self.undo.len() > n {
            self.undo.drain(..self.undo.len() - n);
        }
        if self.redo.len() > n {
            self.redo.drain(..self.redo.len() - n);
        }
    }
    fn record(&mut self, snapshot: Snapshot) {
        if self.options.undo && self.options.history_limit > 0 {
            self.undo.push(snapshot);
            self.trim_history();
        }
        self.redo.clear();
    }
    pub(crate) fn break_group(&mut self) {
        self.group = None;
        self.preferred_x = None;
    }
    fn range(&self, range: Range<usize>) -> Range<usize> {
        let a = range.start.min(range.end).min(self.text.len());
        let b = range.start.max(range.end).min(self.text.len());
        floor(&self.text, a)..if a == b {
            floor(&self.text, b)
        } else {
            ceil(&self.text, b)
        }
    }
    fn clean(&self, value: &str, range: Range<usize>) -> String {
        let value: String = value
            .chars()
            .filter(|c| *c != '\0' && *c != '\r' && (self.options.multiline || *c != '\n'))
            .collect();
        let max = self.options.max_length;
        if max == 0 {
            return value;
        }
        let count = |text: &str| grapheme_boundaries(text).len().saturating_sub(1);
        let candidate = |value: &str| {
            format!(
                "{}{}{}",
                &self.text[..range.start],
                value,
                &self.text[range.end..]
            )
        };
        if count(&candidate(&value)) <= max {
            return value;
        }
        let rest = candidate("");
        let bounds = grapheme_boundaries(&value);
        let mut i = max
            .saturating_sub(count(&rest))
            .saturating_add(2)
            .min(bounds.len() - 1);
        while i > 0 && count(&candidate(&value[..bounds[i]])) > max {
            i -= 1;
        }
        value[..bounds[i]].to_owned()
    }
    fn replace_raw(&mut self, range: Range<usize>, value: &str) -> Range<usize> {
        self.affinity = crate::CaretAffinity::Downstream;
        self.text.replace_range(range.clone(), value);
        let end = range.start + value.len();
        self.selection = Selection {
            anchor: ceil(&self.text, end),
            caret: ceil(&self.text, end),
        };
        self.preferred_x = None;
        range.start..end
    }
    fn edit(
        &mut self,
        range: Range<usize>,
        value: &str,
        group: Option<Group>,
        now: Instant,
    ) -> bool {
        if self.options.read_only {
            return false;
        }
        if self.marked.is_some() {
            self.unmark(true);
        }
        let range = self.range(range);
        let value = self.clean(value, range.clone());
        if self.text[range.clone()] == value {
            self.break_group();
            return false;
        }
        let coalesce = group.is_some()
            && self.selection.range().is_empty()
            && self.group.is_some_and(|(kind, at, caret)| {
                Some(kind) == group
                    && now.saturating_duration_since(at) <= self.options.typing_interval
                    && caret == self.selection.caret
            });
        if !coalesce {
            self.record(self.snapshot());
        } else {
            self.redo.clear();
        }
        self.replace_raw(range, &value);
        self.group = group.map(|kind| (kind, now, self.selection.caret));
        true
    }
    /// Discrete edit (paste, replacement, programmatic action); never coalesces typing.
    pub fn replace(&mut self, range: Range<usize>, value: &str) -> bool {
        if self.options.read_only {
            return false;
        }
        if self.marked.is_some() {
            let range = self.range(range);
            let value = self.clean(value, range.clone());
            self.replace_raw(range, &value);
            return self.unmark(true);
        }
        self.edit(range, value, None, Instant::now())
    }
    pub fn insert(&mut self, value: &str) -> bool {
        self.insert_at(value, Instant::now())
    }
    /// Injectable time keeps typing coalescing deterministic in tests and custom runtimes.
    pub fn insert_at(&mut self, value: &str, now: Instant) -> bool {
        if self.options.read_only {
            return false;
        }
        if self.marked.is_some() {
            let range = self.marked.clone().unwrap();
            let value = self.clean(value, range.clone());
            self.replace_raw(range, &value);
            return self.unmark(true);
        }
        let group = (!value.contains(['\n', '\r']) && self.selection.range().is_empty())
            .then_some(Group::Typing);
        self.edit(self.selection.range(), value, group, now)
    }
    pub fn mark(
        &mut self,
        value: &str,
        selected: Range<usize>,
        replacement: Option<Range<usize>>,
    ) -> bool {
        if self.options.read_only || value.contains(['\n', '\r', '\0']) {
            return false;
        }
        if self.base.is_none() {
            self.base = Some(self.snapshot());
        }
        self.break_group();
        let range = self.range(
            replacement
                .or_else(|| self.marked.clone())
                .unwrap_or_else(|| self.selection.range()),
        );
        let value = self.clean(value, range.clone());
        let range = self.replace_raw(range, &value);
        self.selection = Selection {
            anchor: floor(&self.text, range.start + floor(&value, selected.start)),
            caret: floor(&self.text, range.start + floor(&value, selected.end)),
        };
        self.marked = Some(range);
        true
    }
    pub fn unmark(&mut self, commit: bool) -> bool {
        let Some(base) = self.base.take() else {
            return false;
        };
        let changed = base.text != self.text;
        if !commit {
            self.restore(base);
        } else if changed {
            self.record(base);
        }
        self.marked = None;
        self.break_group();
        changed
    }
    pub fn undo(&mut self) -> bool {
        self.history(false)
    }
    pub fn redo(&mut self) -> bool {
        self.history(true)
    }
    fn history(&mut self, redo: bool) -> bool {
        if self.options.read_only || !self.options.undo {
            return false;
        }
        self.unmark(false);
        self.break_group();
        let prior = if redo {
            self.redo.pop()
        } else {
            self.undo.pop()
        };
        let Some(prior) = prior else { return false };
        let current = self.snapshot();
        if redo {
            self.undo.push(current);
        } else {
            self.redo.push(current);
        }
        self.restore(prior);
        true
    }
    pub fn command(&mut self, command: TextCommand, extend: bool) -> bool {
        use TextCommand::*;
        if command == Undo {
            return self.undo();
        }
        if command == Redo {
            return self.redo();
        }
        if command == Escape {
            return self.unmark(false);
        }
        if command == Enter {
            return self.options.multiline && self.insert("\n");
        }
        self.unmark(true);
        let caret = self.selection.caret;
        let selection = self.selection.range();
        let boundaries = grapheme_boundaries(&self.text);
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
            .unwrap_or(self.text.len());
        let words = word_boundaries(&self.text);
        let word_left = || {
            words
                .windows(2)
                .rev()
                .find(|b| b[0] < caret && !self.text[b[0]..caret.min(b[1])].trim().is_empty())
                .map_or(0, |b| b[0])
        };
        let word_right = || {
            words
                .windows(2)
                .find(|b| b[1] > caret && !self.text[caret.max(b[0])..b[1]].trim().is_empty())
                .map_or(self.text.len(), |b| b[1])
        };
        let line_start = self.text[..caret].rfind('\n').map_or(0, |i| i + 1);
        let line_end = self.text[caret..]
            .find('\n')
            .map_or(self.text.len(), |i| caret + i);
        let paragraph_start = if line_start == caret && caret > 0 {
            self.text[..caret - 1].rfind('\n').map_or(0, |i| i + 1)
        } else {
            line_start
        };
        let paragraph_end = if line_end == caret && caret < self.text.len() {
            self.text[caret + 1..]
                .find('\n')
                .map_or(self.text.len(), |i| caret + 1 + i)
        } else {
            line_end
        };
        let target = match command {
            Start => 0,
            End => self.text.len(),
            LineStart => line_start,
            LineEnd => line_end,
            ParagraphStart => paragraph_start,
            ParagraphEnd => paragraph_end,
            Left if !extend && !selection.is_empty() => selection.start,
            Right if !extend && !selection.is_empty() => selection.end,
            Left => previous,
            Right => next,
            WordLeft => word_left(),
            WordRight => word_right(),
            SelectAll => {
                self.select(Selection {
                    anchor: 0,
                    caret: self.text.len(),
                });
                return true;
            }
            Backspace
            | Delete
            | DeleteWordBackward
            | DeleteWordForward
            | DeleteLineBackward
            | DeleteLineForward
            | DeleteParagraphBackward
            | DeleteParagraphForward => {
                let range = if !selection.is_empty() {
                    selection
                } else {
                    match command {
                        Backspace => previous..caret,
                        Delete => caret..next,
                        DeleteWordBackward => word_left()..caret,
                        DeleteWordForward => caret..word_right(),
                        DeleteLineBackward => line_start..caret,
                        DeleteLineForward => caret..line_end,
                        DeleteParagraphBackward => line_start..caret,
                        _ => caret..if line_end == caret { next } else { line_end },
                    }
                };
                let group = match command {
                    Backspace => Some(Group::Backspace),
                    Delete => Some(Group::Delete),
                    _ => None,
                };
                return self.edit(range, "", group, Instant::now());
            }
            Transpose => {
                if caret == 0 || !selection.is_empty() {
                    return false;
                }
                let end = if caret == self.text.len() {
                    caret
                } else {
                    next
                };
                let mid = if caret == self.text.len() {
                    previous
                } else {
                    caret
                };
                let start = boundaries
                    .iter()
                    .copied()
                    .take_while(|b| *b < mid)
                    .last()
                    .unwrap_or(0);
                if start == mid {
                    return false;
                }
                let value = format!("{}{}", &self.text[mid..end], &self.text[start..mid]);
                return self.replace(start..end, &value);
            }
            _ => return false,
        };
        self.select(Selection {
            anchor: if extend {
                self.selection.anchor
            } else {
                target
            },
            caret: target,
        });
        true
    }
}
