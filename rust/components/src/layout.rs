use crate::{
    Align, Error, Host, Justify, Kind, Length, Measure, Node, NodeId, Position, Rect, Size,
    TextLayout, TextStyle,
};
use std::collections::HashMap;

impl Host {
    pub fn layout(&self, size: Size, measure: &dyn Measure) -> Result<(), Error> {
        if !size.width.is_finite()
            || !size.height.is_finite()
            || size.width < 0.
            || size.height < 0.
        {
            return Err(Error("Invalid viewport".into()));
        }
        self.refresh()?;
        self.0.viewport.set(size);
        let theme = self.theme();
        let mut mounted = self.0.mounted.borrow_mut();
        let mut tree = Tree {
            children: HashMap::new(),
            desired: HashMap::new(),
            layouts: HashMap::new(),
            measure,
            theme,
            scroll: self.0.scroll.borrow().clone(),
        };
        for (i, n) in mounted.nodes.iter().enumerate() {
            tree.children.entry(n.parent).or_default().push(i);
        }
        let roots = tree.children.get(&None).cloned().unwrap_or_default();
        let text = TextStyle {
            size: theme.font_size,
            color: theme.text,
            bold: false,
            wrap: false,
        };
        for root in roots {
            tree.measure_node(root, size, text, &mut mounted.nodes)?;
            tree.arrange(
                root,
                Rect::new(0., 0., size.width, size.height),
                Rect::new(0., 0., size.width, size.height),
                &mut mounted.nodes,
            )?;
        }
        *self.0.scroll.borrow_mut() = tree.scroll;
        mounted.layouts = tree.layouts;
        let mut editors = self.0.editors.borrow_mut();
        let mut field_layouts = Vec::new();
        for node in &mut mounted.nodes {
            if let Some(editor) = editors.get_mut(&node.id) {
                let layout = crate::metrics::field_layout(
                    measure,
                    &editor.text,
                    node.secure,
                    node.text_style,
                    if node.multiline {
                        (node.bounds.width - 20.).max(1.)
                    } else {
                        f64::INFINITY
                    },
                );
                let caret = (layout.range_rects)(editor.selection.caret..editor.selection.caret)
                    .first()
                    .copied()
                    .unwrap_or_default();
                let available = (node.bounds.width - 20.).max(1.);
                if node.multiline {
                    editor.scroll = 0.;
                    let height = (node.bounds.height - 20.).max(1.);
                    if caret.y < editor.scroll_y {
                        editor.scroll_y = caret.y;
                    }
                    if caret.y + caret.height > editor.scroll_y + height {
                        editor.scroll_y = caret.y + caret.height - height;
                    }
                    editor.scroll_y = editor
                        .scroll_y
                        .clamp(0., (layout.size.height - height).max(0.));
                }
                if !node.multiline && caret.x < editor.scroll {
                    editor.scroll = caret.x;
                }
                if !node.multiline && caret.x + 2. > editor.scroll + available {
                    editor.scroll = caret.x + 2. - available;
                }
                editor.scroll = editor
                    .scroll
                    .clamp(0., (layout.size.width + 2. - available).max(0.));
                node.text_bounds = Rect::new(
                    node.bounds.x + 10. - editor.scroll,
                    if node.multiline {
                        node.bounds.y + 10. - editor.scroll_y
                    } else {
                        node.bounds.y + (node.bounds.height - layout.size.height) / 2.
                    },
                    layout.size.width,
                    layout.size.height,
                );
                field_layouts.push((node.id, layout));
            }
        }
        mounted.layouts.extend(field_layouts);
        drop(editors);
        drop(mounted);
        self.reconcile_modal()?;
        Ok(())
    }
    /// Scene order includes z-order while preserving each sibling subtree as a unit.
    pub fn scene(&self) -> Result<Vec<Node>, Error> {
        let nodes = self.snapshot()?;
        let mut children: HashMap<Option<NodeId>, Vec<usize>> = HashMap::new();
        for (i, n) in nodes.iter().enumerate() {
            children.entry(n.parent).or_default().push(i);
        }
        for list in children.values_mut() {
            list.sort_by_key(|i| nodes[*i].style.z_index);
        }
        fn visit(
            parent: Option<NodeId>,
            nodes: &[Node],
            children: &HashMap<Option<NodeId>, Vec<usize>>,
            out: &mut Vec<Node>,
        ) {
            if let Some(list) = children.get(&parent) {
                for i in list {
                    out.push(nodes[*i].clone());
                    visit(Some(nodes[*i].id), nodes, children, out);
                }
            }
        }
        let mut result = Vec::new();
        visit(None, &nodes, &children, &mut result);
        Ok(result)
    }
}
struct Tree<'a> {
    children: HashMap<Option<NodeId>, Vec<usize>>,
    desired: HashMap<NodeId, Size>,
    layouts: HashMap<NodeId, TextLayout>,
    measure: &'a dyn Measure,
    theme: crate::Theme,
    scroll: HashMap<NodeId, f64>,
}
fn horizontal(kind: Kind) -> bool {
    matches!(kind, Kind::Row | Kind::Inline | Kind::Segmented)
}
fn container(kind: Kind) -> bool {
    matches!(
        kind,
        Kind::Fragment
            | Kind::Column
            | Kind::Row
            | Kind::Block
            | Kind::Inline
            | Kind::Layer
            | Kind::Section
            | Kind::GroupBox
            | Kind::Panel
            | Kind::RadioGroup
            | Kind::Segmented
            | Kind::Scroll
            | Kind::Modal
    )
}
fn header(kind: Kind) -> bool {
    matches!(kind, Kind::Section | Kind::GroupBox | Kind::RadioGroup)
}
impl Tree<'_> {
    fn measure_node(
        &mut self,
        i: usize,
        available: Size,
        parent: TextStyle,
        nodes: &mut [Node],
    ) -> Result<Size, Error> {
        let n = nodes[i].clone();
        let s = n.style;
        let text = TextStyle {
            size: s.font_size.unwrap_or(parent.size),
            color: s.foreground.unwrap_or(parent.color),
            bold: s.bold || parent.bold,
            wrap: if n.kind == Kind::TextField {
                n.multiline
            } else {
                s.wrap
            },
        };
        nodes[i].text_style = text;
        let width = s
            .width
            .resolve(available.width)
            .unwrap_or(available.width)
            .min(s.max_width)
            .max(s.min_width);
        let inner = (width - 2. * s.padding).max(0.);
        let layout =
            self.measure
                .text(&n.text, text, if text.wrap { inner } else { f64::INFINITY });
        let mut size = if container(n.kind) {
            let children = self.children.get(&Some(n.id)).cloned().unwrap_or_default();
            let mut main: f64 = 0.;
            let mut cross: f64 = 0.;
            let mut count = 0;
            let mut line_main = 0.;
            let mut line_cross: f64 = 0.;
            for child in children {
                let child_size =
                    self.measure_node(child, Size::new(inner, f64::INFINITY), text, nodes)?;
                if nodes[child].style.position == Position::Absolute {
                    continue;
                }
                let m = nodes[child].style.margin * 2.;
                let (a, b) = if horizontal(n.kind) {
                    (child_size.width + m, child_size.height + m)
                } else {
                    (child_size.height + m, child_size.width + m)
                };
                if horizontal(n.kind) && s.wrap && line_main > 0. && line_main + s.gap + a > inner {
                    main = main.max(line_main);
                    cross += line_cross + s.gap;
                    line_main = 0.;
                    line_cross = 0.;
                }
                if horizontal(n.kind) && s.wrap {
                    if line_main > 0. {
                        line_main += s.gap;
                    }
                    line_main += a;
                    line_cross = line_cross.max(b);
                } else {
                    main += a;
                    if count > 0 {
                        main += if n.kind == Kind::Fragment { 0. } else { s.gap };
                    }
                    cross = cross.max(b);
                }
                count += 1;
            }
            if horizontal(n.kind) && s.wrap {
                main = main.max(line_main);
                cross += line_cross;
            }
            if n.kind == Kind::Layer {
                let sizes = self
                    .children
                    .get(&Some(n.id))
                    .into_iter()
                    .flatten()
                    .filter(|c| nodes[**c].style.position == Position::Flow)
                    .map(|c| self.desired[&nodes[*c].id]);
                let mut max = Size::default();
                for v in sizes {
                    max.width = max.width.max(v.width);
                    max.height = max.height.max(v.height);
                }
                max
            } else if horizontal(n.kind) {
                Size::new(main, cross)
            } else {
                Size::new(cross, main)
            }
        } else {
            match n.kind {
                Kind::Label => layout.size,
                Kind::TextField => Size::new(240., self.theme.control_height.max(text.size + 14.)),
                Kind::Button | Kind::Toggle | Kind::Segment => Size::new(
                    layout.size.width + 28.,
                    self.theme.control_height.max(layout.size.height + 10.),
                ),
                Kind::CheckBox | Kind::Radio => Size::new(
                    layout.size.width + 30.,
                    self.theme.control_height.max(layout.size.height + 6.),
                ),
                Kind::Switch => Size::new(
                    layout.size.width + 54.,
                    self.theme.control_height.max(layout.size.height + 6.),
                ),
                Kind::Slider => Size::new(180., self.theme.control_height),
                Kind::Progress => Size::new(180., 8.),
                Kind::Separator => Size::new(
                    if available.width.is_finite() {
                        available.width
                    } else {
                        1.
                    },
                    1.,
                ),
                Kind::Image => {
                    if let Some((image, _)) = &n.image {
                        self.measure.image(image)?
                    } else {
                        Size::default()
                    }
                }
                _ => Size::default(),
            }
        };
        if header(n.kind) && !n.text.is_empty() {
            size.height += layout.size.height + s.gap;
            size.width = size.width.max(layout.size.width);
        }
        size.width += 2. * s.padding;
        size.height += 2. * s.padding;
        size.width = s
            .width
            .resolve(available.width)
            .unwrap_or(size.width)
            .clamp(s.min_width, s.max_width);
        size.height = s
            .height
            .resolve(available.height)
            .unwrap_or(size.height)
            .clamp(s.min_height, s.max_height);
        if n.kind == Kind::Scroll && s.height == Length::Auto {
            size.height = size.height.min(if available.height.is_finite() {
                available.height
            } else {
                240.
            });
        }
        self.desired.insert(n.id, size);
        self.layouts.insert(n.id, layout);
        Ok(size)
    }
    fn arrange(
        &mut self,
        i: usize,
        bounds: Rect,
        clip: Rect,
        nodes: &mut [Node],
    ) -> Result<(), Error> {
        let n = nodes[i].clone();
        let s = n.style;
        nodes[i].bounds = bounds;
        nodes[i].clip = clip.intersection(bounds);
        let mut content = bounds.inset(s.padding);
        if header(n.kind) && !n.text.is_empty() {
            let height = self.layouts[&n.id].size.height;
            nodes[i].text_bounds = Rect::new(content.x, content.y, content.width, height);
            content.y += height + s.gap;
            content.height = (content.height - height - s.gap).max(0.);
        }
        if n.kind == Kind::Scroll {
            let children = self.children.get(&Some(n.id)).cloned().unwrap_or_default();
            let height = children
                .iter()
                .map(|c| self.desired[&nodes[*c].id].height + 2. * nodes[*c].style.margin)
                .sum::<f64>()
                + s.gap * children.len().saturating_sub(1) as f64;
            let max = (height - content.height).max(0.);
            nodes[i].scroll_max = max;
            let offset = self.scroll.entry(n.id).or_default();
            *offset = offset.clamp(0., max);
            content.y -= *offset;
            content.height = height.max(content.height);
        }
        if let Some(anchor) = &n.anchor
            && let Some(target) = nodes.iter().find(|n| n.key.as_ref() == Some(anchor))
        {
            let children = self.children.get(&Some(n.id)).cloned().unwrap_or_default();
            let width = children
                .iter()
                .map(|c| self.desired[&nodes[*c].id].width)
                .fold(0., f64::max);
            let height = children
                .iter()
                .map(|c| self.desired[&nodes[*c].id].height)
                .sum::<f64>();
            content.x = target
                .bounds
                .x
                .clamp(bounds.x, (bounds.x + bounds.width - width).max(bounds.x));
            content.y =
                if target.bounds.y + target.bounds.height + height > bounds.y + bounds.height {
                    (target.bounds.y - height).max(bounds.y)
                } else {
                    target.bounds.y + target.bounds.height + 6.
                };
            content.width = width.min(bounds.width);
            content.height = height.min(bounds.height);
        }
        let children = self.children.get(&Some(n.id)).cloned().unwrap_or_default();
        if container(n.kind) {
            let flowing: Vec<_> = children
                .iter()
                .copied()
                .filter(|c| nodes[*c].style.position == Position::Flow)
                .collect();
            if n.kind == Kind::Layer {
                for child in flowing {
                    let d = self.desired[&nodes[child].id];
                    let m = nodes[child].style.margin;
                    self.arrange(
                        child,
                        Rect::new(
                            content.x + m,
                            content.y + m,
                            nodes[child].style.width.resolve(content.width).unwrap_or(
                                if s.align == Align::Stretch {
                                    (content.width - 2. * m).max(0.)
                                } else {
                                    d.width
                                },
                            ),
                            nodes[child].style.height.resolve(content.height).unwrap_or(
                                if s.align == Align::Stretch {
                                    (content.height - 2. * m).max(0.)
                                } else {
                                    d.height
                                },
                            ),
                        ),
                        nodes[i].clip,
                        nodes,
                    )?;
                }
            } else {
                let horiz = horizontal(n.kind);
                let main_limit = if horiz { content.width } else { content.height };
                let cross_limit = if horiz { content.height } else { content.width };
                let gap = if n.kind == Kind::Fragment { 0. } else { s.gap };
                let mut lines: Vec<Vec<usize>> = vec![Vec::new()];
                let mut used = 0.;
                for child in flowing {
                    let d = self.desired[&nodes[child].id];
                    let m = nodes[child].style.margin * 2.;
                    let main = if horiz { d.width } else { d.height };
                    if horiz && s.wrap && used > 0. && used + gap + main + m > main_limit {
                        lines.push(Vec::new());
                        used = 0.;
                    }
                    if used > 0. {
                        used += gap;
                    }
                    used += main + m;
                    lines.last_mut().unwrap().push(child);
                }
                let mut line_offset = 0.;
                for line in lines {
                    if line.is_empty() {
                        continue;
                    }
                    let natural: Vec<f64> = line
                        .iter()
                        .map(|c| {
                            let d = self.desired[&nodes[*c].id];
                            if horiz { d.width } else { d.height }
                        })
                        .collect();
                    let margins: f64 = line.iter().map(|c| 2. * nodes[*c].style.margin).sum();
                    let available =
                        (main_limit - margins - gap * line.len().saturating_sub(1) as f64).max(0.);
                    let sum: f64 = natural.iter().sum();
                    let extra = available - sum;
                    let grow: f64 = line.iter().map(|c| nodes[*c].style.grow).sum();
                    let shrink: f64 = line
                        .iter()
                        .zip(&natural)
                        .map(|(c, n)| nodes[*c].style.shrink * n)
                        .sum();
                    let mut sizes = natural.clone();
                    if n.kind == Kind::Fragment
                        && line.len() == 1
                        && nodes[line[0]].style.height == Length::Auto
                    {
                        sizes[0] = available;
                    }
                    for (j, child) in line.iter().enumerate() {
                        let cs = nodes[*child].style;
                        let mut value = sizes[j];
                        if n.kind == Kind::Fragment && line.len() == 1 {
                            continue;
                        }
                        if extra > 0. && grow > 0. {
                            value += extra * cs.grow / grow;
                        } else if extra < 0.
                            && shrink > 0.
                            && !matches!(n.kind, Kind::Block | Kind::Fragment | Kind::Scroll)
                        {
                            value += extra * cs.shrink * natural[j] / shrink;
                        }
                        sizes[j] = value.clamp(
                            if horiz { cs.min_width } else { cs.min_height },
                            if horiz { cs.max_width } else { cs.max_height },
                        );
                    }
                    let allocated: f64 = sizes.iter().sum();
                    let remaining = (available - allocated).max(0.);
                    let mut cursor = match s.justify {
                        Justify::Center => remaining / 2.,
                        Justify::End => remaining,
                        _ => 0.,
                    };
                    let spacing = gap
                        + if s.justify == Justify::SpaceBetween && line.len() > 1 {
                            remaining / (line.len() - 1) as f64
                        } else {
                            0.
                        };
                    let line_cross = if horiz && s.wrap {
                        line.iter()
                            .map(|c| {
                                self.desired[&nodes[*c].id].height + 2. * nodes[*c].style.margin
                            })
                            .fold(0., f64::max)
                    } else {
                        cross_limit
                    };
                    let baseline = line
                        .iter()
                        .map(|c| {
                            self.layouts[&nodes[*c].id].baseline
                                + nodes[*c].style.padding
                                + nodes[*c].style.margin
                        })
                        .fold(0., f64::max);
                    for (j, child) in line.iter().enumerate() {
                        let cs = nodes[*child].style;
                        let desired = self.desired[&nodes[*child].id];
                        let natural_cross = if horiz { desired.height } else { desired.width };
                        let explicit = if horiz { cs.height } else { cs.width };
                        let cross = if s.align == Align::Stretch && explicit == Length::Auto {
                            (line_cross - 2. * cs.margin).max(0.)
                        } else {
                            natural_cross
                        };
                        let offset = match s.align {
                            Align::Center => (line_cross - cross) / 2.,
                            Align::End => line_cross - cross - cs.margin,
                            Align::Baseline if horiz => {
                                baseline - self.layouts[&nodes[*child].id].baseline - cs.padding
                            }
                            _ => cs.margin,
                        };
                        let r = if horiz {
                            Rect::new(
                                content.x + cursor + cs.margin,
                                content.y + line_offset + offset,
                                sizes[j],
                                cross,
                            )
                        } else {
                            Rect::new(
                                content.x + offset,
                                content.y + cursor + cs.margin,
                                cross,
                                sizes[j],
                            )
                        };
                        self.arrange(*child, r, nodes[i].clip, nodes)?;
                        cursor += sizes[j] + 2. * cs.margin + spacing;
                    }
                    line_offset += line_cross + gap;
                }
            }
            for child in children {
                let cs = nodes[child].style;
                if cs.position != Position::Absolute {
                    continue;
                }
                let d = self.desired[&nodes[child].id];
                let l = cs.left.resolve(content.width);
                let r = cs.right.resolve(content.width);
                let t = cs.top.resolve(content.height);
                let b = cs.bottom.resolve(content.height);
                let w = if cs.width == Length::Auto
                    && let (Some(l), Some(r)) = (l, r)
                {
                    (content.width - l - r).max(0.)
                } else {
                    d.width
                };
                let h = if cs.height == Length::Auto
                    && let (Some(t), Some(b)) = (t, b)
                {
                    (content.height - t - b).max(0.)
                } else {
                    d.height
                };
                self.arrange(
                    child,
                    Rect::new(
                        content.x + l.unwrap_or_else(|| r.map_or(0., |v| content.width - v - w)),
                        content.y + t.unwrap_or_else(|| b.map_or(0., |v| content.height - v - h)),
                        w,
                        h,
                    ),
                    nodes[i].clip,
                    nodes,
                )?;
            }
        } else {
            let layout = if n.kind == Kind::Label {
                self.measure.text(
                    &n.text,
                    n.text_style,
                    if s.wrap { content.width } else { f64::INFINITY },
                )
            } else {
                self.layouts[&n.id].clone()
            };
            let x = match n.kind {
                Kind::Button | Kind::Toggle | Kind::Segment => {
                    content.x + (content.width - layout.size.width) / 2.
                }
                Kind::CheckBox | Kind::Radio => content.x + 30.,
                Kind::Switch => content.x + 54.,
                _ => content.x,
            };
            nodes[i].text_bounds = Rect::new(
                x,
                content.y + (content.height - layout.size.height) / 2.,
                layout.size.width,
                layout.size.height,
            );
            self.layouts.insert(n.id, layout);
        }
        Ok(())
    }
}
