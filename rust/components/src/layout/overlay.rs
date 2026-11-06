use super::*;
impl Tree<'_> {
    pub(super) fn arrange_popover(
        &mut self,
        i: usize,
        viewport: Rect,
        nodes: &mut [Node],
    ) -> Result<(), Error> {
        let n = &nodes[i];
        let options = n.overlay.unwrap_or_default();
        let anchor = n
            .overlay_point
            .map(|p| Rect::new(p.x, p.y, 0., 0.))
            .or_else(|| {
                n.anchor
                    .as_ref()
                    .and_then(|key| nodes.iter().find(|n| n.key.as_ref() == Some(key)))
                    .map(|n| n.bounds)
            })
            .unwrap_or_default();
        let children = self.children.get(&Some(n.id)).cloned().unwrap_or_default();
        let width = options.width.unwrap_or_else(|| {
            children
                .iter()
                .map(|c| self.desired[&nodes[*c].id].width)
                .fold(0., f64::max)
        });
        let height = children
            .iter()
            .map(|c| self.desired[&nodes[*c].id].height)
            .sum::<f64>()
            .min(options.max_height);
        let bounds = crate::overlay_bounds(
            anchor,
            Size::new(width, height),
            viewport,
            options.placement,
            options.gap,
            options.margin,
        );
        let mut y = bounds.y;
        for child in children {
            let h = self.desired[&nodes[child].id]
                .height
                .min((bounds.y + bounds.height - y).max(0.));
            self.reflow(child, bounds.width, Some(h), nodes)?;
            self.arrange(
                child,
                Rect::new(bounds.x, y, bounds.width, h),
                bounds,
                Size::new(bounds.width, h),
                nodes,
            )?;
            y += h;
        }
        Ok(())
    }
}
