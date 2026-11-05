use crate::{Binding, Host, Kind, Length, Node, NodeId, Point, Rect, State, Style, View};
use std::collections::HashMap;
#[derive(Clone, Debug, PartialEq)]
pub struct Inspection {
    pub id: NodeId,
    pub kind: Kind,
    pub key: Option<String>,
    pub text: String,
    pub bounds: Rect,
    pub style: Style,
}
impl From<Node> for Inspection {
    fn from(n: Node) -> Self {
        Self {
            id: n.id,
            kind: n.kind,
            key: n.key,
            text: n.text,
            bounds: n.bounds,
            style: n.style,
        }
    }
}
#[derive(Clone)]
pub struct Inspector {
    pub picking: State<bool>,
    pub selected: State<Option<Inspection>>,
    pub overrides: State<HashMap<NodeId, Style>>,
}
impl Inspector {
    pub fn new() -> Self {
        Self {
            picking: State::new(false),
            selected: State::new(None),
            overrides: State::new(HashMap::new()),
        }
    }
    pub fn view(&self) -> View {
        let mut children = vec![View::checkbox(
            "Pick a control · Escape exits",
            self.picking.binding(),
        )];
        if let Some(info) = self.selected.get() {
            children.push(View::label(format!("{:?} #{}", info.kind, info.id)).bold(true));
            children.push(
                View::label(format!(
                    "Key: {}\n{:.0} × {:.0} at {:.0}, {:.0}",
                    info.key.unwrap_or_default(),
                    info.bounds.width,
                    info.bounds.height,
                    info.bounds.x,
                    info.bounds.y
                ))
                .wrap(true),
            );
            let read = self.selected.clone();
            let write = self.clone();
            children.push(
                View::slider(
                    Binding::new(
                        move || read.get().map_or(100., |n| n.bounds.width),
                        move |width: f64| {
                            if let Some(info) = write.selected.get() {
                                let mut style = info.style;
                                style.width = Length::Points(width);
                                write.overrides.update(|map| {
                                    map.insert(info.id, style);
                                });
                            }
                        },
                    ),
                    40.,
                    800.,
                    1.,
                )
                .accessible_label("Override width"),
            );
            children.push(View::button(
                "Reset overrides",
                self.overrides.action(|map| map.set(HashMap::new())),
            ));
        } else {
            children.push(
                View::label(
                    "Pick a rendered control to inspect its bounds and try a width override.",
                )
                .wrap(true),
            );
        }
        View::panel(children).padding(16.)
    }
}
impl Default for Inspector {
    fn default() -> Self {
        Self::new()
    }
}
impl Host {
    pub fn set_inspector(&self, inspector: Inspector) {
        *self.0.inspector.borrow_mut() = inspector;
        self.0.observer.dirty.set(true);
        self.redraw();
    }
    pub fn inspector(&self) -> Inspector {
        self.0.inspector.borrow().clone()
    }
    pub fn pick(&self, point: Point) -> Result<Option<Inspection>, crate::Error> {
        Ok(self
            .scene()?
            .into_iter()
            .rev()
            .find(|n| !n.hidden && n.clip.contains(point) && self.in_scope(n.id))
            .map(Inspection::from))
    }
}
