//! Fixed-height virtual collections with stable application keys.
use crate::{Binding, State, View};
use std::{
    cell::{Cell, RefCell},
    collections::{BTreeSet, HashMap},
    rc::Rc,
    time::{Duration, Instant},
};
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ListItem {
    pub key: String,
    pub cells: Vec<String>,
    pub enabled: bool,
}
impl ListItem {
    pub fn new(key: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            cells: vec![label.into()],
            enabled: true,
        }
    }
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
    pub fn label(&self) -> String {
        self.cells.join(" · ")
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
struct Viewport {
    offset: f64,
    height: f64,
}
type Activation = Rc<dyn Fn(String)>;
pub(super) struct Model {
    pub(super) items: State<Rc<Vec<ListItem>>>,
    index: RefCell<HashMap<String, usize>>,
    pub selected: State<BTreeSet<String>>,
    viewport: State<Viewport>,
    height: f64,
    options: State<ListOptions>,
    anchor: State<Option<String>>,
    cursor: State<Option<String>>,
    typing: RefCell<(String, Option<Instant>)>,
    activate: RefCell<Option<Activation>>,
    activating: Cell<bool>,
    revision: Cell<u64>,
    tokens: RefCell<HashMap<String, u64>>,
    pub(super) pins: State<std::collections::BTreeMap<String, u64>>,
    pub(super) leases: RefCell<HashMap<String, std::rc::Weak<super::pin::Lease>>>,
}
#[derive(Clone)]
pub struct ListModel(pub(super) Rc<Model>);
impl PartialEq for ListModel {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}
impl Eq for ListModel {}
impl ListModel {
    pub fn new(items: Vec<ListItem>, row_height: f64) -> Self {
        assert!(
            row_height.is_finite() && row_height > 0.,
            "Invalid row height"
        );
        let model = Self(Rc::new(Model {
            items: State::new(Rc::new(Vec::new())),
            index: RefCell::new(HashMap::new()),
            selected: State::new(BTreeSet::new()),
            viewport: State::new(Viewport {
                offset: 0.,
                height: 240.,
            }),
            height: row_height,
            options: State::new(ListOptions::default()),
            anchor: State::new(None),
            cursor: State::new(None),
            typing: RefCell::new((String::new(), None)),
            activate: RefCell::new(None),
            activating: Cell::new(false),
            revision: Cell::new(0),
            tokens: RefCell::new(HashMap::new()),
            pins: State::new(Default::default()),
            leases: RefCell::new(HashMap::new()),
        }));
        model.set_items(items).expect("Unique collection keys");
        model
    }
    pub fn multiple(self) -> Self {
        let mut options = self.0.options.get();
        options.selection = ListSelectionMode::Multiple;
        self.set_options(options).expect("Valid list options");
        self
    }
    pub fn options(self, options: ListOptions) -> Result<Self, crate::Error> {
        self.set_options(options)?;
        Ok(self)
    }
    pub fn set_options(&self, options: ListOptions) -> Result<(), crate::Error> {
        crate::state::check_write();
        if options.max_pinned_rows < self.0.pins.with(|pins| pins.len()) {
            return Err(crate::Error(
                "Release pinned rows before reducing the retention limit".into(),
            ));
        }
        if options.visible_rows == 0
            || !(options.visible_rows as f64 * self.0.height).is_finite()
            || options.typeahead_timeout.is_zero()
            || Instant::now()
                .checked_add(options.typeahead_timeout)
                .is_none()
        {
            return Err(crate::Error(
                "List needs a finite positive preferred height and typeahead timeout".into(),
            ));
        }
        if options.selection == ListSelectionMode::Single {
            let keep = self
                .cursor()
                .filter(|k| self.is_selected(k))
                .or_else(|| self.selected().first().cloned());
            self.0.selected.set(keep.into_iter().collect());
        }
        self.0.options.set(options);
        Ok(())
    }
    pub fn list_options(&self) -> ListOptions {
        self.0.options.get()
    }
    pub fn on_activate(self, callback: impl Fn(String) + 'static) -> Self {
        crate::state::check_write();
        let old = self.0.activate.replace(Some(Rc::new(callback)));
        drop(old);
        self
    }
    /// Supply model context without a strong self-capture in an activation handler.
    pub fn on_activate_with_model(self, callback: impl Fn(&ListModel, String) + 'static) -> Self {
        let weak = Rc::downgrade(&self.0);
        self.on_activate(move |key| {
            if let Some(owner) = weak.upgrade() {
                callback(&ListModel(owner), key);
            }
        })
    }
    pub fn activate(&self, key: &str) -> bool {
        crate::state::check_write();
        if !self
            .index_of(key)
            .and_then(|i| self.item(i))
            .is_some_and(|item| item.enabled)
            || self.0.activating.replace(true)
        {
            return false;
        }
        struct Guard<'a>(&'a Cell<bool>);
        impl Drop for Guard<'_> {
            fn drop(&mut self) {
                self.0.set(false);
            }
        }
        let _guard = Guard(&self.0.activating);
        let callback = self.0.activate.borrow().clone();
        if let Some(callback) = callback {
            callback(key.into());
            true
        } else {
            false
        }
    }
    pub fn set_items(&self, items: Vec<ListItem>) -> Result<(), crate::Error> {
        crate::state::check_write();
        let previous_cursor = self.cursor().and_then(|k| self.index_of(&k));
        let mut index = HashMap::new();
        for (i, item) in items.iter().enumerate() {
            if item.key.is_empty() || index.insert(item.key.clone(), i).is_some() {
                return Err(crate::Error(
                    "Collection keys must be nonempty and unique".into(),
                ));
            }
        }
        if !(items.len() as f64 * self.0.height).is_finite() {
            return Err(crate::Error("Collection extent must be finite".into()));
        }
        let selected = self
            .0
            .selected
            .get()
            .into_iter()
            .filter(|key| index.get(key).is_some_and(|i| items[*i].enabled))
            .collect();
        {
            use std::sync::atomic::{AtomicU64, Ordering};
            static NEXT: AtomicU64 = AtomicU64::new(1);
            let mut tokens = self.0.tokens.borrow_mut();
            tokens.retain(|key, _| index.contains_key(key));
            for key in index.keys() {
                tokens
                    .entry(key.clone())
                    .or_insert_with(|| NEXT.fetch_add(1, Ordering::Relaxed));
            }
        }
        let cursor = self
            .cursor()
            .filter(|k| index.get(k).is_some_and(|i| items[*i].enabled))
            .or_else(|| {
                previous_cursor.and_then(|i| {
                    let start = i.min(items.len().saturating_sub(1));
                    (start..items.len())
                        .chain((0..start).rev())
                        .find(|i| items[*i].enabled)
                        .map(|i| items[i].key.clone())
                })
            });
        let anchor = self
            .anchor()
            .filter(|k| index.get(k).is_some_and(|i| items[*i].enabled));
        let pins = self
            .0
            .pins
            .get()
            .into_iter()
            .filter(|(key, _)| index.get(key).is_some_and(|i| items[*i].enabled))
            .collect();
        self.0
            .leases
            .borrow_mut()
            .retain(|key, _| index.get(key).is_some_and(|i| items[*i].enabled));
        *self.0.index.borrow_mut() = index;
        if *self.0.items.get() != items {
            self.0.revision.set(self.0.revision.get().wrapping_add(1));
        }
        self.0.items.set(Rc::new(items));
        self.0.pins.set(pins);
        self.0.selected.set(selected);
        self.0.cursor.set(cursor);
        self.0.anchor.set(anchor);
        *self.0.typing.borrow_mut() = (String::new(), None);
        let viewport = self.0.viewport.get();
        self.set_viewport(viewport.offset, viewport.height);
        Ok(())
    }
    pub fn len(&self) -> usize {
        self.0.items.get().len()
    }
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
    pub fn item(&self, index: usize) -> Option<ListItem> {
        self.0.items.get().get(index).cloned()
    }
    pub fn index_of(&self, key: &str) -> Option<usize> {
        self.0.index.borrow().get(key).copied()
    }
    pub fn selection_count(&self) -> usize {
        self.0.selected.with(|selected| selected.len())
    }
    pub fn selected(&self) -> BTreeSet<String> {
        self.0.selected.get()
    }
    pub fn cursor(&self) -> Option<String> {
        self.0.cursor.get()
    }
    pub fn anchor(&self) -> Option<String> {
        self.0.anchor.get()
    }
    pub fn is_selected(&self, key: &str) -> bool {
        self.0.selected.with(|selected| selected.contains(key))
    }
    /// Compatibility shorthand: `extend` toggles one item additively.
    pub fn select(&self, key: &str, extend: bool) -> bool {
        self.select_with(
            key,
            if extend {
                ListSelection::Toggle
            } else {
                ListSelection::Replace
            },
        )
    }
    pub fn select_with(&self, key: &str, operation: ListSelection) -> bool {
        crate::state::check_write();
        let Some(index) = self
            .index_of(key)
            .filter(|i| self.item(*i).is_some_and(|v| v.enabled))
        else {
            return false;
        };
        let multiple = self.0.options.get().selection == ListSelectionMode::Multiple;
        let operation = if !multiple && operation != ListSelection::Focus {
            ListSelection::Replace
        } else {
            operation
        };
        let anchor = self
            .anchor()
            .or_else(|| self.cursor())
            .unwrap_or_else(|| key.into());
        let anchor_index = self.index_of(&anchor).unwrap_or(index);
        let mut selected = self.selected();
        match operation {
            ListSelection::Replace => {
                selected.clear();
                selected.insert(key.into());
            }
            ListSelection::Toggle => {
                if !selected.insert(key.into()) {
                    selected.remove(key);
                }
            }
            ListSelection::Range | ListSelection::AddRange => {
                if operation == ListSelection::Range {
                    selected.clear();
                }
                let items = self.0.items.get();
                selected.extend(
                    items[anchor_index.min(index)..=anchor_index.max(index)]
                        .iter()
                        .filter(|item| item.enabled)
                        .map(|item| item.key.clone()),
                );
            }
            ListSelection::Focus => {}
        }
        if operation != ListSelection::Focus {
            self.0.selected.set(selected);
            self.0.anchor.set(Some(
                if matches!(operation, ListSelection::Range | ListSelection::AddRange) {
                    anchor
                } else {
                    key.into()
                },
            ));
        }
        self.0.cursor.set(Some(key.into()));
        true
    }
    pub fn select_all(&self) -> bool {
        crate::state::check_write();
        if self.0.options.get().selection != ListSelectionMode::Multiple {
            return false;
        }
        self.0.selected.set(
            self.0
                .items
                .get()
                .iter()
                .filter(|i| i.enabled)
                .map(|i| i.key.clone())
                .collect(),
        );
        true
    }
    pub fn clear_selection(&self) {
        self.0.selected.set(BTreeSet::new());
    }
    pub(crate) fn focus_key(&self, key: &str) {
        if self.anchor().is_none() {
            self.0.anchor.set(Some(key.into()));
        }
        self.0.cursor.set(Some(key.into()));
    }
    pub(crate) fn enabled_from(&self, start: usize, forward: bool) -> Option<usize> {
        let items = self.0.items.get();
        if forward {
            (start..items.len()).find(|i| items[*i].enabled)
        } else {
            (0..=start.min(items.len().checked_sub(1)?))
                .rev()
                .find(|i| items[*i].enabled)
        }
    }
    pub(crate) fn typeahead(&self, text: &str, now: Instant) -> Option<usize> {
        if text.is_empty() || text.chars().any(char::is_control) {
            return None;
        }
        let input = text.to_lowercase();
        let timeout = self.0.options.get().typeahead_timeout;
        let (query, cycle, new) = {
            let mut typing = self.0.typing.borrow_mut();
            if typing
                .1
                .is_none_or(|at| now.saturating_duration_since(at) > timeout)
            {
                typing.0.clear();
            }
            let new = typing.0.is_empty();
            let cycle = typing.0 == input;
            if !cycle {
                typing.0.push_str(&input);
            }
            typing.1 = Some(now);
            (typing.0.clone(), cycle, new)
        };
        let items = self.0.items.get();
        let start = self
            .cursor()
            .and_then(|k| self.index_of(&k))
            .map_or(0, |i| if cycle || new { i + 1 } else { i });
        (0..items.len())
            .map(|n| (start + n) % items.len())
            .find(|i| items[*i].enabled && items[*i].label().to_lowercase().starts_with(&query))
    }
    pub(crate) fn revision(&self) -> u64 {
        self.0.revision.get()
    }
    pub fn item_token(&self, key: &str) -> Option<u64> {
        self.0.tokens.borrow().get(key).copied()
    }
    pub fn row_height(&self) -> f64 {
        self.0.height
    }
    pub(crate) fn viewport_height(&self) -> f64 {
        self.0.viewport.get().height
    }
    pub fn offset(&self) -> f64 {
        self.0.viewport.get().offset
    }
    pub fn reveal(&self, index: usize) {
        let mut v = self.0.viewport.get();
        let y = index.min(self.len().saturating_sub(1)) as f64 * self.0.height;
        if y < v.offset {
            v.offset = y;
        } else if y + self.0.height > v.offset + v.height {
            v.offset = (y + self.0.height - v.height).max(0.);
        }
        self.set_viewport(v.offset, v.height);
    }
    pub(crate) fn set_viewport(&self, offset: f64, height: f64) {
        let max = (self.len() as f64 * self.0.height - height).max(0.);
        self.0.viewport.set(Viewport {
            offset: offset.clamp(0., max),
            height,
        });
    }
    pub fn visible_range(&self) -> std::ops::Range<usize> {
        let v = self.0.viewport.get();
        let count = self.len();
        let start = (v.offset / self.0.height).floor() as usize;
        let end = ((v.offset + v.height) / self.0.height).ceil() as usize;
        let overscan = self.0.options.get().overscan_rows;
        start.saturating_sub(overscan).min(count)..end.saturating_add(overscan).min(count)
    }
    pub fn view(&self) -> View {
        self.view_with(|item| View::label(item.label()))
    }
    pub fn view_with(&self, row: impl Fn(&ListItem) -> View) -> View {
        let items = self.0.items.get();
        let indices = self.realized_indices();
        self.0.selected.with(|_| ());
        let mut children = vec![];
        let mut end = 0;
        for index in indices {
            if index > end {
                children.push(
                    View::spacer()
                        .grow(0.)
                        .height((index - end) as f64 * self.0.height)
                        .shrink(0.),
                );
            }
            end = index + 1;
            let item = &items[index];
            let model = self.clone();
            let key = item.key.clone();
            let select = self.clone();
            let target = key.clone();
            let row = View::column(vec![row(item)])
                .height(self.0.height)
                .shrink(0.)
                .padding(6.)
                .key(&key)
                .enabled(item.enabled);
            let mut row = row.on_context(move || {
                if !select.is_selected(&target) {
                    select.select(&target, false);
                }
            });
            // The row itself participates in standard keyboard/pointer button handling.
            let read = self.clone();
            let write = self.clone();
            let read_key = key.clone();
            let write_key = key.clone();
            let d = Rc::make_mut(&mut row.0);
            d.kind = crate::Kind::CollectionRow;
            d.collection_row = Some((
                self.clone(),
                key.clone(),
                self.item_token(&key).unwrap(),
                self.revision(),
            ));
            d.data = crate::view::Data::Bool(Binding::new(
                move || read.is_selected(&read_key),
                move |_| {
                    write.select(&write_key, false);
                },
            ));
            d.action = Some(Rc::new(move || {
                model.select(&key, false);
            }));

            children.push(row);
        }
        children.push(
            View::spacer()
                .grow(0.)
                .height((items.len() - end) as f64 * self.0.height)
                .shrink(0.),
        );
        let mut view = View::scroll(children)
            .gap(0.)
            .height(self.0.options.get().visible_rows as f64 * self.0.height);
        Rc::make_mut(&mut view.0).collection = Some(self.clone());
        view
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ListSelectionMode {
    #[default]
    Single,
    Multiple,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ListSelection {
    Replace,
    Toggle,
    Range,
    AddRange,
    Focus,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ListOptions {
    pub selection: ListSelectionMode,
    pub visible_rows: usize,
    pub overscan_rows: usize,
    pub typeahead_timeout: Duration,
    /// Maximum distinct pinned rows, including pins currently in the viewport.
    pub max_pinned_rows: usize,
}
impl Default for ListOptions {
    fn default() -> Self {
        Self {
            selection: ListSelectionMode::Single,
            visible_rows: 5,
            overscan_rows: 2,
            typeahead_timeout: Duration::from_secs(1),
            max_pinned_rows: 1,
        }
    }
}
