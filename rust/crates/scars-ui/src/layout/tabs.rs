use super::*;
impl Tree<'_> {
    fn tab_children(&self, i: usize, nodes: &[Node]) -> (usize, Option<usize>, Vec<usize>) {
        let children = self.children[&Some(nodes[i].id)].clone();
        let strip = children[0];
        let selected = nodes[i].tabs.as_ref().and_then(|t| t.selected.as_ref());
        let active = children
            .iter()
            .skip(1)
            .find(|c| nodes[**c].kind == Kind::TabPanel && nodes[**c].tab_id.as_ref() == selected)
            .copied();
        (
            strip,
            active,
            children
                .into_iter()
                .skip(1)
                .filter(|c| nodes[*c].kind == Kind::TabPanel)
                .collect(),
        )
    }
    pub(super) fn measure_tabs(
        &mut self,
        i: usize,
        available: Size,
        text: TextStyle,
        nodes: &mut [Node],
    ) -> Result<Size, Error> {
        let (strip, page, _) = self.tab_children(i, nodes);
        let header = self.measure_node(strip, available, text, nodes)?;
        let body = if let Some(page) = page {
            self.measure_node(page, available, text, nodes)?
        } else {
            Size::default()
        };
        Ok(Size::new(
            header.width.max(body.width),
            header.height
                + if page.is_some() {
                    nodes[i].style.gap + body.height
                } else {
                    0.
                },
        ))
    }
    pub(super) fn reflow_tabs(
        &mut self,
        i: usize,
        width: f64,
        height: Option<f64>,
        nodes: &mut [Node],
    ) -> Result<f64, Error> {
        let (strip, page, _) = self.tab_children(i, nodes);
        // Flex/split allocation can be narrower than the first measurement.
        // Recompute natural header widths against the actual strip viewport.
        let row = self.children[&Some(nodes[strip].id)][0];
        let headers = self
            .children
            .get(&Some(nodes[row].id))
            .cloned()
            .unwrap_or_default();
        let mut extent = nodes[row].style.gap * headers.len().saturating_sub(1) as f64;
        for header in headers {
            let n = &nodes[header];
            let natural = self
                .measure
                .text(&n.text, n.text_style, f64::INFINITY)
                .size
                .width
                .ceil()
                + 29.;
            let w = natural
                .min(width)
                .clamp(n.style.min_width, n.style.max_width);
            self.desired.get_mut(&n.id).unwrap().width = w;
            extent += w + n.style.margin.horizontal();
        }
        self.desired.get_mut(&nodes[row].id).unwrap().width = extent;
        self.reflow(strip, width, None, nodes)?;
        let head = self.desired[&nodes[strip].id].height;
        let body = if let Some(page) = page {
            self.reflow(
                page,
                width,
                height.map(|h| (h - head - nodes[i].style.gap).max(0.)),
                nodes,
            )?;
            self.desired[&nodes[page].id].height
        } else {
            0.
        };
        Ok(nodes[i].style.padding.vertical()
            + head
            + if page.is_some() {
                nodes[i].style.gap + body
            } else {
                0.
            })
    }
    pub(super) fn arrange_tabs(
        &mut self,
        i: usize,
        content: Rect,
        nodes: &mut [Node],
    ) -> Result<(), Error> {
        let (strip, page, pages) = self.tab_children(i, nodes);
        let head = self.desired[&nodes[strip].id].height.min(content.height);
        self.arrange(
            strip,
            Rect::new(content.x, content.y, content.width, head),
            nodes[i].clip,
            Size::new(content.width, head),
            nodes,
        )?;
        for child in pages {
            if Some(child) == page {
                let gap = nodes[i].style.gap;
                let bounds = Rect::new(
                    content.x,
                    content.y + head + gap,
                    content.width,
                    (content.height - head - gap).max(0.),
                );
                self.reflow(child, bounds.width, Some(bounds.height), nodes)?;
                self.arrange(
                    child,
                    bounds,
                    nodes[i].clip,
                    Size::new(bounds.width, bounds.height),
                    nodes,
                )?;
            } else {
                self.clear_tab_page(child, nodes);
            }
        }
        Ok(())
    }
    fn clear_tab_page(&self, i: usize, nodes: &mut [Node]) {
        nodes[i].layout_hidden = true;
        nodes[i].bounds = Rect::default();
        nodes[i].clip = Rect::default();
        nodes[i].text_bounds = Rect::default();
        for child in self.children.get(&Some(nodes[i].id)).into_iter().flatten() {
            self.clear_tab_page(*child, nodes);
        }
    }
}
