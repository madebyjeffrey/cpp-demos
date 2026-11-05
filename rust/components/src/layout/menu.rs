use super::*;
impl Tree<'_> {
    pub(super) fn arrange_menu(
        &mut self,
        i: usize,
        viewport: Rect,
        nodes: &mut [Node],
    ) -> Result<(), Error> {
        let panels = self
            .children
            .get(&Some(nodes[i].id))
            .cloned()
            .unwrap_or_default();
        let anchor = nodes[i]
            .anchor
            .as_ref()
            .and_then(|key| nodes.iter().find(|n| n.key.as_ref() == Some(key)))
            .map(|n| n.bounds)
            .unwrap_or(Rect::new(viewport.x, viewport.y, 0., 0.));
        for (depth, panel) in panels.into_iter().enumerate() {
            let desired = self.desired[&nodes[panel].id];
            let width = desired.width.min(viewport.width);
            let height = desired.height.min(viewport.height);
            let target = if depth == 0 {
                anchor
            } else {
                let path = &nodes[panel].menu_path;
                nodes
                    .iter()
                    .find(|n| n.menu_item.as_ref().is_some_and(|m| &m.path == path))
                    .map(|n| n.bounds)
                    .unwrap_or(anchor)
            };
            let x = if depth == 0 {
                target.x
            } else if target.x + target.width + width > viewport.x + viewport.width {
                target.x - width
            } else {
                target.x + target.width
            };
            let y = if depth == 0 {
                if target.y + target.height + 4. + height > viewport.y + viewport.height {
                    target.y - height - 4.
                } else {
                    target.y + target.height + 4.
                }
            } else {
                target.y - 6.
            };
            let bounds = Rect::new(
                x.clamp(
                    viewport.x,
                    (viewport.x + viewport.width - width).max(viewport.x),
                ),
                y.clamp(
                    viewport.y,
                    (viewport.y + viewport.height - height).max(viewport.y),
                ),
                width,
                height,
            );
            self.reflow(panel, width, Some(height), nodes)?;
            self.arrange(panel, bounds, viewport, Size::new(width, height), nodes)?;
        }
        Ok(())
    }
}
