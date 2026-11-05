use crate::{
    Align, Error, Host, Justify, Kind, Length, Measure, Node, NodeId, Position, Rect, Size,
    TextLayout, TextStyle,
};
use std::collections::HashMap;

impl Host {
    pub fn layout(&self, size: Size, measure: &dyn Measure) -> Result<(), Error> {
        for _ in 0..4 {
            self.layout_pass(size, measure)?;
            if !self.0.observer.dirty.get() {
                return Ok(());
            }
        }
        Err(Error("Layout did not settle".into()))
    }
    fn layout_pass(&self, size: Size, measure: &dyn Measure) -> Result<(), Error> {
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
        {
            let m = self.0.mounted.borrow();
            let mut offsets = self.0.scroll.borrow_mut();
            for n in &m.nodes {
                if let Some(model) = &n.collection {
                    offsets.insert(n.id, model.offset());
                }
            }
        }
        let mut mounted = self.0.mounted.borrow_mut();
        let overrides = self.inspector().overrides.get();
        let descriptions = mounted.descriptions.clone();
        for node in &mut mounted.nodes {
            if let Some(style) = overrides.get(&node.id) {
                let mut description = (*descriptions[&node.id]).clone();
                description.style = *style;
                crate::host::validate(&description)?;
            }
            node.text = descriptions[&node.id].text.clone();
            node.style = overrides
                .get(&node.id)
                .copied()
                .unwrap_or(descriptions[&node.id].style);
        }
        let mut tree = Tree {
            children: HashMap::new(),
            desired: HashMap::new(),
            layouts: HashMap::new(),
            measure,
            theme,
            inherited: HashMap::new(),
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
            ..TextStyle::default()
        };
        for root in roots {
            tree.measure_node(root, size, text, &mut mounted.nodes)?;
            tree.arrange(
                root,
                Rect::new(0., 0., size.width, size.height),
                Rect::new(0., 0., size.width, size.height),
                size,
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
        let collections: Vec<_> = mounted
            .nodes
            .iter()
            .filter_map(|n| {
                n.collection.clone().map(|model| {
                    (
                        model,
                        self.0.scroll.borrow().get(&n.id).copied().unwrap_or(0.),
                        n.bounds.height,
                    )
                })
            })
            .collect();
        drop(editors);
        drop(mounted);
        for (model, offset, height) in collections {
            model.set_viewport(offset, height);
        }
        self.reconcile_modal()?;
        if let Some(id) = self.0.autofocus.take().filter(|id| self.in_scope(*id)) {
            self.focus(id)?;
        }
        if self.focused().is_none() {
            let id = {
                let m = self.0.mounted.borrow();
                m.nodes
                    .iter()
                    .rev()
                    .find(|n| {
                        n.enabled
                            && !n.hidden
                            && self.in_scope(n.id)
                            && m.descriptions[&n.id].autofocus
                            && crate::host::focusable(n.kind)
                    })
                    .map(|n| n.id)
            };
            if let Some(id) = id {
                self.focus(id)?;
            }
        }
        self.focus_pending_row()?;
        let inspector = self.inspector();
        if let Some(selected) = inspector.selected.get() {
            let node = self
                .0
                .mounted
                .borrow()
                .nodes
                .iter()
                .find(|n| n.id == selected.id)
                .cloned();
            inspector.selected.set(node.map(crate::Inspection::from));
        }
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
    inherited: HashMap<NodeId, (u32, crate::Density)>,
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
            | Kind::CollectionRow
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
        let (parent_accent, parent_density) = n
            .parent
            .and_then(|id| self.inherited.get(&id).copied())
            .unwrap_or((self.theme.accent, crate::Density::Standard));
        let accent = s.accent.unwrap_or(parent_accent);
        let density = s.density.unwrap_or(parent_density);
        self.inherited.insert(n.id, (accent, density));
        nodes[i].accent = accent;
        nodes[i].density = density;
        let control_height = if density == crate::Density::Standard {
            self.theme.control_height
        } else {
            density.control_height()
        };
        let mut text = parent;
        if let Some(label) = s.label_style {
            use crate::LabelStyle::*;
            match label {
                Standard => {}
                Heading => {
                    text.size = 28.;
                    text.bold = true;
                }
                Section => {
                    text.size = 13.;
                    text.bold = true;
                }
                Secondary => text.color = self.theme.secondary,
                Micro => {
                    text.size = 11.;
                    text.color = self.theme.secondary;
                }
                Status => {
                    text.color = accent;
                    text.bold = true;
                }
                Error => text.color = 0xffff8787,
            }
        }
        text.size = s.font_size.unwrap_or(text.size);
        text.color = s.foreground.unwrap_or(text.color);
        text.bold |= s.bold;
        text.wrap = if n.kind == Kind::TextField {
            n.multiline
        } else {
            s.text_wrap
                .map_or(s.wrap, |wrap| wrap != crate::TextWrap::NoWrap)
        };
        text.align = s.text_align;
        text.overflow = s.text_overflow;
        text.max_lines = s.max_lines;
        text.line_height = s.line_height.unwrap_or(parent.line_height);
        text.letter_spacing = s.letter_spacing;
        if s.uppercase || s.label_style == Some(crate::LabelStyle::Section) {
            nodes[i].text = n.text.to_uppercase();
        }
        nodes[i].text_style = text;
        let width = s
            .width
            .resolve(available.width)
            .unwrap_or(available.width)
            .min(s.max_width)
            .max(s.min_width);
        let inner = (width - s.padding.horizontal()).max(0.);
        let layout = self.measure.text(
            &nodes[i].text,
            text,
            if text.constrained() {
                inner
            } else {
                f64::INFINITY
            },
        );
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
                let m = nodes[child].style.margin.main(horizontal(n.kind));
                let (a, b) = if horizontal(n.kind) {
                    (
                        child_size.width + m,
                        child_size.height + nodes[child].style.margin.vertical(),
                    )
                } else {
                    (
                        child_size.height + m,
                        child_size.width + nodes[child].style.margin.horizontal(),
                    )
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
                    .filter(|c| nodes[**c].style.position != Position::Absolute)
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
                Kind::TextField => Size::new(240., control_height.max(text.size + 14.)),
                Kind::Button | Kind::Toggle | Kind::Segment => Size::new(
                    layout.size.width + 28.,
                    control_height.max(layout.size.height + 10.),
                ),
                Kind::CheckBox | Kind::Radio => Size::new(
                    layout.size.width + 30.,
                    control_height.max(layout.size.height + 6.),
                ),
                Kind::Switch => Size::new(
                    layout.size.width + 54.,
                    control_height.max(layout.size.height + 6.),
                ),
                Kind::Slider => Size::new(180., control_height),
                Kind::Progress => Size::new(180., 8.),
                Kind::Separator => {
                    if s.orientation == crate::Orientation::Vertical {
                        Size::new(1., 1.)
                    } else {
                        Size::new(
                            if available.width.is_finite() {
                                available.width
                            } else {
                                1.
                            },
                            1.,
                        )
                    }
                }
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
        size.width += s.padding.horizontal();
        size.height += s.padding.vertical();
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
        containing: Size,
        nodes: &mut [Node],
    ) -> Result<(), Error> {
        let n = nodes[i].clone();
        let s = n.style;
        let mut bounds = bounds;
        if s.position == Position::Relative {
            bounds.x += s
                .left
                .resolve(containing.width)
                .unwrap_or_else(|| -s.right.resolve(containing.width).unwrap_or(0.));
            bounds.y += s
                .top
                .resolve(containing.height)
                .unwrap_or_else(|| -s.bottom.resolve(containing.height).unwrap_or(0.));
        }
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
                .map(|c| self.desired[&nodes[*c].id].height + nodes[*c].style.margin.vertical())
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
                .filter(|c| nodes[*c].style.position != Position::Absolute)
                .collect();
            if n.kind == Kind::Layer {
                for child in flowing {
                    let d = self.desired[&nodes[child].id];
                    let m = nodes[child].style.margin;
                    self.arrange(
                        child,
                        Rect::new(
                            content.x + m.left,
                            content.y + m.top,
                            nodes[child].style.width.resolve(content.width).unwrap_or(
                                if s.align == Align::Stretch {
                                    (content.width - m.horizontal()).max(0.)
                                } else {
                                    d.width
                                },
                            ),
                            nodes[child].style.height.resolve(content.height).unwrap_or(
                                if s.align == Align::Stretch {
                                    (content.height - m.vertical()).max(0.)
                                } else {
                                    d.height
                                },
                            ),
                        ),
                        nodes[i].clip,
                        Size::new(content.width, content.height),
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
                    let m = nodes[child].style.margin.main(horizontal(n.kind));
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
                            let style = nodes[*c].style;
                            if horiz {
                                style
                                    .basis
                                    .resolve(main_limit)
                                    .or_else(|| style.width.resolve(main_limit))
                                    .unwrap_or(d.width)
                            } else {
                                style
                                    .basis
                                    .resolve(main_limit)
                                    .or_else(|| style.height.resolve(main_limit))
                                    .unwrap_or(d.height)
                            }
                        })
                        .collect();
                    let margins: f64 = line
                        .iter()
                        .map(|c| nodes[*c].style.margin.main(horiz))
                        .sum();
                    let available =
                        (main_limit - margins - gap * line.len().saturating_sub(1) as f64).max(0.);
                    let mut sizes = flex_sizes(
                        &line,
                        &natural,
                        available,
                        horiz,
                        !matches!(n.kind, Kind::Block | Kind::Fragment | Kind::Scroll),
                        nodes,
                    );
                    if n.kind == Kind::Fragment
                        && line.len() == 1
                        && nodes[line[0]].style.height == Length::Auto
                    {
                        sizes[0] = available.clamp(
                            nodes[line[0]].style.min_height,
                            nodes[line[0]].style.max_height,
                        );
                    }
                    let allocated: f64 = sizes.iter().sum();
                    let remaining = (available - allocated).max(0.);
                    let mut cursor = match s.justify {
                        Justify::SpaceAround => remaining / (2. * line.len() as f64),
                        Justify::Center => remaining / 2.,
                        Justify::End => remaining,
                        _ => 0.,
                    };
                    let spacing = gap
                        + if s.justify == Justify::SpaceBetween && line.len() > 1 {
                            remaining / (line.len() - 1) as f64
                        } else if s.justify == Justify::SpaceAround {
                            remaining / line.len() as f64
                        } else {
                            0.
                        };
                    let line_cross = if horiz && s.wrap {
                        line.iter()
                            .map(|c| {
                                self.desired[&nodes[*c].id].height
                                    + nodes[*c].style.margin.vertical()
                            })
                            .fold(0., f64::max)
                    } else {
                        cross_limit
                    };
                    let baseline = line
                        .iter()
                        .map(|c| {
                            self.layouts[&nodes[*c].id].baseline
                                + nodes[*c].style.padding.top
                                + nodes[*c].style.margin.top
                        })
                        .fold(0., f64::max);
                    for (j, child) in line.iter().enumerate() {
                        let cs = nodes[*child].style;
                        let desired = self.desired[&nodes[*child].id];
                        let natural_cross = if horiz { desired.height } else { desired.width };
                        let explicit = if horiz { cs.height } else { cs.width };
                        let natural_cross = explicit.resolve(cross_limit).unwrap_or(natural_cross);
                        let align = cs.align_self.unwrap_or(s.align);
                        let cross = if align == Align::Stretch && explicit == Length::Auto {
                            (line_cross - cs.margin.main(!horiz)).max(0.)
                        } else {
                            natural_cross
                        };
                        let cross = cross.clamp(
                            if horiz { cs.min_height } else { cs.min_width },
                            if horiz { cs.max_height } else { cs.max_width },
                        );
                        let offset = match align {
                            Align::Center => (line_cross - cross) / 2.,
                            Align::End => line_cross - cross - cs.margin.after(!horiz),
                            Align::Baseline if horiz => {
                                baseline - self.layouts[&nodes[*child].id].baseline - cs.padding.top
                            }
                            _ => cs.margin.before(!horiz),
                        };
                        let r = if horiz {
                            Rect::new(
                                content.x + cursor + cs.margin.before(horiz),
                                content.y + line_offset + offset,
                                sizes[j],
                                cross,
                            )
                        } else {
                            Rect::new(
                                content.x + offset,
                                content.y + cursor + cs.margin.before(horiz),
                                cross,
                                sizes[j],
                            )
                        };
                        self.arrange(
                            *child,
                            r,
                            nodes[i].clip,
                            Size::new(content.width, content.height),
                            nodes,
                        )?;
                        cursor += sizes[j] + cs.margin.main(horiz) + spacing;
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
                    cs.width.resolve(content.width).unwrap_or(d.width)
                };
                let h = if cs.height == Length::Auto
                    && let (Some(t), Some(b)) = (t, b)
                {
                    (content.height - t - b).max(0.)
                } else {
                    cs.height.resolve(content.height).unwrap_or(d.height)
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
                    Size::new(content.width, content.height),
                    nodes,
                )?;
            }
        } else {
            let mut text_area = content;
            match n.kind {
                Kind::Button | Kind::Toggle | Kind::Segment => {
                    text_area.x += 14.;
                    text_area.width = (text_area.width - 28.).max(0.);
                }
                Kind::CheckBox | Kind::Radio => {
                    text_area.x += 30.;
                    text_area.width = (text_area.width - 30.).max(0.);
                }
                Kind::Switch => {
                    text_area.x += 54.;
                    text_area.width = (text_area.width - 54.).max(0.);
                }
                _ => {}
            }
            let constrained = n.text_style.constrained();
            let layout = self.measure.text(
                &n.text,
                n.text_style,
                if constrained {
                    text_area.width
                } else {
                    f64::INFINITY
                },
            );
            let x = if matches!(n.kind, Kind::Button | Kind::Toggle | Kind::Segment) && !constrained
            {
                content.x + (content.width - layout.size.width) / 2.
            } else {
                text_area.x
            };
            nodes[i].text_bounds = Rect::new(
                x,
                content.y + (content.height - layout.size.height) / 2.,
                if constrained {
                    text_area.width
                } else {
                    layout.size.width
                },
                layout.size.height,
            );
            self.layouts.insert(n.id, layout);
        }
        Ok(())
    }
}

/// Distribute free space repeatedly so items frozen at min/max do not strand space.
fn flex_sizes(
    line: &[usize],
    basis: &[f64],
    available: f64,
    horizontal: bool,
    shrink: bool,
    nodes: &[Node],
) -> Vec<f64> {
    let limits: Vec<_> = line
        .iter()
        .map(|i| {
            let s = nodes[*i].style;
            if horizontal {
                (s.min_width, s.max_width)
            } else {
                (s.min_height, s.max_height)
            }
        })
        .collect();
    let mut sizes: Vec<_> = basis
        .iter()
        .zip(&limits)
        .map(|(n, (min, max))| n.clamp(*min, *max))
        .collect();
    for _ in 0..=line.len() {
        let free = available - sizes.iter().sum::<f64>();
        if free.abs() < 0.000001 {
            break;
        }
        let weights: Vec<_> = line
            .iter()
            .enumerate()
            .map(|(j, i)| {
                let s = nodes[*i].style;
                if free > 0. && sizes[j] < limits[j].1 {
                    s.grow
                } else if free < 0. && shrink && sizes[j] > limits[j].0 {
                    s.shrink * basis[j]
                } else {
                    0.
                }
            })
            .collect();
        let total = weights.iter().sum::<f64>();
        if total <= 0. {
            break;
        }
        for (j, value) in sizes.iter_mut().enumerate() {
            *value = (*value + free * weights[j] / total).clamp(limits[j].0, limits[j].1);
        }
    }
    sizes
}
