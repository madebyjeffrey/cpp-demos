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
        let options = nodes[i].overlay.unwrap_or_default();
        let anchor = nodes[i]
            .anchor
            .as_ref()
            .and_then(|key| nodes.iter().find(|n| n.key.as_ref() == Some(key)))
            .map(|n| n.bounds)
            .unwrap_or(Rect::new(viewport.x, viewport.y, 0., 0.));
        let anchor = nodes[i]
            .overlay_owner
            .and_then(|id| nodes.iter().find(|n| n.id == id))
            .map(|n| n.bounds)
            .unwrap_or(anchor);
        let anchor = nodes[i]
            .overlay_point
            .map(|p| Rect::new(p.x, p.y, 0., 0.))
            .unwrap_or(anchor);
        for (depth, panel) in panels.into_iter().enumerate() {
            let desired = self.desired[&nodes[panel].id];
            let width = options.width.unwrap_or(desired.width).min(viewport.width);
            let height = desired.height.min(options.max_height).min(viewport.height);
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
            let bounds = crate::overlay_bounds(
                target,
                Size::new(width, height),
                viewport,
                if depth == 0 {
                    options.placement
                } else {
                    crate::OverlayPlacement::Right
                },
                if depth == 0 { options.gap } else { 0. },
                options.margin,
            );
            self.reflow(panel, width, Some(height), nodes)?;
            self.arrange(panel, bounds, viewport, Size::new(width, height), nodes)?;
        }
        Ok(())
    }
}
