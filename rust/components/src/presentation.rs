//! Composable presentation models. State lives outside view construction.
use crate::{Menu, MenuAction, OptionItem, State, View};
#[derive(Clone)]
pub struct Popup {
    pub open: State<bool>,
    menu: Menu,
    options: crate::OverlayOptions,
    close: Option<crate::overlay::DismissCallback>,
}
impl Popup {
    pub fn new() -> Self {
        let menu = Menu::new();
        Self {
            open: menu.open.clone(),
            menu,
            options: Default::default(),
            close: None,
        }
    }
    pub fn options(mut self, options: crate::OverlayOptions) -> Self {
        self.options = options;
        self
    }
    pub fn on_dismiss(mut self, callback: impl Fn(crate::DismissReason) + 'static) -> Self {
        self.close = Some(std::rc::Rc::new(callback));
        self
    }
    pub fn button(&self, label: impl Into<String>, anchor: impl Into<String>) -> View {
        let popup = self.clone();
        View::button(label, move || popup.toggle()).key(anchor.into())
    }
    fn attach(&self, mut view: View) -> View {
        let popup = self.clone();
        let presented = self.clone();
        let generation = self.menu.generation();
        let data = crate::overlay::OverlayData {
            options: self.options,
            point: None,
            presented: Some(std::rc::Rc::new(move || {
                presented.open.get() && presented.menu.generation() == generation
            })),
            close: std::rc::Rc::new(move |reason| {
                if popup.menu.generation() == generation {
                    popup.dismiss();
                }
                if let Some(close) = &popup.close {
                    close(reason);
                }
            }),
        };
        std::rc::Rc::make_mut(&mut view.0).overlay = Some(data);
        view.key(self.menu.presentation_key())
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
        self.attach(View::modal(
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
        ))
    }
    pub fn popover(&self, anchor: impl Into<String>, body: View) -> View {
        if !self.open.get() {
            return View::fragment(vec![]);
        }
        let close = self.clone();
        self.attach(View::popover(
            anchor,
            vec![View::scroll(vec![body]).scrollbars(true)],
            move || close.dismiss(),
        ))
    }
    pub fn menu(&self, anchor: impl Into<String>, items: Vec<MenuAction>) -> View {
        let mut menu = self.menu.clone().options(self.options);
        menu.close = self.close.clone();
        menu.view(anchor, items)
    }
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
