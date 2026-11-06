use super::*;
use crate::{CaretAffinity, Point, TextLayout};
impl TextBuffer {
    /// Visual movement uses the same shaped geometry as painting and hit testing.
    /// Other commands use the buffer's logical Unicode boundaries.
    pub fn navigate(
        &mut self,
        command: TextCommand,
        extend: bool,
        layout: &TextLayout,
        page_height: f64,
    ) -> bool {
        use TextCommand::*;
        if !matches!(
            command,
            Up | Down
                | PageUp
                | PageDown
                | LineStart
                | LineEnd
                | DeleteLineBackward
                | DeleteLineForward
        ) || layout.lines.is_empty()
        {
            return self.command(command, extend);
        }
        self.unmark(true);
        let caret = self.selection.caret;
        let index = if self.affinity == CaretAffinity::Upstream {
            layout.lines.iter().position(|l| l.range.end == caret)
        } else {
            None
        }
        .unwrap_or_else(|| {
            layout
                .lines
                .iter()
                .rposition(|l| l.range.start <= caret)
                .unwrap_or(0)
        });
        let line = &layout.lines[index];
        if matches!(command, DeleteLineBackward | DeleteLineForward) {
            let range = if !self.selection.range().is_empty() {
                self.selection.range()
            } else if command == DeleteLineBackward {
                line.range.start..caret
            } else {
                caret..line.range.end
            };
            return self.replace(range, "");
        }
        let vertical = matches!(command, Up | Down | PageUp | PageDown);
        let x = self
            .preferred_x
            .unwrap_or_else(|| layout.caret_rect(caret, self.affinity).x);
        let (target, affinity) = match command {
            LineStart => (line.range.start, CaretAffinity::Downstream),
            LineEnd => (line.range.end, CaretAffinity::Upstream),
            _ => {
                let up = matches!(command, Up | PageUp);
                let target_index = if matches!(command, Up | Down) {
                    if up {
                        index.saturating_sub(1)
                    } else {
                        (index + 1).min(layout.lines.len() - 1)
                    }
                } else {
                    let distance = if page_height.is_finite() {
                        (page_height * 0.9).max(line.bounds.height)
                    } else {
                        line.bounds.height
                    };
                    let y = line.bounds.y + if up { -distance } else { distance };
                    layout
                        .lines
                        .iter()
                        .enumerate()
                        .min_by(|(_, a), (_, b)| {
                            (a.bounds.y - y).abs().total_cmp(&(b.bounds.y - y).abs())
                        })
                        .map_or(index, |(i, _)| i)
                };
                let next = &layout.lines[target_index];
                let target = if target_index == index {
                    if up { 0 } else { self.text.len() }
                } else {
                    (layout.hit_test)(Point {
                        x,
                        y: next.bounds.y + next.bounds.height / 2.,
                    })
                    .clamp(next.range.start, next.range.end)
                };
                (
                    target,
                    if target == next.range.end {
                        CaretAffinity::Upstream
                    } else {
                        CaretAffinity::Downstream
                    },
                )
            }
        };
        self.select(Selection {
            anchor: if extend {
                self.selection.anchor
            } else {
                target
            },
            caret: target,
        });
        self.affinity = affinity;
        if vertical {
            self.preferred_x = Some(x);
        }
        true
    }
}
