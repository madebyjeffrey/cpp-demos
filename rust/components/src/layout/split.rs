use super::*;
impl Tree<'_> {
    fn split_children(&self, i: usize, nodes: &[Node]) -> [usize; 3] {
        self.children[&Some(nodes[i].id)]
            .as_slice()
            .try_into()
            .expect("split owns two panes and a divider")
    }
    pub(super) fn measure_split(
        &mut self,
        i: usize,
        available: Size,
        text: TextStyle,
        nodes: &mut [Node],
    ) -> Result<Size, Error> {
        let [first, divider, second] = self.split_children(i, nodes);
        let options = nodes[i].split_options.unwrap();
        let a = self.measure_node(first, available, text, nodes)?;
        self.measure_node(divider, available, text, nodes)?;
        let b = self.measure_node(second, available, text, nodes)?;
        Ok(if options.orientation == crate::Orientation::Horizontal {
            Size::new(
                a.width + b.width + options.divider_width,
                a.height.max(b.height),
            )
        } else {
            Size::new(
                a.width.max(b.width),
                a.height + b.height + options.divider_width,
            )
        })
    }
    pub(super) fn reflow_split(
        &mut self,
        i: usize,
        width: f64,
        height: Option<f64>,
        nodes: &mut [Node],
    ) -> Result<f64, Error> {
        let n = nodes[i].clone();
        let options = n.split_options.unwrap();
        let [first, _, second] = self.split_children(i, nodes);
        let h = height.unwrap_or((self.desired[&n.id].height - n.style.padding.vertical()).max(0.));
        let info = options.geometry(
            Rect::new(0., 0., width, h),
            n.split_ratio,
            n.split_collapsed,
        );
        for (child, r) in [(first, info.first), (second, info.second)] {
            self.reflow(child, r.width, height.map(|_| r.height), nodes)?;
        }
        let a = self.desired[&nodes[first].id].height;
        let b = self.desired[&nodes[second].id].height;
        Ok(if options.orientation == crate::Orientation::Horizontal {
            a.max(b)
        } else {
            a + b + options.divider_width
        } + n.style.padding.vertical())
    }
    pub(super) fn arrange_split(
        &mut self,
        i: usize,
        content: Rect,
        nodes: &mut [Node],
    ) -> Result<(), Error> {
        let n = nodes[i].clone();
        let info = n
            .split_options
            .unwrap()
            .geometry(content, n.split_ratio, n.split_collapsed);
        nodes[i].split = Some(info);
        let [first, divider, second] = self.split_children(i, nodes);
        let clip = n.clip.intersection(content);
        let size = Size::new(content.width, content.height);
        for (child, r) in [
            (first, info.first),
            (second, info.second),
            (divider, info.divider),
        ] {
            if child == divider {
                nodes[child].split = Some(info);
            }
            self.reflow(child, r.width, Some(r.height), nodes)?;
            self.arrange(child, r, clip, size, nodes)?;
            if r.width <= 0.
                || r.height <= 0.
                || child == first && info.collapsed == crate::SplitPane::First
                || child == second && info.collapsed == crate::SplitPane::Second
            {
                self.hide_subtree(child, nodes);
            }
        }
        let value = if info.orientation == crate::Orientation::Horizontal {
            info.first.width
        } else {
            info.first.height
        };
        nodes[divider].value = crate::Value::Number(value);
        nodes[divider].range = Some((
            if info.collapsed == crate::SplitPane::First {
                0.
            } else {
                info.limits.0
            },
            if info.collapsed == crate::SplitPane::Second {
                info.available
            } else {
                info.limits.1
            },
            info.keyboard_step,
        ));
        Ok(())
    }
}
