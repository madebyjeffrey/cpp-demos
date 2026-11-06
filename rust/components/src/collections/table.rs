//! Keyed table data and columns, sharing the virtual list's row lifetimes.
use crate::{
    Align, Error, ListItem, ListModel, ListOptions, ListSelection, ListSelectionMode, State,
};
use std::{
    cell::{Cell, RefCell},
    collections::{BTreeMap, BTreeSet, HashMap, HashSet},
    rc::Rc,
};
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TableCell {
    pub row: String,
    pub column: String,
}
impl TableCell {
    pub fn new(row: impl Into<String>, column: impl Into<String>) -> Self {
        Self {
            row: row.into(),
            column: column.into(),
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TableSelection {
    pub rows: BTreeSet<String>,
    pub cells: BTreeSet<TableCell>,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TableSelectionMode {
    #[default]
    Rows,
    Cells,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SortOrder {
    #[default]
    Ascending,
    Descending,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SortDescriptor {
    pub column: String,
    pub order: SortOrder,
}
impl SortDescriptor {
    pub fn new(column: impl Into<String>, order: SortOrder) -> Self {
        Self {
            column: column.into(),
            order,
        }
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct TableColumnWidth {
    pub column: String,
    pub width: f64,
}
#[derive(Clone, Debug, PartialEq)]
pub struct TableColumn {
    pub id: String,
    pub title: String,
    pub width: f64,
    pub min_width: f64,
    pub max_width: f64,
    pub resizable: bool,
    pub sortable: bool,
    pub alignment: Align,
    pub(super) policy: Rc<super::table_edit::CellPolicy>,
}
impl TableColumn {
    pub fn new(id: impl Into<String>, title: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            width: 160.,
            min_width: 48.,
            max_width: 800.,
            resizable: true,
            sortable: true,
            alignment: Align::Start,
            policy: Rc::new(Default::default()),
        }
    }
    pub fn width(mut self, width: f64) -> Self {
        self.width = width;
        self
    }
    pub fn limits(mut self, min: f64, max: f64) -> Self {
        self.min_width = min;
        self.max_width = max;
        self
    }
    pub fn resizable(mut self, value: bool) -> Self {
        self.resizable = value;
        self
    }
    pub fn sortable(mut self, value: bool) -> Self {
        self.sortable = value;
        self
    }
    pub fn alignment(mut self, value: Align) -> Self {
        self.alignment = value;
        self
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TableRow {
    pub key: String,
    pub cells: BTreeMap<String, String>,
    pub enabled: bool,
}
impl TableRow {
    pub fn new(key: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            cells: BTreeMap::new(),
            enabled: true,
        }
    }
    pub fn cell(mut self, column: impl Into<String>, text: impl Into<String>) -> Self {
        self.cells.insert(column.into(), text.into());
        self
    }
    pub fn enabled(mut self, value: bool) -> Self {
        self.enabled = value;
        self
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TableOptions {
    pub row_height: f64,
    pub header_height: f64,
    pub visible_rows: usize,
    pub overscan_rows: usize,
    pub selection_mode: TableSelectionMode,
    pub multiple_selection: bool,
}
impl Default for TableOptions {
    fn default() -> Self {
        Self {
            row_height: 38.,
            header_height: 34.,
            visible_rows: 8,
            overscan_rows: 2,
            selection_mode: TableSelectionMode::Rows,
            multiple_selection: false,
        }
    }
}
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Column {
    pub value: TableColumn,
    pub token: u64,
    pub policy_version: u64,
}
#[derive(Clone, Default, PartialEq)]
struct Cells {
    selected: BTreeSet<TableCell>,
    cursor: Option<TableCell>,
    anchor: Option<TableCell>,
}
type SortHandler = Rc<dyn Fn(Vec<SortDescriptor>)>;
pub(super) struct TableData {
    pub list: ListModel,
    pub source: State<Rc<Vec<TableRow>>>,
    pub index: RefCell<HashMap<String, usize>>,
    pub columns: State<Rc<Vec<Column>>>,
    options: State<TableOptions>,
    sort: State<Vec<SortDescriptor>>,
    cells: State<Cells>,
    on_sort: RefCell<Option<SortHandler>>,
    notifying: Cell<bool>,
    pub active: State<Option<Rc<super::table_edit::EditSession>>>,
    pub versions: RefCell<HashMap<TableCell, u64>>,
    pub last_edit_error: State<Option<String>>,
    pub on_commit: RefCell<Option<super::table_edit::CommitHandler>>,
}
#[derive(Clone)]
pub struct TableModel(pub(super) Rc<TableData>);
#[derive(Clone, Debug, PartialEq)]
pub struct CellEdit {
    pub row: String,
    pub column: String,
    pub original: String,
    pub text: String,
    pub(super) row_token: u64,
    pub(super) column_token: u64,
    pub(super) policy_token: u64,
    pub(super) value_token: u64,
}
impl TableModel {
    /// Positional convenience for simple string tables; columns get `column-N` IDs.
    pub fn new(headers: Vec<String>, rows: Vec<ListItem>) -> Self {
        let columns: Vec<_> = headers
            .into_iter()
            .enumerate()
            .map(|(i, title)| TableColumn::new(format!("column-{i}"), title).editable(true))
            .collect();
        let rows = rows
            .into_iter()
            .map(|r| TableRow {
                key: r.key,
                enabled: r.enabled,
                cells: columns
                    .iter()
                    .zip(r.cells)
                    .map(|(c, v)| (c.id.clone(), v))
                    .collect(),
            })
            .collect();
        Self::with_columns(columns, rows).expect("Valid table rows and columns")
    }
    pub fn with_columns(columns: Vec<TableColumn>, rows: Vec<TableRow>) -> Result<Self, Error> {
        Self::with_options(columns, rows, Default::default())
    }
    pub fn with_options(
        columns: Vec<TableColumn>,
        rows: Vec<TableRow>,
        options: TableOptions,
    ) -> Result<Self, Error> {
        validate_columns(&columns)?;
        validate_rows(&rows, options.row_height)?;
        if !options.header_height.is_finite() || options.header_height <= 0. {
            return Err(Error(
                "Table header height must be finite and positive".into(),
            ));
        }
        if !(options.header_height + options.visible_rows as f64 * options.row_height).is_finite() {
            return Err(Error("Table preferred extent must be finite".into()));
        }
        let list = ListModel::new(vec![], options.row_height).options(ListOptions {
            visible_rows: options.visible_rows,
            overscan_rows: options.overscan_rows,
            selection: if options.multiple_selection {
                ListSelectionMode::Multiple
            } else {
                ListSelectionMode::Single
            },
            ..Default::default()
        })?;
        let model = Self(Rc::new(TableData {
            list,
            source: State::new(Rc::new(rows)),
            index: RefCell::new(HashMap::new()),
            columns: State::new(Rc::new(
                columns
                    .into_iter()
                    .map(|value| Column {
                        value,
                        token: super::list::next_token(),
                        policy_version: super::list::next_token(),
                    })
                    .collect(),
            )),
            options: State::new(options),
            sort: State::new(vec![]),
            cells: State::new(Cells::default()),
            on_sort: RefCell::new(None),
            notifying: Cell::new(false),
            active: State::new(None),
            versions: RefCell::new(HashMap::new()),
            last_edit_error: State::new(None),
            on_commit: RefCell::new(None),
        }));
        model.publish()?;
        Ok(model)
    }
    pub fn options(&self) -> TableOptions {
        self.0.options.get()
    }
    pub fn columns(&self) -> Vec<TableColumn> {
        self.0
            .columns
            .get()
            .iter()
            .map(|c| c.value.clone())
            .collect()
    }
    pub fn column_count(&self) -> usize {
        self.0.columns.with(|columns| columns.len())
    }
    pub fn column_index(&self, key: &str) -> Option<usize> {
        self.0.columns.get().iter().position(|c| c.value.id == key)
    }
    pub fn column_token(&self, key: &str) -> Option<u64> {
        self.0
            .columns
            .get()
            .iter()
            .find(|c| c.value.id == key)
            .map(|c| c.token)
    }
    pub fn row_token(&self, key: &str) -> Option<u64> {
        self.0.list.item_token(key)
    }
    pub fn row_count(&self) -> usize {
        self.0.list.len()
    }
    pub fn row_index(&self, key: &str) -> Option<usize> {
        self.0.list.index_of(key)
    }
    pub fn row(&self, index: usize) -> Option<TableRow> {
        let key = self.0.list.item(index)?.key;
        let source = self.0.source.get();
        Some(source[*self.0.index.borrow().get(&key)?].clone())
    }
    pub fn text(&self, row: &str, column: &str) -> Option<String> {
        self.column_index(column)?;
        let source = self.0.source.get();
        let index = *self.0.index.borrow().get(row)?;
        Some(source[index].cells.get(column).cloned().unwrap_or_default())
    }
    /// Apply authoritative data without emitting a user commit callback.
    pub fn set_text(
        &self,
        row: &str,
        column: &str,
        text: impl Into<String>,
    ) -> Result<bool, Error> {
        let current = self
            .text(row, column)
            .ok_or_else(|| Error("Table row or column was removed".into()))?;
        let text = text.into();
        if current == text {
            return Ok(false);
        }
        let mut rows = (*self.0.source.get()).clone();
        let index = *self.0.index.borrow().get(row).unwrap();
        rows[index].cells.insert(column.into(), text);
        self.set_rows(rows)?;
        Ok(true)
    }
    pub fn reveal_row(&self, key: &str) -> bool {
        let Some(index) = self.row_index(key) else {
            return false;
        };
        self.0.list.reveal(index);
        true
    }
    pub fn realized_row_count(&self) -> usize {
        self.0.list.realized_indices().len()
    }
    pub fn set_rows(&self, rows: Vec<TableRow>) -> Result<(), Error> {
        validate_rows(&rows, self.options().row_height)?;
        crate::state::batch(|| {
            self.update_versions(&rows);
            self.0.source.set(Rc::new(rows));
            self.publish()
        })
    }
    pub fn set_columns(&self, columns: Vec<TableColumn>) -> Result<(), Error> {
        crate::state::check_write();
        validate_columns(&columns)?;
        let old = self.0.columns.get();
        let columns: Vec<_> = columns
            .into_iter()
            .map(|value| {
                let token = old
                    .iter()
                    .find(|c| c.value.id == value.id)
                    .map_or_else(super::list::next_token, |c| c.token);
                let policy_version = old
                    .iter()
                    .find(|c| c.value.id == value.id && c.value.policy == value.policy)
                    .map_or_else(super::list::next_token, |c| c.policy_version);
                Column {
                    value,
                    token,
                    policy_version,
                }
            })
            .collect();
        let sort: Vec<_> = self
            .0
            .sort
            .get()
            .into_iter()
            .filter(|s| {
                columns
                    .iter()
                    .any(|c| c.value.id == s.column && c.value.sortable)
            })
            .collect();
        let changed = old
            .iter()
            .map(|c| &c.value.id)
            .ne(columns.iter().map(|c| &c.value.id))
            || sort != self.sort();
        crate::state::batch(|| {
            self.0.columns.set(Rc::new(columns));
            self.0.sort.set(sort);
            if changed {
                self.publish()
            } else {
                self.reconcile_edit();
                Ok(())
            }
        })
    }
    pub fn widths(&self) -> Vec<TableColumnWidth> {
        self.0
            .columns
            .get()
            .iter()
            .map(|c| TableColumnWidth {
                column: c.value.id.clone(),
                width: c.value.width,
            })
            .collect()
    }
    pub fn set_widths(&self, widths: &[TableColumnWidth]) -> Result<(), Error> {
        let mut columns = self.columns();
        let mut seen = HashSet::new();
        for width in widths {
            if !width.width.is_finite() || !seen.insert(&width.column) {
                return Err(Error(
                    "Table widths must be finite with unique column IDs".into(),
                ));
            }
            let c = columns
                .iter_mut()
                .find(|c| c.id == width.column)
                .ok_or_else(|| Error("Table column was removed".into()))?;
            c.width = width.width.clamp(c.min_width, c.max_width);
        }
        self.set_columns(columns)
    }
    pub fn resize_column(&self, key: &str, width: f64) -> Result<bool, Error> {
        let c = self
            .columns()
            .into_iter()
            .find(|c| c.id == key)
            .ok_or_else(|| Error("Table column was removed".into()))?;
        if !width.is_finite() {
            return Err(Error("Table width must be finite".into()));
        }
        if !c.resizable {
            return Ok(false);
        }
        let width = width.clamp(c.min_width, c.max_width);
        if c.width == width {
            return Ok(false);
        }
        self.set_widths(&[TableColumnWidth {
            column: key.into(),
            width,
        }])?;
        Ok(true)
    }
    pub fn move_column(&self, key: &str, index: usize) -> Result<bool, Error> {
        let mut columns = (*self.0.columns.get()).clone();
        let old = self
            .column_index(key)
            .ok_or_else(|| Error("Table column was removed".into()))?;
        if index >= columns.len() {
            return Err(Error("Table destination column is out of range".into()));
        }
        if old == index {
            return Ok(false);
        }
        let column = columns.remove(old);
        columns.insert(index, column);
        crate::state::batch(|| {
            self.0.columns.set(Rc::new(columns));
            self.publish()
        })?;
        Ok(true)
    }
    pub fn sort(&self) -> Vec<SortDescriptor> {
        self.0.sort.get()
    }
    pub fn set_sort(&self, sort: Vec<SortDescriptor>) -> Result<(), Error> {
        let columns = self.0.columns.get();
        let mut seen = HashSet::new();
        if sort.iter().any(|s| {
            !seen.insert(&s.column)
                || !columns
                    .iter()
                    .any(|c| c.value.id == s.column && c.value.sortable)
        }) {
            return Err(Error(
                "Sort columns must be unique, present and sortable".into(),
            ));
        }
        if self.sort() == sort {
            return Ok(());
        }
        crate::state::batch(|| {
            self.0.sort.set(sort.clone());
            self.publish()
        })?;
        if !self.0.notifying.replace(true) {
            struct Guard<'a>(&'a Cell<bool>);
            impl Drop for Guard<'_> {
                fn drop(&mut self) {
                    self.0.set(false)
                }
            }
            let _guard = Guard(&self.0.notifying);
            let callback = self.0.on_sort.borrow().clone();
            if let Some(callback) = callback {
                callback(sort)
            }
        }
        Ok(())
    }
    pub fn sort_by(&self, index: usize, descending: bool) -> Result<(), Error> {
        let column = self
            .columns()
            .get(index)
            .ok_or_else(|| Error("Table column was removed".into()))?
            .id
            .clone();
        self.set_sort(vec![SortDescriptor::new(
            column,
            if descending {
                SortOrder::Descending
            } else {
                SortOrder::Ascending
            },
        )])
    }
    pub fn toggle_sort(&self, key: &str, add: bool) -> Result<(), Error> {
        let mut sort = if add {
            self.sort()
        } else {
            self.sort()
                .into_iter()
                .filter(|s| s.column == key)
                .collect()
        };
        if let Some(i) = sort.iter().position(|s| s.column == key) {
            if sort[i].order == SortOrder::Ascending {
                sort[i].order = SortOrder::Descending
            } else {
                sort.remove(i);
            }
        } else {
            sort.push(SortDescriptor::new(key, SortOrder::Ascending));
        }
        self.set_sort(sort)
    }
    pub fn on_sort(self, handler: impl Fn(Vec<SortDescriptor>) + 'static) -> Self {
        crate::state::check_write();
        let old = self.0.on_sort.replace(Some(Rc::new(handler)));
        drop(old);
        self
    }
    pub fn on_sort_with_model(
        self,
        handler: impl Fn(&Self, Vec<SortDescriptor>) + 'static,
    ) -> Self {
        let weak = Rc::downgrade(&self.0);
        self.on_sort(move |s| {
            if let Some(i) = weak.upgrade() {
                handler(&Self(i), s)
            }
        })
    }
    fn publish(&self) -> Result<(), Error> {
        let source = self.0.source.get();
        let sort = self.sort();
        let columns = self.0.columns.get();
        let mut rows: Vec<_> = source.iter().collect();
        if !sort.is_empty() {
            rows.sort_by(|a, b| {
                for s in &sort {
                    let cmp = a
                        .cells
                        .get(&s.column)
                        .map(String::as_str)
                        .unwrap_or("")
                        .cmp(b.cells.get(&s.column).map(String::as_str).unwrap_or(""));
                    let cmp = if s.order == SortOrder::Descending {
                        cmp.reverse()
                    } else {
                        cmp
                    };
                    if !cmp.is_eq() {
                        return cmp;
                    }
                }
                std::cmp::Ordering::Equal
            });
        }
        *self.0.index.borrow_mut() = source
            .iter()
            .enumerate()
            .map(|(i, r)| (r.key.clone(), i))
            .collect();
        let rows = rows
            .into_iter()
            .map(|r| ListItem {
                key: r.key.clone(),
                enabled: r.enabled,
                cells: columns
                    .iter()
                    .map(|c| r.cells.get(&c.value.id).cloned().unwrap_or_default())
                    .collect(),
            })
            .collect();
        self.0.list.set_items(rows)?;
        self.prune_cells();
        self.reconcile_edit();
        Ok(())
    }
    fn enabled(&self, cell: &TableCell) -> bool {
        self.column_index(&cell.column).is_some()
            && self
                .row_index(&cell.row)
                .and_then(|i| self.0.list.item(i))
                .is_some_and(|r| r.enabled)
    }
    fn prune_cells(&self) {
        let mut cells = self.0.cells.get();
        cells.selected.retain(|c| self.enabled(c));
        let old = cells.cursor.clone();
        if cells.cursor.as_ref().is_some_and(|c| !self.enabled(c)) {
            cells.cursor = None;
        }
        if cells.cursor.is_none() {
            let row = old
                .as_ref()
                .and_then(|c| self.row_index(&c.row))
                .and_then(|i| self.0.list.item(i))
                .filter(|r| r.enabled)
                .map(|r| r.key)
                .or_else(|| self.0.list.cursor());
            let columns = self.columns();
            let column = old
                .as_ref()
                .and_then(|c| columns.iter().find(|p| p.id == c.column))
                .or_else(|| columns.first());
            if let (Some(row), Some(column)) = (row, column) {
                cells.cursor = Some(TableCell::new(row, &column.id));
            }
        }
        if cells.anchor.as_ref().is_some_and(|c| !self.enabled(c)) {
            cells.anchor = None;
        }
        self.0.cells.set(cells);
    }
    pub fn selection(&self) -> TableSelection {
        if self.options().selection_mode == TableSelectionMode::Rows {
            TableSelection {
                rows: self.0.list.selected(),
                cells: Default::default(),
            }
        } else {
            TableSelection {
                cells: self.0.cells.with(|c| c.selected.clone()),
                rows: Default::default(),
            }
        }
    }
    pub fn selection_count(&self) -> usize {
        if self.options().selection_mode == TableSelectionMode::Rows {
            self.0.list.selection_count()
        } else {
            self.0.cells.with(|c| c.selected.len())
        }
    }
    /// Logical ranges do not clone the complete selection or realize any controls.
    pub fn is_row_selected(&self, key: &str) -> bool {
        self.options().selection_mode == TableSelectionMode::Rows && self.0.list.is_selected(key)
    }
    pub fn selected_rows(&self, start: usize, count: usize) -> Vec<String> {
        if self.options().selection_mode == TableSelectionMode::Rows {
            self.0.list.selected_range(start, count)
        } else {
            vec![]
        }
    }
    pub fn selected_cells(&self, start: usize, count: usize) -> Vec<TableCell> {
        if self.options().selection_mode == TableSelectionMode::Cells {
            self.0
                .cells
                .with(|c| c.selected.iter().skip(start).take(count).cloned().collect())
        } else {
            vec![]
        }
    }
    /// Reject a wrong-mode, disabled or missing target before publishing any change.
    pub fn set_selection(&self, selection: TableSelection) -> Result<(), Error> {
        crate::state::check_write();
        let rows = self.options().selection_mode == TableSelectionMode::Rows;
        if rows && !selection.cells.is_empty()
            || !rows && !selection.rows.is_empty()
            || !self.options().multiple_selection
                && selection.rows.len() + selection.cells.len() > 1
            || selection.rows.iter().any(|key| {
                self.0
                    .list
                    .index_of(key)
                    .and_then(|i| self.0.list.item(i))
                    .is_none_or(|r| !r.enabled)
            })
            || selection.cells.iter().any(|cell| !self.enabled(cell))
        {
            return Err(Error("Table selection must contain current enabled targets and respect the selection mode".into()));
        }
        crate::state::batch(|| {
            if rows {
                let cursor = selection
                    .rows
                    .last()
                    .cloned()
                    .or_else(|| self.0.list.cursor());
                let anchor = selection
                    .rows
                    .first()
                    .cloned()
                    .or_else(|| self.0.list.anchor());
                self.0
                    .list
                    .restore_selection(selection.rows, cursor, anchor);
            } else {
                let mut cells = self.0.cells.get();
                if let Some(cursor) = selection.cells.last() {
                    cells.cursor = Some(cursor.clone());
                    self.0.list.focus_key(&cursor.row);
                }
                if let Some(anchor) = selection.cells.first() {
                    cells.anchor = Some(anchor.clone());
                }
                cells.selected = selection.cells;
                self.0.cells.set(cells);
            }
        });
        Ok(())
    }
    pub fn row_cursor(&self) -> Option<String> {
        self.0.list.cursor()
    }
    pub fn cursor(&self) -> Option<TableCell> {
        if self.options().selection_mode == TableSelectionMode::Cells {
            self.0.cells.with(|c| c.cursor.clone())
        } else {
            self.row_cursor()
                .and_then(|r| self.columns().first().map(|c| TableCell::new(r, &c.id)))
        }
    }
    pub fn set_selection_mode(&self, mode: TableSelectionMode, multiple: bool) {
        let mut o = self.options();
        o.selection_mode = mode;
        o.multiple_selection = multiple;
        crate::state::batch(|| {
            self.0.options.set(o);
            let mut list = self.0.list.list_options();
            list.selection = if multiple {
                ListSelectionMode::Multiple
            } else {
                ListSelectionMode::Single
            };
            self.0
                .list
                .set_options(list)
                .expect("Valid table list options");
            self.0.list.clear_selection();
            let mut cells = self.0.cells.get();
            cells.selected.clear();
            cells.anchor = None;
            self.0.cells.set(cells);
            self.prune_cells();
        });
    }
    pub fn clear_selection(&self) {
        self.0.list.clear_selection();
        let mut c = self.0.cells.get();
        c.selected.clear();
        self.0.cells.set(c);
    }
    pub fn select_row(&self, key: &str, operation: ListSelection) -> bool {
        if self.options().selection_mode != TableSelectionMode::Rows {
            return false;
        }
        self.0.list.select_with(key, operation)
    }
    pub fn select_cell(&self, cell: TableCell, operation: ListSelection) -> bool {
        if self.options().selection_mode != TableSelectionMode::Cells || !self.enabled(&cell) {
            return false;
        }
        let mut s = self.0.cells.get();
        let multiple = self.options().multiple_selection;
        if operation != ListSelection::Focus {
            if !multiple || operation == ListSelection::Replace {
                s.selected.clear();
                s.selected.insert(cell.clone());
                s.anchor = Some(cell.clone());
            } else {
                match operation {
                    ListSelection::Toggle => {
                        if !s.selected.remove(&cell) {
                            s.selected.insert(cell.clone());
                        }
                        s.anchor = Some(cell.clone());
                    }
                    ListSelection::Range | ListSelection::AddRange => {
                        let anchor = s
                            .anchor
                            .clone()
                            .filter(|c| self.enabled(c))
                            .unwrap_or_else(|| cell.clone());
                        let a = self.row_index(&anchor.row).unwrap();
                        let b = self.row_index(&cell.row).unwrap();
                        let x = self.column_index(&anchor.column).unwrap();
                        let y = self.column_index(&cell.column).unwrap();
                        if operation == ListSelection::Range {
                            s.selected.clear();
                        }
                        let columns = self.columns();
                        for r in a.min(b)..=a.max(b) {
                            let row = self.0.list.item(r).unwrap();
                            if row.enabled {
                                for column in &columns[x.min(y)..=x.max(y)] {
                                    s.selected.insert(TableCell::new(&row.key, &column.id));
                                }
                            }
                        }
                        s.anchor = Some(anchor);
                    }
                    _ => {}
                }
            }
        }
        if s.anchor.is_none() {
            s.anchor = Some(cell.clone());
        }
        s.cursor = Some(cell.clone());
        crate::state::batch(|| {
            self.0.list.focus_key(&cell.row);
            self.0.cells.set(s);
        });
        true
    }
    pub(crate) fn is_selected(&self, cell: &TableCell) -> bool {
        self.0.cells.with(|c| c.selected.contains(cell))
    }
    pub fn select_all(&self) -> bool {
        if self.options().selection_mode == TableSelectionMode::Rows {
            return self.0.list.select_all();
        }
        if !self.options().multiple_selection {
            return false;
        }
        let mut cells = self.0.cells.get();
        let columns = self.columns();
        for i in 0..self.row_count() {
            let row = self.0.list.item(i).unwrap();
            if row.enabled {
                for column in &columns {
                    cells.selected.insert(TableCell::new(&row.key, &column.id));
                }
            }
        }
        self.0.cells.set(cells);
        true
    }
    pub fn begin_edit(&self, key: &str, column: usize) -> Option<CellEdit> {
        self.edit_cell(&TableCell::new(key, &self.columns().get(column)?.id))
    }
    pub fn edit_cell(&self, cell: &TableCell) -> Option<CellEdit> {
        crate::state::check_write();
        if !self.enabled(cell)
            || !self
                .0
                .columns
                .get()
                .iter()
                .any(|c| c.value.id == cell.column && c.value.policy.editable)
        {
            return None;
        }
        let text = self.text(&cell.row, &cell.column)?;
        Some(CellEdit {
            row: cell.row.clone(),
            column: cell.column.clone(),
            original: text.clone(),
            text,
            row_token: self.row_token(&cell.row)?,
            column_token: self.column_token(&cell.column)?,
            policy_token: self
                .0
                .columns
                .get()
                .iter()
                .find(|c| c.value.id == cell.column)?
                .policy_version,
            value_token: self.value_token(cell),
        })
    }
    pub fn commit(&self, edit: CellEdit) -> Result<(), Error> {
        self.commit_checked(edit)
    }
}
fn validate_rows(rows: &[TableRow], height: f64) -> Result<(), Error> {
    if !height.is_finite() || height <= 0. || !(rows.len() as f64 * height).is_finite() {
        return Err(Error(
            "Table row height/extent must be finite and positive".into(),
        ));
    }
    let mut keys = HashSet::new();
    if rows
        .iter()
        .any(|r| r.key.trim().is_empty() || !keys.insert(&r.key))
    {
        return Err(Error("Table row keys must be nonempty and unique".into()));
    }
    Ok(())
}
fn validate_columns(columns: &[TableColumn]) -> Result<(), Error> {
    let mut keys = HashSet::new();
    let mut width = 0.;
    for c in columns {
        if c.id.trim().is_empty()
            || !keys.insert(&c.id)
            || ![c.width, c.min_width, c.max_width]
                .iter()
                .all(|v| v.is_finite())
            || c.min_width <= 0.
            || c.max_width < c.min_width
            || c.width < c.min_width
            || c.width > c.max_width
        {
            return Err(Error("Invalid table column key or width bounds".into()));
        }
        width += c.width;
    }
    if !width.is_finite() {
        return Err(Error("Table width must be finite".into()));
    }
    Ok(())
}
