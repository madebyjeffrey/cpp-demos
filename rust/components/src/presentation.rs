//! Composable presentation models. State lives outside view construction.
use crate::{Menu, MenuAction, OptionItem, State, View};
#[derive(Clone)]
pub struct Popup {
    pub open: State<bool>,
    menu: Menu,
}
impl Popup {
    pub fn new() -> Self {
        let menu = Menu::new();
        Self {
            open: menu.open.clone(),
            menu,
        }
    }
    pub fn show(&self) {
        self.menu.show()
    }
    pub fn dismiss(&self) {
        self.menu.dismiss()
    }
    pub fn toggle(&self) {
        self.menu.toggle()
    }
    pub fn modal(&self, title: impl Into<String>, body: View) -> View {
        if !self.open.get() {
            return View::fragment(vec![]);
        }
        let close = self.clone();
        let button = self.clone();
        View::modal(
            vec![
                View::panel(vec![
                    View::label(title).font_size(22.).bold(true),
                    body,
                    View::button("Done", move || button.dismiss()),
                ])
                .padding(24.)
                .width(400),
            ],
            move || close.dismiss(),
        )
    }
    pub fn popover(&self, anchor: impl Into<String>, body: View) -> View {
        if !self.open.get() {
            return View::fragment(vec![]);
        }
        let close = self.clone();
        View::popover(anchor, vec![body], move || close.dismiss())
    }
    pub fn menu(&self, anchor: impl Into<String>, items: Vec<MenuAction>) -> View {
        self.menu.view(anchor, items)
    }
}
/// Tabs compose ordinary views. Only the selected page is mounted.
pub fn tabs(selected: &State<String>, pages: Vec<(String, String, View)>) -> View {
    let options = pages
        .iter()
        .map(|(id, label, _)| OptionItem::new(id, label))
        .collect();
    let active = selected.get();
    let content = pages
        .into_iter()
        .find(|(id, _, _)| *id == active)
        .map(|(_, _, v)| v)
        .unwrap_or_else(|| View::label("No selected page"));
    View::column(vec![
        View::segmented(selected.binding(), options),
        content.grow(1.),
    ])
}
pub fn alert(popup: &Popup, title: impl Into<String>, message: impl Into<String>) -> View {
    popup.modal(title, View::label(message).wrap(true))
}
pub fn dropdown(
    label: impl Into<String>,
    selected: &State<String>,
    options: Vec<OptionItem>,
    popup: &Popup,
    key: impl Into<String>,
) -> (View, View) {
    let key = key.into();
    let toggle = popup.clone();
    let selected_label = options
        .iter()
        .find(|o| o.id == selected.get())
        .map(|o| o.label.as_str())
        .unwrap_or("");
    let button = View::button(
        format!("{}: {} ▾", label.into(), selected_label),
        move || toggle.toggle(),
    )
    .key(&key);
    let items = options
        .into_iter()
        .map(|o| {
            let selected = selected.clone();
            let id = o.id.clone();
            MenuAction::new(o.label, move || selected.set(o.id.clone()))
                .key(id)
                .enabled(o.enabled)
        })
        .collect();
    (button, popup.menu(key, items))
}

impl Default for Popup {
    fn default() -> Self {
        Self::new()
    }
}

pub fn wizard(step: &State<usize>, pages: Vec<View>) -> View {
    let count = pages.len();
    let current = step.get().min(count.saturating_sub(1));
    let back = step.clone();
    let next = step.clone();
    let content = pages
        .into_iter()
        .nth(current)
        .unwrap_or_else(|| View::label("No steps"));
    View::column(vec![
        View::label(format!("Step {} of {}", current + 1, count)),
        content.grow(1.),
        View::row(vec![
            View::button("Back", move || back.update(|n| *n = n.saturating_sub(1)))
                .enabled(current > 0),
            View::button("Next", move || {
                next.update(|n| *n = (*n + 1).min(count.saturating_sub(1)))
            })
            .enabled(current + 1 < count),
        ]),
    ])
}
pub fn split_button(
    label: impl Into<String>,
    action: impl Fn() + 'static,
    popup: &Popup,
    key: impl Into<String>,
) -> View {
    let open = popup.clone();
    View::row(vec![
        View::button(label, action).grow(1.),
        View::button("▾", move || open.toggle())
            .key(key.into())
            .width(36),
    ])
}
