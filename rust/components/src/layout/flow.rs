//! Width-dependent measurement and a shared flex plan for measurement/placement.
use super::*;

pub(super) struct Line {
    pub items: Vec<usize>,
    pub sizes: Vec<f64>,
    pub free: f64,
}
impl Tree<'_> {
    pub(super) fn lines(&self, i: usize, content: Size, nodes: &[Node]) -> Vec<Line> {
        let n = &nodes[i];
        let horizontal = horizontal(n.kind);
        let limit = if horizontal {
            content.width
        } else {
            content.height
        };
        let gap = if n.kind == Kind::Fragment {
            0.
        } else {
            n.style.gap
        };
        let mut groups: Vec<Vec<usize>> = vec![vec![]];
        let mut used = 0.;
        for child in self
            .children
            .get(&Some(n.id))
            .into_iter()
            .flatten()
            .copied()
            .filter(|c| nodes[*c].style.position != Position::Absolute)
        {
            let s = nodes[child].style;
            let desired = self.desired[&nodes[child].id];
            let basis = s
                .basis
                .resolve(limit)
                .or_else(|| {
                    if horizontal {
                        s.width.resolve(limit)
                    } else {
                        s.height.resolve(limit)
                    }
                })
                .unwrap_or(if horizontal {
                    desired.width
                } else {
                    desired.height
                });
            let basis = basis.clamp(
                if horizontal {
                    s.min_width
                } else {
                    s.min_height
                },
                if horizontal {
                    s.max_width
                } else {
                    s.max_height
                },
            );
            let outer = basis + s.margin.main(horizontal);
            let current = groups.last().unwrap();
            if horizontal && n.style.wrap && !current.is_empty() && used + gap + outer > limit {
                groups.push(vec![]);
                used = 0.;
            }
            if !groups.last().unwrap().is_empty() {
                used += gap;
            }
            used += outer;
            groups.last_mut().unwrap().push(child);
        }
        groups
            .into_iter()
            .filter(|line| !line.is_empty())
            .map(|items| {
                let natural: Vec<_> = items
                    .iter()
                    .map(|c| {
                        let s = nodes[*c].style;
                        let desired = self.desired[&nodes[*c].id];
                        s.basis
                            .resolve(limit)
                            .or_else(|| {
                                if horizontal {
                                    s.width.resolve(limit)
                                } else {
                                    s.height.resolve(limit)
                                }
                            })
                            .unwrap_or(if horizontal {
                                desired.width
                            } else {
                                desired.height
                            })
                    })
                    .collect();
                let available = (limit
                    - items
                        .iter()
                        .map(|c| nodes[*c].style.margin.main(horizontal))
                        .sum::<f64>()
                    - gap * items.len().saturating_sub(1) as f64)
                    .max(0.);
                // Natural measurement may have an indefinite block height.
                let mut sizes = if available.is_finite() {
                    flex_sizes(
                        &items,
                        &natural,
                        available,
                        horizontal,
                        !matches!(n.kind, Kind::Block | Kind::Fragment | Kind::Scroll),
                        nodes,
                    )
                } else {
                    natural
                };
                if n.kind == Kind::Fragment
                    && items.len() == 1
                    && nodes[items[0]].style.height == Length::Auto
                    && available.is_finite()
                {
                    sizes[0] = available.clamp(
                        nodes[items[0]].style.min_height,
                        nodes[items[0]].style.max_height,
                    );
                }
                let free = if available.is_finite() {
                    (available - sizes.iter().sum::<f64>()).max(0.)
                } else {
                    0.
                };
                Line { items, sizes, free }
            })
            .collect()
    }
    pub(super) fn child_width(
        &self,
        child: usize,
        parent: crate::Style,
        available: f64,
        nodes: &[Node],
    ) -> f64 {
        let s = nodes[child].style;
        let preferred = self.desired[&nodes[child].id].width;
        s.width
            .resolve(available)
            .unwrap_or_else(|| {
                if s.align_self.unwrap_or(parent.align) == Align::Stretch {
                    (available - s.margin.horizontal()).max(0.)
                } else {
                    preferred
                }
            })
            .clamp(s.min_width, s.max_width)
    }
    /// Remeasure height at the actual allocated width without losing intrinsic flex basis.
    pub(super) fn reflow(
        &mut self,
        i: usize,
        width: f64,
        height: Option<f64>,
        nodes: &mut [Node],
    ) -> Result<(), Error> {
        let n = nodes[i].clone();
        let s = n.style;
        if self.reflowed.get(&n.id) == Some(&(width, height)) {
            return Ok(());
        }
        let inner = (width - s.padding.horizontal()).max(0.);
        let inner_height = height.map(|h| (h - s.padding.vertical()).max(0.));
        let mut paragraph = self.layouts[&n.id].clone();
        let mut natural;
        let mut baseline = s.padding.top;
        if n.kind == Kind::Split {
            natural = self.reflow_split(i, inner, inner_height, nodes)?;
            let children = &self.children[&Some(n.id)];
            let first = children[if n.split_collapsed == crate::SplitPane::First {
                2
            } else {
                0
            }];
            baseline = s.padding.top + self.baselines[&nodes[first].id];
        } else if n.kind == Kind::RichText {
            let children = self.children.get(&Some(n.id)).cloned().unwrap_or_default();
            for child in &children {
                let size = self.desired[&nodes[*child].id];
                self.reflow(*child, size.width, None, nodes)?;
            }
            let spans = self.rich_runs(&n, &children, nodes);
            paragraph = self.measure.paragraph(
                &spans,
                n.text_style,
                if n.text_style.constrained() {
                    inner
                } else {
                    f64::INFINITY
                },
            )?;
            natural = paragraph.size.height + s.padding.vertical();
            baseline = s.padding.top + paragraph.baseline;
            nodes[i].spans = Some(spans);
        } else if container(n.kind) {
            if header(n.kind) && !n.text.is_empty() {
                paragraph = self.measure.text(
                    &n.text,
                    n.text_style,
                    if n.text_style.constrained() {
                        inner
                    } else {
                        f64::INFINITY
                    },
                );
            }
            let heading = if header(n.kind) && !n.text.is_empty() {
                paragraph.size.height + s.gap
            } else {
                0.
            };
            let available_height = inner_height.map(|h| (h - heading).max(0.));
            let children = self.children.get(&Some(n.id)).cloned().unwrap_or_default();
            if horizontal(n.kind) {
                let lines = self.lines(
                    i,
                    Size::new(inner, available_height.unwrap_or(f64::INFINITY)),
                    nodes,
                );
                let mut cross = 0.;
                let mut first_baseline = None;
                for (index, line) in lines.iter().enumerate() {
                    for (j, child) in line.items.iter().enumerate() {
                        let cs = nodes[*child].style;
                        let child_height =
                            cs.height.resolve(available_height.unwrap_or(f64::INFINITY));
                        self.reflow(*child, line.sizes[j], child_height, nodes)?;
                    }
                    let (extent, ascent) = self.line_metrics(&line.items, s.align, nodes);
                    if index == 0 {
                        first_baseline = ascent.or_else(|| {
                            line.items
                                .first()
                                .map(|c| self.baselines[&nodes[*c].id] + nodes[*c].style.margin.top)
                        });
                    }
                    if index > 0 {
                        cross += s.gap;
                    }
                    cross += extent;
                }
                natural = cross + heading + s.padding.vertical();
                baseline += heading + first_baseline.unwrap_or(0.);
            } else if n.kind == Kind::Layer {
                let mut extent: f64 = 0.;
                for child in children
                    .iter()
                    .copied()
                    .filter(|c| nodes[*c].style.position != Position::Absolute)
                    .collect::<Vec<_>>()
                {
                    let cs = nodes[child].style;
                    let child_width = self.child_width(child, s, inner, nodes);
                    let child_height = cs.height.resolve(available_height.unwrap_or(f64::INFINITY));
                    self.reflow(child, child_width, child_height, nodes)?;
                    extent =
                        extent.max(self.desired[&nodes[child].id].height + cs.margin.vertical());
                }
                natural = extent + s.padding.vertical();
                if let Some(child) = children
                    .iter()
                    .find(|c| nodes[**c].style.position != Position::Absolute)
                {
                    baseline += nodes[*child].style.margin.top + self.baselines[&nodes[*child].id];
                }
            } else {
                let mut extent = 0.;
                let mut first = true;
                let flowing: Vec<_> = children
                    .iter()
                    .copied()
                    .filter(|c| nodes[*c].style.position != Position::Absolute)
                    .collect();
                for (count, child) in flowing.iter().copied().enumerate() {
                    let cs = nodes[child].style;
                    let child_width = if n.kind == Kind::Scroll
                        && n.scroll_axes.x()
                        && cs.width == Length::Auto
                    {
                        self.desired[&nodes[child].id]
                            .width
                            .max((inner - cs.margin.horizontal()).max(0.))
                    } else {
                        self.child_width(child, s, inner, nodes)
                    };
                    let child_height = if n.kind == Kind::Fragment
                        && flowing.len() == 1
                        && cs.height == Length::Auto
                    {
                        available_height
                    } else {
                        cs.height.resolve(available_height.unwrap_or(f64::INFINITY))
                    };
                    self.reflow(child, child_width, child_height, nodes)?;
                    if first {
                        baseline += heading + cs.margin.top + self.baselines[&nodes[child].id];
                        first = false;
                    }
                    if count > 0 && n.kind != Kind::Fragment {
                        extent += s.gap;
                    }
                    extent += self.desired[&nodes[child].id].height + cs.margin.vertical();
                }
                natural = extent + heading + s.padding.vertical();
                if n.kind == Kind::Scroll && s.height == Length::Auto {
                    natural = natural.min(height.unwrap_or(240.));
                }
            }
            // Absolute content does not size its parent, but still needs width reflow.
            for child in children
                .into_iter()
                .filter(|c| nodes[*c].style.position == Position::Absolute)
                .collect::<Vec<_>>()
            {
                let cs = nodes[child].style;
                let left = cs.left.resolve(inner);
                let right = cs.right.resolve(inner);
                let child_width = if cs.width == Length::Auto
                    && let (Some(l), Some(r)) = (left, right)
                {
                    (inner - l - r).max(0.)
                } else {
                    cs.width
                        .resolve(inner)
                        .unwrap_or(self.desired[&nodes[child].id].width)
                }
                .clamp(cs.min_width, cs.max_width);
                self.reflow(
                    child,
                    child_width,
                    cs.height.resolve(available_height.unwrap_or(f64::INFINITY)),
                    nodes,
                )?;
            }
            if header(n.kind) && !n.text.is_empty() {
                baseline = s.padding.top + paragraph.baseline;
            }
        } else {
            let extra = match n.kind {
                Kind::Button | Kind::Toggle | Kind::Segment => 28.,
                Kind::CheckBox | Kind::Radio => 30.,
                Kind::Switch => 54.,
                _ => 0.,
            };
            paragraph = self.measure.text(
                &n.text,
                n.text_style,
                if n.text_style.constrained() {
                    (inner - extra).max(0.)
                } else {
                    f64::INFINITY
                },
            );
            let control_height = if n.density == crate::Density::Standard {
                self.theme.control_height
            } else {
                n.density.control_height()
            };
            let text_height = paragraph.size.height;
            natural = match n.kind {
                Kind::Label => text_height,
                Kind::Button | Kind::Toggle | Kind::Segment => {
                    control_height.max(text_height + 10.)
                }
                Kind::CheckBox | Kind::Radio | Kind::Switch => control_height.max(text_height + 6.),
                _ => self.desired[&n.id].height - s.padding.vertical(),
            } + s.padding.vertical();
            let final_height = if s.height != Length::Auto {
                height.unwrap_or(natural)
            } else {
                natural
            }
            .clamp(s.min_height, s.max_height);
            baseline = if matches!(
                n.kind,
                Kind::Label
                    | Kind::Button
                    | Kind::Toggle
                    | Kind::Segment
                    | Kind::TextField
                    | Kind::CheckBox
                    | Kind::Radio
                    | Kind::Switch
            ) {
                s.padding.top
                    + (final_height - s.padding.vertical() - text_height).max(0.) / 2.
                    + paragraph.baseline
            } else {
                final_height
            };
        }
        if s.height != Length::Auto
            && let Some(h) = height
        {
            natural = h;
        }
        self.desired.get_mut(&n.id).unwrap().height = natural.clamp(s.min_height, s.max_height);
        self.baselines.insert(n.id, baseline);
        self.layouts.insert(n.id, paragraph);
        self.reflowed.insert(n.id, (width, height));
        Ok(())
    }
    pub(super) fn rich_runs(
        &self,
        n: &Node,
        children: &[usize],
        nodes: &[Node],
    ) -> Vec<crate::ParagraphSpan> {
        n.rich
            .as_ref()
            .unwrap()
            .iter()
            .filter_map(|run| match run {
                crate::rich_text::Run::Text(span) => Some(crate::ParagraphSpan::Text {
                    text: if n.style.uppercase
                        || n.style.label_style == Some(crate::LabelStyle::Section)
                    {
                        span.text.to_uppercase()
                    } else {
                        span.text.clone()
                    },
                    style: span.style.resolve(n.text_style),
                }),
                crate::rich_text::Run::Child(slot) => {
                    let child = children.iter().find(|c| nodes[**c].slot == *slot)?;
                    let size = self.desired[&nodes[*child].id];
                    let margin = nodes[*child].style.margin;
                    Some(crate::ParagraphSpan::Box {
                        size: Size::new(
                            size.width + margin.horizontal(),
                            size.height + margin.vertical(),
                        ),
                        baseline: self.baselines[&nodes[*child].id] + margin.top,
                    })
                }
            })
            .collect()
    }
    pub(super) fn line_metrics(
        &self,
        line: &[usize],
        align: Align,
        nodes: &[Node],
    ) -> (f64, Option<f64>) {
        let mut extent: f64 = 0.;
        let mut ascent: f64 = 0.;
        let mut descent: f64 = 0.;
        let mut has_baseline = false;
        for child in line {
            let n = &nodes[*child];
            let height = self.desired[&n.id].height;
            extent = extent.max(height + n.style.margin.vertical());
            if n.style.align_self.unwrap_or(align) == Align::Baseline {
                has_baseline = true;
                let base = self.baselines[&n.id];
                ascent = ascent.max(base + n.style.margin.top);
                descent = descent.max(height - base + n.style.margin.bottom);
            }
        }
        (extent.max(ascent + descent), has_baseline.then_some(ascent))
    }
    pub(super) fn hide_subtree(&self, i: usize, nodes: &mut [Node]) {
        nodes[i].layout_hidden = true;
        for child in self.children.get(&Some(nodes[i].id)).into_iter().flatten() {
            self.hide_subtree(*child, nodes);
        }
    }
    pub(super) fn translate(&self, i: usize, dy: f64, parent_clip: Rect, nodes: &mut [Node]) {
        nodes[i].bounds.y += dy;
        nodes[i].text_bounds.y += dy;
        nodes[i].clip = parent_clip.intersection(nodes[i].bounds);
        for child in self.children.get(&Some(nodes[i].id)).into_iter().flatten() {
            self.translate(*child, dy, nodes[i].clip, nodes);
        }
    }
}
