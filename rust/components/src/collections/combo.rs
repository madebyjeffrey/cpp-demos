//! An editable query, committed choice and virtual suggestion list in one model.
use crate::{Binding, Error, ListItem, ListModel, ListOptions, OptionItem, State, View};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ComboBoxMode {
    #[default]
    Selection,
    FreeText,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ComboBoxFilter {
    #[default]
    Contains,
    Prefix,
    None,
}
#[derive(Clone, Debug, PartialEq)]
pub struct ComboBoxOptions {
    pub mode: ComboBoxMode,
    pub filter: ComboBoxFilter,
    pub loading: bool,
    pub read_only: bool,
    pub auto_highlight: bool,
    pub max_length: usize,
    pub max_height: f64,
    pub row_height: f64,
    pub no_results: String,
    pub loading_text: String,
}
impl Default for ComboBoxOptions {
    fn default() -> Self {
        Self {
            mode: Default::default(),
            filter: Default::default(),
            loading: false,
            read_only: false,
            auto_highlight: true,
            max_length: 0,
            max_height: 240.,
            row_height: 32.,
            no_results: "No matches".into(),
            loading_text: "Loading…".into(),
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ComboAccept {
    pub selection: Option<String>,
    pub text: String,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ComboInfo {
    pub query: String,
    pub selection: Option<String>,
    pub highlighted: Option<String>,
    pub matches: usize,
    pub open: bool,
    pub loading: bool,
    pub read_only: bool,
}
#[derive(Clone, PartialEq)]
struct Session {
    generation: u64,
    query: String,
}
#[derive(Clone, PartialEq)]
struct Snapshot {
    query: String,
    selection: Option<String>,
    session: Option<Session>,
    loading: bool,
    read_only: bool,
}
type QueryHandler = Rc<dyn Fn(String)>;
type AcceptHandler = Rc<dyn Fn(ComboAccept)>;
struct Inner {
    id: u64,
    options: ComboBoxOptions,
    items: State<Rc<Vec<OptionItem>>>,
    folded: RefCell<Vec<String>>,
    state: State<Snapshot>,
    list: ListModel,
    query: RefCell<Option<QueryHandler>>,
    accept: RefCell<Option<AcceptHandler>>,
    cancel: RefCell<Option<Rc<dyn Fn()>>>,
    notifying: Cell<bool>,
}
#[derive(Clone)]
pub struct ComboBox(Rc<Inner>);
impl ComboBox {
    pub fn new(items: Vec<OptionItem>) -> Result<Self, Error> {
        Self::with_options(items, Default::default())
    }
    pub fn with_options(items: Vec<OptionItem>, options: ComboBoxOptions) -> Result<Self, Error> {
        if !options.row_height.is_finite()
            || options.row_height <= 0.
            || !options.max_height.is_finite()
            || options.max_height < options.row_height
        {
            return Err(Error(
                "Combo row height must be positive and fit within a finite maximum height".into(),
            ));
        }
        validate(&items)?;
        if !(items.len() as f64 * options.row_height).is_finite() {
            return Err(Error("Combo extent must be finite".into()));
        }
        let list = ListModel::new(vec![], options.row_height).options(ListOptions {
            visible_rows: (options.max_height / options.row_height).floor() as usize,
            ..Default::default()
        })?;
        let result = Self(Rc::new(Inner {
            id: super::list::next_token(),
            folded: RefCell::new(items.iter().map(|i| i.label.to_lowercase()).collect()),
            items: State::new(Rc::new(items)),
            state: State::new(Snapshot {
                query: String::new(),
                selection: None,
                session: None,
                loading: options.loading,
                read_only: options.read_only,
            }),
            options,
            list,
            query: RefCell::new(None),
            accept: RefCell::new(None),
            cancel: RefCell::new(None),
            notifying: Cell::new(false),
        }));
        result.rebuild()?;
        Ok(result)
    }
    pub fn info(&self) -> ComboInfo {
        let s = self.0.state.get();
        ComboInfo {
            query: s.query,
            selection: s.selection,
            highlighted: self.0.list.cursor(),
            matches: self.0.list.len(),
            open: s.session.is_some(),
            loading: s.loading,
            read_only: s.read_only,
        }
    }
    pub fn query(&self) -> String {
        self.0.state.get().query
    }
    pub fn selection(&self) -> Option<String> {
        self.0.state.get().selection
    }
    pub fn highlighted(&self) -> Option<String> {
        self.0.list.cursor()
    }
    pub fn is_open(&self) -> bool {
        self.0.state.get().session.is_some()
    }
    pub fn options(&self) -> ComboBoxOptions {
        let mut o = self.0.options.clone();
        let s = self.0.state.get();
        o.loading = s.loading;
        o.read_only = s.read_only;
        o
    }
    pub fn set_items(&self, items: Vec<OptionItem>) -> Result<(), Error> {
        crate::state::check_write();
        validate(&items)?;
        if !(items.len() as f64 * self.0.options.row_height).is_finite() {
            return Err(Error("Combo extent must be finite".into()));
        }
        let folded = items.iter().map(|i| i.label.to_lowercase()).collect();
        let mut s = self.0.state.get();
        if s.selection
            .as_ref()
            .is_some_and(|key| !items.iter().any(|i| &i.id == key && i.enabled))
        {
            s.selection = None;
        }
        crate::state::batch(|| {
            self.0.items.set(Rc::new(items));
            *self.0.folded.borrow_mut() = folded;
            self.0.state.set(s);
            self.rebuild()
        })
    }
    /// Replace the draft without emitting a user query or changing the choice.
    pub fn set_query(&self, query: impl Into<String>) -> Result<(), Error> {
        let mut s = self.0.state.get();
        s.query = query.into();
        crate::state::batch(|| {
            self.0.state.set(s);
            self.rebuild()
        })
    }
    pub fn set_selection(&self, key: Option<&str>) -> Result<(), Error> {
        let mut s = self.0.state.get();
        s.query = match key {
            Some(key) => self
                .0
                .items
                .get()
                .iter()
                .find(|i| i.id == key && i.enabled)
                .ok_or_else(|| Error("Combo choice is missing or disabled".into()))?
                .label
                .clone(),
            None => String::new(),
        };
        s.selection = key.map(str::to_owned);
        s.session = None;
        crate::state::batch(|| {
            self.0.state.set(s);
            self.rebuild()
        })
    }
    pub fn set_loading(&self, loading: bool) {
        let mut s = self.0.state.get();
        s.loading = loading;
        self.0.state.set(s);
    }
    pub fn set_read_only(&self, read_only: bool) {
        let mut s = self.0.state.get();
        s.read_only = read_only;
        self.0.state.set(s);
        if read_only {
            self.cancel();
        }
    }
    fn rebuild(&self) -> Result<(), Error> {
        let s = self.0.state.get();
        let query = s.query.to_lowercase();
        let items = self.0.items.get();
        let rows = items
            .iter()
            .zip(self.0.folded.borrow().iter())
            .filter(|(_, folded)| match self.0.options.filter {
                ComboBoxFilter::Contains => folded.contains(&query),
                ComboBoxFilter::Prefix => folded.starts_with(&query),
                ComboBoxFilter::None => true,
            })
            .map(|(i, _)| ListItem::new(&i.id, &i.label).enabled(i.enabled))
            .collect();
        let old = self.0.list.cursor();
        self.0.list.set_items(rows)?;
        let key = old
            .filter(|key| {
                self.0
                    .list
                    .index_of(key)
                    .and_then(|i| self.0.list.item(i))
                    .is_some_and(|i| i.enabled)
            })
            .or_else(|| {
                self.0
                    .options
                    .auto_highlight
                    .then(|| {
                        (0..self.0.list.len())
                            .find_map(|i| self.0.list.item(i).filter(|i| i.enabled).map(|i| i.key))
                    })
                    .flatten()
            });
        self.0.list.clear_selection();
        if let Some(key) = key {
            self.0.list.select(&key, false);
            if let Some(index) = self.0.list.index_of(&key) {
                self.0.list.reveal(index);
            }
        } else {
            self.0
                .list
                .restore_selection(Default::default(), None, None);
        }
        Ok(())
    }
    pub fn show(&self) -> bool {
        let mut s = self.0.state.get();
        if s.read_only || s.session.is_some() {
            return false;
        }
        s.session = Some(Session {
            generation: super::list::next_token(),
            query: s.query.clone(),
        });
        self.0.state.set(s);
        true
    }
    pub fn cancel(&self) -> bool {
        let mut s = self.0.state.get();
        let Some(session) = s.session.take() else {
            return false;
        };
        s.query = session.query;
        crate::state::batch(|| {
            self.0.state.set(s);
            self.rebuild().expect("Validated combo geometry");
        });
        let callback = self.0.cancel.borrow().clone();
        self.notify(|| {
            if let Some(callback) = callback {
                callback();
            }
        });
        true
    }
    pub fn accept(&self) -> bool {
        if let Some(key) = self.highlighted() {
            self.accept_key(&key)
        } else if self.0.options.mode == ComboBoxMode::FreeText {
            self.accept_value(None, self.query())
        } else {
            false
        }
    }
    pub fn accept_key(&self, key: &str) -> bool {
        let Some(item) = self
            .0
            .list
            .index_of(key)
            .and_then(|i| self.0.list.item(i))
            .filter(|i| i.enabled)
        else {
            return false;
        };
        let text = item.label();
        self.accept_value(Some(item.key), text)
    }
    fn accept_value(&self, selection: Option<String>, text: String) -> bool {
        let mut s = self.0.state.get();
        if s.session.is_none() || s.loading || s.read_only {
            return false;
        }
        s.session = None;
        s.query = text.clone();
        s.selection = selection.clone();
        crate::state::batch(|| {
            self.0.state.set(s);
            self.rebuild().expect("Validated combo geometry");
        });
        let callback = self.0.accept.borrow().clone();
        self.notify(|| {
            if let Some(callback) = callback {
                callback(ComboAccept { selection, text });
            }
        });
        true
    }
    fn edit(&self, text: String) {
        if self.info().read_only || self.query() == text {
            return;
        }
        crate::state::batch(|| {
            self.show();
            self.set_query(text.clone())
                .expect("Validated combo geometry");
        });
        let callback = self.0.query.borrow().clone();
        self.notify(|| {
            if let Some(callback) = callback {
                callback(text);
            }
        });
    }
    fn notify(&self, action: impl FnOnce()) {
        if self.0.notifying.replace(true) {
            return;
        }
        struct Guard<'a>(&'a Cell<bool>);
        impl Drop for Guard<'_> {
            fn drop(&mut self) {
                self.0.set(false);
            }
        }
        let _guard = Guard(&self.0.notifying);
        action();
    }
    pub fn on_query(self, handler: impl Fn(String) + 'static) -> Self {
        crate::state::check_write();
        let old = self.0.query.replace(Some(Rc::new(handler)));
        drop(old);
        self
    }
    pub fn on_query_with_model(self, handler: impl Fn(&Self, String) + 'static) -> Self {
        let weak = Rc::downgrade(&self.0);
        self.on_query(move |q| {
            if let Some(owner) = weak.upgrade() {
                handler(&Self(owner), q)
            }
        })
    }
    pub fn on_accept(self, handler: impl Fn(ComboAccept) + 'static) -> Self {
        crate::state::check_write();
        let old = self.0.accept.replace(Some(Rc::new(handler)));
        drop(old);
        self
    }
    pub fn on_cancel(self, handler: impl Fn() + 'static) -> Self {
        crate::state::check_write();
        let old = self.0.cancel.replace(Some(Rc::new(handler)));
        drop(old);
        self
    }
    pub(crate) fn generation(&self) -> Option<u64> {
        self.0.state.get().session.map(|s| s.generation)
    }
    pub(crate) fn field_key(&self) -> String {
        format!("$combo-{}", self.0.id)
    }
    pub(crate) fn navigate(&self, key: crate::Key) -> bool {
        if self.info().read_only {
            return false;
        }
        let opened = self.show();
        if opened && key == crate::Key::Down && self.highlighted().is_some() {
            return true;
        }
        if self.info().loading {
            return true;
        }
        use crate::Key;
        let len = self.0.list.len();
        if len == 0 {
            return true;
        }
        let current = self.highlighted().and_then(|k| self.0.list.index_of(&k));
        let step = (self.0.options.max_height / self.0.options.row_height).floor() as usize;
        let start = match key {
            Key::Home => 0,
            Key::End => len - 1,
            Key::PageDown => current.map_or(0, |i| i.saturating_add(step).min(len - 1)),
            Key::PageUp => current.map_or(len - 1, |i| i.saturating_sub(step)),
            Key::Up if opened => len - 1,
            Key::Up => current.map_or(len - 1, |i| i.saturating_sub(1)),
            _ => current.map_or(0, |i| (i + 1).min(len - 1)),
        };
        let forward = !matches!(key, Key::Up | Key::PageUp | Key::End);
        let index = if forward {
            (start..len).find(|i| self.0.list.item(*i).is_some_and(|i| i.enabled))
        } else {
            (0..=start)
                .rev()
                .find(|i| self.0.list.item(*i).is_some_and(|i| i.enabled))
        };
        if let Some(index) = index {
            let item = self.0.list.item(index).unwrap();
            self.0.list.select(&item.key, false);
            self.0.list.reveal(index);
        }
        true
    }
    /// One field and an owned portal; application code needs no popup wiring.
    pub fn view(&self, placeholder: impl Into<String>) -> View {
        let read = Rc::downgrade(&self.0);
        let write = read.clone();
        let binding = Binding::new(
            move || {
                read.upgrade()
                    .map_or(String::new(), |i| i.state.get().query)
            },
            move |text| {
                if let Some(i) = write.upgrade() {
                    Self(i).edit(text)
                }
            },
        );
        let mut field = View::text_field(placeholder, binding)
            .grow(1.)
            .key(self.field_key())
            .read_only(self.info().read_only)
            .max_length(self.0.options.max_length);
        let close = Rc::downgrade(&self.0);
        let d = Rc::make_mut(&mut field.0);
        d.combo = Some(self.clone());
        d.blur = Some(Rc::new(move || {
            if let Some(i) = close.upgrade() {
                Self(i).cancel();
            }
        }));
        let mut toggle = View::button("▾", || {})
            .width(34)
            .accessible_label("Show choices");
        Rc::make_mut(&mut toggle.0).combo_action = Some(self.clone());
        let mut children = vec![field, toggle];
        if let Some(generation) = self.generation() {
            let info = self.info();
            let mut body = if info.loading {
                View::label(&self.0.options.loading_text).padding(8.)
            } else if info.matches == 0 {
                View::label(&self.0.options.no_results).padding(8.)
            } else {
                let mut list = self
                    .0
                    .list
                    .view()
                    .height(
                        (info.matches as f64 * self.0.options.row_height)
                            .min(self.0.options.max_height),
                    )
                    .gap(0.);
                Rc::make_mut(&mut list.0).combo_popup = Some(self.clone());
                list
            };
            Rc::make_mut(&mut body.0).combo_popup = Some(self.clone());
            let weak = Rc::downgrade(&self.0);
            let shown = weak.clone();
            let mut portal = View::popover(self.field_key(), vec![body], || {})
                .key(format!("$combo-popup-{}-{generation}", self.0.id));
            let d = Rc::make_mut(&mut portal.0);
            d.overlay_parent = true;
            d.combo_popup = Some(self.clone());
            d.overlay = Some(crate::overlay::OverlayData {
                options: crate::OverlayOptions {
                    focus: crate::OverlayFocus::Preserve,
                    max_height: self.0.options.max_height,
                    ..Default::default()
                },
                point: None,
                presented: Some(Rc::new(move || {
                    shown
                        .upgrade()
                        .is_some_and(|i| Self(i).generation() == Some(generation))
                })),
                close: Rc::new(move |_| {
                    if let Some(i) = weak.upgrade() {
                        let model = Self(i);
                        if model.generation() == Some(generation) {
                            model.cancel();
                        }
                    }
                }),
            });
            children.push(portal);
        }
        View::row(children).gap(0.)
    }
}
fn validate(items: &[OptionItem]) -> Result<(), Error> {
    let mut keys = std::collections::HashSet::new();
    if items
        .iter()
        .any(|i| i.id.trim().is_empty() || !keys.insert(&i.id))
    {
        return Err(Error("Combo option IDs must be nonempty and unique".into()));
    }
    Ok(())
}
