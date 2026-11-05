//! Fixed-height virtual collections with stable application keys.
use crate::{Binding, State, View};
use std::{
    cell::RefCell,
    collections::{BTreeSet, HashMap},
    rc::Rc,
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
    pub fn label(&self) -> String {
        self.cells.join(" · ")
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
struct Viewport {
    offset: f64,
    height: f64,
}
struct Model {
    items: State<Rc<Vec<ListItem>>>,
    index: RefCell<HashMap<String, usize>>,
    pub selected: State<BTreeSet<String>>,
    viewport: State<Viewport>,
    height: f64,
    multiple: bool,
    tokens: RefCell<HashMap<String, u64>>,
}
#[derive(Clone)]
pub struct ListModel(Rc<Model>);
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
            multiple: false,
            tokens: RefCell::new(HashMap::new()),
        }));
        model.set_items(items).expect("Unique collection keys");
        model
    }
    pub fn multiple(mut self) -> Self {
        Rc::get_mut(&mut self.0)
            .expect("Set selection mode before cloning")
            .multiple = true;
        self
    }
    pub fn set_items(&self, items: Vec<ListItem>) -> Result<(), crate::Error> {
        let mut index = HashMap::new();
        for (i, item) in items.iter().enumerate() {
            if item.key.is_empty() || index.insert(item.key.clone(), i).is_some() {
                return Err(crate::Error(
                    "Collection keys must be nonempty and unique".into(),
                ));
            }
        }
        let selected = self
            .0
            .selected
            .get()
            .into_iter()
            .filter(|key| index.contains_key(key))
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
        *self.0.index.borrow_mut() = index;
        self.0.items.set(Rc::new(items));
        self.0.selected.set(selected);
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
    pub fn selected(&self) -> BTreeSet<String> {
        self.0.selected.get()
    }
    pub fn select(&self, key: &str, extend: bool) -> bool {
        let Some(index) = self.index_of(key) else {
            return false;
        };
        if !self.item(index).is_some_and(|v| v.enabled) {
            return false;
        }
        self.0.selected.update(|selected| {
            if !self.0.multiple || !extend {
                selected.clear();
            }
            if !selected.insert(key.to_owned()) {
                selected.remove(key);
            }
        });
        true
    }
    pub fn item_token(&self, key: &str) -> Option<u64> {
        self.0.tokens.borrow().get(key).copied()
    }
    pub fn row_height(&self) -> f64 {
        self.0.height
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
        start.saturating_sub(2).min(count)..end.saturating_add(2).min(count)
    }
    pub fn view(&self) -> View {
        self.view_with(|item| View::label(item.label()))
    }
    pub fn view_with(&self, row: impl Fn(&ListItem) -> View) -> View {
        let items = self.0.items.get();
        let range = self.visible_range();
        let _selected = self.selected();
        let mut children = vec![
            View::spacer()
                .grow(0.)
                .height(range.start as f64 * self.0.height)
                .shrink(0.),
        ];
        for item in &items[range.clone()] {
            let model = self.clone();
            let key = item.key.clone();
            let select = self.clone();
            let target = key.clone();
            let row = View::column(vec![row(item)])
                .height(self.0.height)
                .padding(6.)
                .key(&key)
                .enabled(item.enabled);
            let mut row = row.on_context(move || {
                select.select(&target, true);
            });
            // The row itself participates in standard keyboard/pointer button handling.
            let read = self.clone();
            let write = self.clone();
            let read_key = key.clone();
            let write_key = key.clone();
            let d = Rc::make_mut(&mut row.0);
            d.kind = crate::Kind::CollectionRow;
            d.data = crate::view::Data::Bool(Binding::new(
                move || read.selected().contains(&read_key),
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
                .height((items.len() - range.end) as f64 * self.0.height)
                .shrink(0.),
        );
        let mut view = View::scroll(children).gap(0.);
        Rc::make_mut(&mut view.0).collection = Some(self.clone());
        view
    }
}
/// Editable combo box: the same text binding remains usable for values outside the options.
pub fn combo_box(
    text: Binding<String>,
    options: &[String],
    popup: &crate::Popup,
    key: &str,
) -> (View, View) {
    let show = popup.clone();
    let field = View::row(vec![
        View::text_field("Type or choose…", text.clone()).grow(1.),
        View::button("▾", move || show.toggle()).width(34),
    ])
    .key(key);
    let query = text.get().to_lowercase();
    let items = options
        .iter()
        .enumerate()
        .filter(|(_, s)| s.to_lowercase().contains(&query))
        .map(|(index, option)| {
            let value = option.clone();
            let text = text.clone();
            crate::MenuAction::new(option, move || text.set(value.clone())).key(index.to_string())
        })
        .take(100)
        .collect();
    (field, popup.menu(key, items))
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TreeItem {
    pub key: String,
    pub label: String,
    pub children: Vec<TreeItem>,
}
#[derive(Clone)]
pub struct TreeModel {
    pub expanded: State<BTreeSet<String>>,
    pub list: ListModel,
    roots: Rc<RefCell<Vec<TreeItem>>>,
}
impl TreeModel {
    pub fn new() -> Self {
        Self {
            expanded: State::new(BTreeSet::new()),
            list: ListModel::new(vec![], 34.),
            roots: Rc::new(RefCell::new(Vec::new())),
        }
    }
    /// Flatten only expanded branches. Applications can fetch lazy children before calling this.
    pub fn set_tree(&self, roots: &[TreeItem]) -> Result<(), crate::Error> {
        fn visit(
            items: &[TreeItem],
            expanded: &BTreeSet<String>,
            depth: usize,
            out: &mut Vec<ListItem>,
        ) {
            for item in items {
                let marker = if item.children.is_empty() {
                    "  "
                } else if expanded.contains(&item.key) {
                    "▾ "
                } else {
                    "▸ "
                };
                out.push(ListItem::new(
                    &item.key,
                    format!("{}{}{}", "  ".repeat(depth), marker, item.label),
                ));
                if expanded.contains(&item.key) {
                    visit(&item.children, expanded, depth + 1, out);
                }
            }
        }
        *self.roots.borrow_mut() = roots.to_vec();
        let mut rows = Vec::new();
        visit(roots, &self.expanded.get(), 0, &mut rows);
        self.list.set_items(rows)
    }
    pub fn toggle(&self, key: &str) {
        self.expanded.update(|keys| {
            if !keys.insert(key.to_owned()) {
                keys.remove(key);
            }
        });
        let roots = self.roots.borrow().clone();
        let _ = self.set_tree(&roots);
    }
    pub fn view(&self) -> View {
        let tree = self.clone();
        self.list.view_with(move |item| {
            let key = item.key.clone();
            let tree = tree.clone();
            View::button(item.label(), move || tree.toggle(&key))
        })
    }
}
impl Default for TreeModel {
    fn default() -> Self {
        Self::new()
    }
}
#[derive(Clone)]
pub struct TableModel {
    pub rows: ListModel,
    pub headers: Vec<String>,
}
#[derive(Clone, Debug, PartialEq)]
pub struct CellEdit {
    pub row: String,
    pub column: usize,
    pub original: String,
    pub text: String,
}
impl TableModel {
    pub fn new(headers: Vec<String>, rows: Vec<ListItem>) -> Self {
        Self {
            rows: ListModel::new(rows, 38.),
            headers,
        }
    }
    pub fn sort_by(&self, column: usize, descending: bool) {
        let mut rows = (*self.rows.0.items.get()).clone();
        rows.sort_by(|a, b| {
            let cmp = a.cells.get(column).cmp(&b.cells.get(column));
            if descending { cmp.reverse() } else { cmp }
        });
        self.rows.set_items(rows).unwrap();
    }
    pub fn begin_edit(&self, key: &str, column: usize) -> Option<CellEdit> {
        let row = self.rows.item(self.rows.index_of(key)?)?;
        let text = row.cells.get(column)?.clone();
        Some(CellEdit {
            row: key.into(),
            column,
            original: text.clone(),
            text,
        })
    }
    pub fn commit(&self, edit: CellEdit) -> Result<(), crate::Error> {
        let index = self
            .rows
            .index_of(&edit.row)
            .ok_or_else(|| crate::Error("Row was removed".into()))?;
        let mut rows = (*self.rows.0.items.get()).clone();
        let cell = rows[index]
            .cells
            .get_mut(edit.column)
            .ok_or_else(|| crate::Error("Column was removed".into()))?;
        if *cell != edit.original {
            return Err(crate::Error("Cell changed while editing".into()));
        }
        *cell = edit.text;
        self.rows.set_items(rows)
    }
    pub fn view(&self) -> View {
        let headers = self
            .headers
            .iter()
            .enumerate()
            .map(|(column, label)| {
                let model = self.clone();
                View::button(label, move || model.sort_by(column, false))
                    .grow(1.)
                    .width(crate::percent(100. / self.headers.len().max(1) as f64))
            })
            .collect();
        View::column(vec![
            View::row(headers),
            self.rows
                .view_with(|item| {
                    View::row(
                        item.cells
                            .iter()
                            .map(|cell| {
                                View::label(cell)
                                    .grow(1.)
                                    .width(crate::percent(100. / item.cells.len().max(1) as f64))
                            })
                            .collect(),
                    )
                })
                .grow(1.),
        ])
    }
}
