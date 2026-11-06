//! Menu trees share one modal scope. Models live outside view construction.
use crate::{Binding, Error, Host, Key, KeyEvent, NodeId, Pointer, PointerKind, State, View};
use std::{
    cell::RefCell,
    rc::Rc,
    time::{Duration, Instant},
};

#[derive(Clone)]
enum Action {
    Run(Rc<dyn Fn()>),
    Check(Binding<bool>),
    Radio(Binding<String>, String),
    Submenu(Vec<MenuAction>),
    Separator,
}
/// A stable key defaults to the label; use `key` when labels can change.
/// Shortcut text is a hint; accelerator registration belongs to the application.
#[derive(Clone)]
pub struct MenuAction {
    pub label: String,
    pub enabled: bool,
    id: String,
    shortcut: String,
    action: Action,
}
impl MenuAction {
    pub fn new(label: impl Into<String>, action: impl Fn() + 'static) -> Self {
        Self::entry(label, Action::Run(Rc::new(action)))
    }
    fn entry(label: impl Into<String>, action: Action) -> Self {
        let label = label.into();
        Self {
            id: label.clone(),
            label,
            enabled: true,
            shortcut: String::new(),
            action,
        }
    }
    pub fn check(label: impl Into<String>, value: Binding<bool>) -> Self {
        Self::entry(label, Action::Check(value))
    }
    pub fn radio(
        label: impl Into<String>,
        value: Binding<String>,
        choice: impl Into<String>,
    ) -> Self {
        Self::entry(label, Action::Radio(value, choice.into()))
    }
    pub fn submenu(label: impl Into<String>, children: Vec<Self>) -> Self {
        Self::entry(label, Action::Submenu(children))
    }
    pub fn separator() -> Self {
        Self::entry("", Action::Separator)
    }
    pub fn key(mut self, id: impl Into<String>) -> Self {
        self.id = id.into();
        self
    }
    pub fn shortcut(mut self, text: impl Into<String>) -> Self {
        self.shortcut = text.into();
        self
    }
    pub fn enabled(mut self, value: bool) -> Self {
        self.enabled = value;
        self
    }
    fn selectable(&self) -> bool {
        self.enabled && !matches!(self.action, Action::Separator)
    }
    fn executable(&self) -> bool {
        self.selectable() && !matches!(self.action, Action::Submenu(_))
    }
    fn invoke(&self) {
        match &self.action {
            Action::Run(f) => f(),
            Action::Check(b) => b.set(!b.get()),
            Action::Radio(b, value) => b.set(value.clone()),
            _ => (),
        }
    }
    fn checked(&self) -> Option<bool> {
        match &self.action {
            Action::Check(b) => Some(b.get()),
            Action::Radio(b, value) => Some(b.get() == *value),
            _ => None,
        }
    }
}
#[derive(Default)]
struct Typeahead {
    query: String,
    at: Option<Instant>,
}
/// Retained opening/path state. Create once; rebuild `view` with current items.
#[derive(Clone)]
pub struct Menu {
    pub open: State<bool>,
    id: u64,
    presentation: State<u64>,
    options: crate::OverlayOptions,
    point: State<Option<crate::Point>>,
    pub(crate) close: Option<crate::overlay::DismissCallback>,
    path: State<Vec<String>>,
    typing: Rc<RefCell<Typeahead>>,
}
impl Default for Menu {
    fn default() -> Self {
        Self::new()
    }
}
impl Menu {
    pub fn new() -> Self {
        Self {
            open: State::new(false),
            id: {
                static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
                NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
            },
            presentation: State::new(0),
            options: Default::default(),
            point: State::new(None),
            close: None,
            path: State::new(vec![]),
            typing: Rc::default(),
        }
    }
    pub(crate) fn generation(&self) -> u64 {
        self.presentation.get()
    }
    pub(crate) fn presentation_key(&self) -> String {
        format!("$overlay-{}-{}", self.id, self.presentation.get())
    }
    pub(crate) fn identity(&self) -> u64 {
        self.id
    }
    pub fn options(mut self, options: crate::OverlayOptions) -> Self {
        self.options = options;
        self
    }
    pub fn on_dismiss(mut self, callback: impl Fn(crate::DismissReason) + 'static) -> Self {
        self.close = Some(Rc::new(callback));
        self
    }
    pub fn show_at(&self, point: crate::Point) {
        self.show();
        self.point.set(Some(point));
    }
    pub fn show(&self) {
        if !self.open.get() {
            self.presentation.update(|n| *n = n.wrapping_add(1));
        }
        self.point.set(None);
        self.path.set(vec![]);
        self.typing.replace(Typeahead::default());
        self.open.set(true);
    }
    pub fn dismiss(&self) {
        self.open.set(false);
        self.path.set(vec![]);
        self.typing.replace(Typeahead::default());
    }
    pub fn toggle(&self) {
        if self.open.get() {
            self.dismiss()
        } else {
            self.show()
        }
    }
    /// An anchored trigger with no application-side clone/callback plumbing.
    pub fn button(&self, label: impl Into<String>, anchor: impl Into<String>) -> View {
        let menu = self.clone();
        View::button(label, move || menu.toggle()).key(anchor.into())
    }
    pub fn view(&self, anchor: impl Into<String>, items: Vec<MenuAction>) -> View {
        self.build(anchor.into(), items, None)
    }
    fn build(
        &self,
        anchor: String,
        items: Vec<MenuAction>,
        select: Option<Rc<dyn Fn(Vec<String>)>>,
    ) -> View {
        if !self.open.get() {
            return View::fragment(vec![]);
        }
        let mut panels = vec![self.panel(&items, &[])];
        let path = self.path.get();
        let mut children = &items;
        let mut prefix = vec![];
        for id in path {
            let Some(item) = children.iter().find(|a| a.id == id && a.selectable()) else {
                break;
            };
            let Action::Submenu(sub) = &item.action else {
                break;
            };
            prefix.push(id);
            panels.push(self.panel(sub, &prefix));
            children = sub;
        }
        let close = self.clone();
        let mut view =
            View::popover(anchor, panels, move || close.dismiss()).key(self.presentation_key());
        let close = self.clone();
        let presented = self.clone();
        let generation = self.generation();
        Rc::make_mut(&mut view.0).overlay = Some(crate::overlay::OverlayData {
            options: self.options,
            point: self.point.get(),
            presented: Some(Rc::new(move || {
                presented.open.get() && presented.generation() == generation
            })),
            close: Rc::new(move |reason| {
                if close.generation() == generation {
                    close.dismiss();
                }
                if let Some(callback) = &close.close {
                    callback(reason);
                }
            }),
        });
        Rc::make_mut(&mut view.0).menu = Some(MenuData {
            model: self.clone(),
            items,
            select,
        });
        view
    }
    fn panel(&self, items: &[MenuAction], prefix: &[String]) -> View {
        let rows = items
            .iter()
            .enumerate()
            .map(|(i, item)| {
                if matches!(item.action, Action::Separator) {
                    return View::separator()
                        .key(format!("$separator-{i}"))
                        .margin(4)
                        .shrink(0.);
                }
                let mut path = prefix.to_vec();
                path.push(item.id.clone());
                let mut row = View::button(&item.label, || {})
                    .key(&item.id)
                    .enabled(item.enabled)
                    .shrink(0.)
                    .padding(crate::Insets {
                        left: 28.,
                        right: 90.,
                        top: 0.,
                        bottom: 0.,
                    })
                    .text_overflow(crate::TextOverflow::Ellipsis);
                Rc::make_mut(&mut row.0).menu_item = Some(MenuItemInfo {
                    path: path.clone(),
                    checked: item.checked(),
                    submenu: matches!(item.action, Action::Submenu(_)),
                    expanded: matches!(item.action, Action::Submenu(_))
                        && self.path.get().starts_with(&path),
                    shortcut: item.shortcut.clone(),
                });
                row
            })
            .collect();
        let mut panel = View::scroll(rows)
            .width(280)
            .max_height(480.)
            .padding(6)
            .gap(2.)
            .radius(7.);
        Rc::make_mut(&mut panel.0).menu_panel = true;
        Rc::make_mut(&mut panel.0).menu_path = prefix.to_vec();
        panel.key(format!("$menu-{}", prefix.len()))
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MenuItemInfo {
    pub path: Vec<String>,
    pub checked: Option<bool>,
    pub submenu: bool,
    pub expanded: bool,
    pub shortcut: String,
}
#[derive(Clone)]
pub(crate) struct MenuData {
    model: Menu,
    items: Vec<MenuAction>,
    select: Option<Rc<dyn Fn(Vec<String>)>>,
}
fn find<'a>(items: &'a [MenuAction], path: &[String]) -> Option<&'a MenuAction> {
    let (id, rest) = path.split_first()?;
    let item = items.iter().find(|a| a.id == *id && a.selectable())?;
    if rest.is_empty() {
        Some(item)
    } else if let Action::Submenu(children) = &item.action {
        find(children, rest)
    } else {
        None
    }
}
pub(crate) fn validate(menu: &MenuData) -> Result<(), Error> {
    fn visit(items: &[MenuAction]) -> Result<(), Error> {
        let mut ids = std::collections::HashSet::new();
        for item in items {
            if matches!(item.action, Action::Separator) {
                continue;
            }
            if item.id.is_empty() || item.id.starts_with('$') || !ids.insert(&item.id) {
                return Err(Error(format!(
                    "Menu keys must be nonempty, unique among siblings and not start with '$': {}",
                    item.id
                )));
            }
            if let Action::Submenu(children) = &item.action {
                visit(children)?;
            }
        }
        Ok(())
    }
    visit(&menu.items)
}
/// Remembers a selected leaf by key path. Removed/disabled choices fall back to
/// the first enabled leaf without overwriting the retained choice.
#[derive(Clone)]
pub struct SplitAction {
    pub menu: Menu,
    selected: State<Option<Vec<String>>>,
}
impl Default for SplitAction {
    fn default() -> Self {
        Self::new()
    }
}
impl SplitAction {
    pub fn new() -> Self {
        Self {
            menu: Menu::new(),
            selected: State::new(None),
        }
    }
    pub fn selected(&self) -> Option<Vec<String>> {
        self.selected.get()
    }
    /// Returns the compound button and its overlay. Place both in a `layer`.
    pub fn view(&self, key: impl Into<String>, items: Vec<MenuAction>) -> (View, View) {
        fn first(items: &[MenuAction], prefix: Vec<String>) -> Option<(Vec<String>, MenuAction)> {
            for item in items.iter().filter(|a| a.selectable()) {
                let mut path = prefix.clone();
                path.push(item.id.clone());
                if let Action::Submenu(children) = &item.action {
                    if let Some(found) = first(children, path) {
                        return Some(found);
                    }
                } else {
                    return Some((path, item.clone()));
                }
            }
            None
        }
        let key = key.into();
        let choice = self
            .selected
            .get()
            .and_then(|path| {
                find(&items, &path)
                    .filter(|a| a.executable())
                    .map(|a| (path, a.clone()))
            })
            .or_else(|| first(&items, vec![]));
        let primary = if let Some((_, item)) = choice {
            View::button(item.label.clone(), move || item.invoke())
        } else {
            View::button("No action", || {}).enabled(false)
        };
        let toggle = self.menu.clone();
        let button = View::row(vec![
            primary.key("$primary").grow(1.),
            View::button("▾", move || toggle.toggle())
                .width(36)
                .key(&key),
        ]);
        let selected = self.selected.clone();
        let overlay = self.menu.build(
            key,
            items,
            Some(Rc::new(move |path| selected.set(Some(path)))),
        );
        (button, overlay)
    }
}

impl Host {
    fn current_menu(&self) -> Option<MenuData> {
        let id = self.active_modal()?;
        self.0.mounted.borrow().descriptions.get(&id)?.menu.clone()
    }
    fn menu_rows(&self, depth: usize) -> Vec<crate::Node> {
        self.0
            .mounted
            .borrow()
            .nodes
            .iter()
            .filter(|n| {
                n.enabled
                    && !n.hidden
                    && !n.layout_hidden
                    && self.in_scope(n.id)
                    && n.menu_item
                        .as_ref()
                        .is_some_and(|m| m.path.len() == depth + 1)
            })
            .cloned()
            .collect()
    }
    fn menu_focus(&self, depth: usize, last: bool) -> Result<bool, Error> {
        self.refresh()?;
        let rows = self.menu_rows(depth);
        match if last { rows.last() } else { rows.first() } {
            Some(n) => self.focus(n.id),
            None => Ok(true),
        }
    }
    pub(crate) fn activate_menu_item(&self, id: NodeId) -> Result<Option<bool>, Error> {
        let info = self
            .0
            .mounted
            .borrow()
            .nodes
            .iter()
            .find(|n| n.id == id)
            .and_then(|n| n.menu_item.clone());
        let Some(info) = info else { return Ok(None) };
        let Some(menu) = self.current_menu() else {
            return Ok(Some(false));
        };
        let Some(item) = find(&menu.items, &info.path).cloned() else {
            return Ok(Some(false));
        };
        if let Action::Submenu(_) = &item.action {
            self.focus(id)?;
            menu.model.path.set(info.path.clone());
            self.menu_focus(info.path.len(), false)?;
        } else {
            if let Some(id) = self.active_modal() {
                self.dismiss_overlay(id, crate::DismissReason::Action)?;
            } else {
                menu.model.dismiss();
            }
            if let Some(select) = menu.select {
                select(info.path);
            }
            item.invoke();
        }
        self.redraw();
        Ok(Some(true))
    }
    pub(crate) fn menu_key(&self, event: KeyEvent) -> Result<Option<bool>, Error> {
        let Some(menu) = self.current_menu() else {
            return Ok(None);
        };
        let focused = self.focused().and_then(|id| {
            self.0
                .mounted
                .borrow()
                .nodes
                .iter()
                .find(|n| n.id == id)
                .and_then(|n| n.menu_item.clone())
        });
        let depth = focused.as_ref().map_or(0, |m| m.path.len() - 1);
        let open_path = menu.model.path.get();
        if matches!(event.key, Key::Escape | Key::Left) && open_path.len() > depth {
            menu.model
                .path
                .set(open_path[..open_path.len() - 1].to_vec());
            menu.model.typing.replace(Typeahead::default());
            self.refresh()?;
            if let Some(n) = self
                .menu_rows(open_path.len() - 1)
                .into_iter()
                .find(|n| n.menu_item.as_ref().unwrap().path == open_path)
            {
                self.focus(n.id)?;
            }
            return Ok(Some(true));
        }
        let rows = self.menu_rows(depth);
        let current = self
            .focused()
            .and_then(|id| rows.iter().position(|n| n.id == id));
        match event.key {
            Key::Up | Key::Down | Key::Home | Key::End if !rows.is_empty() => {
                let index = match event.key {
                    Key::Home => 0,
                    Key::End => rows.len() - 1,
                    Key::Up => {
                        current.map_or(rows.len() - 1, |i| (i + rows.len() - 1) % rows.len())
                    }
                    _ => current.map_or(0, |i| (i + 1) % rows.len()),
                };
                // Moving at a parent level closes its old submenu.
                let path = rows[index].menu_item.as_ref().unwrap().path[..depth].to_vec();
                menu.model.path.set(path);
                self.focus(rows[index].id)?;
            }
            Key::Right => {
                if let Some(info) = focused
                    && info.submenu
                {
                    self.activate_menu_item(self.focused().unwrap())?;
                }
            }
            Key::Left | Key::Escape if depth > 0 => {
                let path = focused.unwrap().path[..depth].to_vec();
                menu.model.path.set(path[..depth - 1].to_vec());
                self.refresh()?;
                if let Some(n) = self
                    .menu_rows(depth - 1)
                    .into_iter()
                    .find(|n| n.menu_item.as_ref().unwrap().path == path)
                {
                    self.focus(n.id)?;
                }
            }
            Key::Escape => {
                if menu.model.options.dismiss_on_escape
                    && let Some(id) = self.active_modal()
                {
                    self.dismiss_overlay(id, crate::DismissReason::Escape)?;
                }
            }
            Key::Tab => {
                if let Some(id) = self.active_modal() {
                    self.dismiss_overlay(id, crate::DismissReason::Escape)?;
                }
            }
            Key::Enter | Key::Space => {
                if !event.repeat
                    && let Some(id) = self.focused()
                {
                    self.activate_menu_item(id)?;
                }
            }
            _ => return Ok(Some(false)),
        }
        menu.model.typing.replace(Typeahead::default());
        self.redraw();
        Ok(Some(true))
    }
    /// Committed printable input for an open menu. No editor/IME or clipboard is
    /// created. Repeated letters cycle; a one-second pause starts a new prefix.
    pub fn menu_text(&self, text: &str) -> Result<bool, Error> {
        self.menu_text_at(text, Instant::now())
    }
    pub fn menu_text_at(&self, text: &str, now: Instant) -> Result<bool, Error> {
        self.refresh()?;
        let Some(menu) = self.current_menu() else {
            return Ok(false);
        };
        if text.is_empty() || text.chars().any(char::is_control) {
            return Ok(false);
        }
        let focused = self.focused();
        let depth = focused
            .and_then(|id| {
                self.0
                    .mounted
                    .borrow()
                    .nodes
                    .iter()
                    .find(|n| n.id == id)
                    .and_then(|n| n.menu_item.as_ref().map(|m| m.path.len() - 1))
            })
            .unwrap_or(0);
        let rows = self.menu_rows(depth);
        let input = text.to_lowercase();
        let (query, cycle) = {
            let mut typing = menu.model.typing.borrow_mut();
            if typing
                .at
                .is_none_or(|at| now.saturating_duration_since(at) > Duration::from_secs(1))
            {
                typing.query.clear();
            }
            let cycle = typing.query == input;
            if !cycle {
                typing.query.push_str(&input);
            }
            typing.at = Some(now);
            (typing.query.clone(), cycle)
        };
        let start = focused
            .and_then(|id| rows.iter().position(|n| n.id == id))
            .map_or(0, |i| if cycle { i + 1 } else { i });
        if let Some(n) = (0..rows.len())
            .map(|i| &rows[(start + i) % rows.len()])
            .find(|n| n.text.to_lowercase().starts_with(&query))
        {
            menu.model
                .path
                .set(n.menu_item.as_ref().unwrap().path[..depth].to_vec());
            self.focus(n.id)?;
        }
        self.redraw();
        Ok(true)
    }
    pub(crate) fn menu_pointer(&self, event: Pointer) -> Result<bool, Error> {
        let Some(menu) = self.current_menu() else {
            return Ok(false);
        };
        if !self.0.interaction.borrow().active {
            return Ok(false);
        }
        if event.kind == PointerKind::Cancel {
            menu.model.dismiss();
            return Ok(true);
        }
        let dragging = self.0.interaction.borrow().pressed.is_some_and(|id| {
            self.0
                .mounted
                .borrow()
                .nodes
                .iter()
                .any(|n| n.id == id && n.menu_item.is_some())
        });
        if event.kind == PointerKind::Up && dragging && self.hit_test(event.position)?.is_none() {
            self.0.interaction.borrow_mut().pressed = None;
            menu.model.dismiss();
            self.redraw();
            return Ok(true);
        }
        if event.kind == PointerKind::Move
            && let Some(id) = self.hit_test(event.position)?
        {
            let info = self
                .0
                .mounted
                .borrow()
                .nodes
                .iter()
                .find(|n| n.id == id)
                .and_then(|n| n.menu_item.clone());
            if let Some(info) = info {
                let mut path = info.path.clone();
                if !info.submenu {
                    path.pop();
                }
                menu.model.path.set(path);
                self.focus(id)?;
                if dragging {
                    self.0.interaction.borrow_mut().pressed = Some(id);
                }
            }
        }
        Ok(false)
    }
    pub(crate) fn reconcile_menu(&self) -> Result<(), Error> {
        let Some(menu) = self.current_menu() else {
            return Ok(());
        };
        let anchor_valid = {
            let m = self.0.mounted.borrow();
            self.active_modal()
                .and_then(|id| m.nodes.iter().find(|n| n.id == id))
                .is_some_and(|root| {
                    root.overlay_owner
                        .and_then(|id| m.nodes.iter().find(|n| n.id == id))
                        .or_else(|| {
                            root.anchor.as_ref().and_then(|key| {
                                m.nodes.iter().find(|n| n.key.as_ref() == Some(key))
                            })
                        })
                        .is_some_and(|n| n.enabled && !n.hidden && !n.layout_hidden)
                })
        };
        if !anchor_valid {
            if let Some(id) = self.active_modal() {
                self.dismiss_overlay(id, crate::DismissReason::OwnerRemoved)?;
            }
            return Ok(());
        }
        if !self.0.interaction.borrow().active {
            return Ok(());
        }
        if menu.model.options.focus == crate::OverlayFocus::Preserve
            && self.focused().is_some_and(|id| {
                self.in_scope(id)
                    && !self
                        .0
                        .mounted
                        .borrow()
                        .nodes
                        .iter()
                        .any(|n| n.id == id && n.menu_item.is_some())
            })
        {
            return Ok(());
        }
        let path = menu.model.path.get();
        let valid = (0..path.len())
            .take_while(|i| {
                find(&menu.items, &path[..=*i])
                    .is_some_and(|a| matches!(a.action, Action::Submenu(_)))
            })
            .count();
        if valid != path.len() {
            menu.model.path.set(path[..valid].to_vec());
        }
        let focused_valid = self.focused().is_some_and(|id| {
            self.menu_rows(valid).iter().any(|n| n.id == id)
                || (0..valid).any(|depth| self.menu_rows(depth).iter().any(|n| n.id == id))
        });
        if !focused_valid {
            self.menu_focus(valid, false)?;
        }
        Ok(())
    }
}
