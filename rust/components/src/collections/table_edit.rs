//! One generation-checked, pinned text edit per table.
use super::{
    TableCell, TableColumn, TableModel,
    table::{CellEdit, TableData},
};
use crate::{Error, RowPin, State, View};
use std::{
    cell::RefCell,
    collections::HashMap,
    fmt,
    rc::{Rc, Weak},
};
type Validator = Rc<dyn Fn(&TableCell, &str) -> Result<(), Error>>;
type Content = Rc<dyn Fn(&TableCell, &str) -> View>;
#[derive(Clone, Default)]
pub(super) struct CellPolicy {
    pub editable: bool,
    pub multiline: bool,
    pub max_length: usize,
    pub validate: Option<Validator>,
    pub content: Option<Content>,
}
impl PartialEq for CellPolicy {
    fn eq(&self, other: &Self) -> bool {
        self.editable == other.editable
            && self.multiline == other.multiline
            && self.max_length == other.max_length
            && match (&self.validate, &other.validate) {
                (None, None) => true,
                (Some(a), Some(b)) => Rc::ptr_eq(a, b),
                _ => false,
            }
            && match (&self.content, &other.content) {
                (None, None) => true,
                (Some(a), Some(b)) => Rc::ptr_eq(a, b),
                _ => false,
            }
    }
}
impl fmt::Debug for CellPolicy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CellPolicy")
            .field("editable", &self.editable)
            .field("multiline", &self.multiline)
            .field("max_length", &self.max_length)
            .field("validation", &self.validate.is_some())
            .field("custom_content", &self.content.is_some())
            .finish()
    }
}
impl TableColumn {
    pub fn is_editable(&self) -> bool {
        self.policy.editable
    }
    pub fn editable(mut self, value: bool) -> Self {
        Rc::make_mut(&mut self.policy).editable = value;
        self
    }
    pub fn multiline(mut self, value: bool) -> Self {
        Rc::make_mut(&mut self.policy).multiline = value;
        self
    }
    pub fn max_length(mut self, value: usize) -> Self {
        Rc::make_mut(&mut self.policy).max_length = value;
        self
    }
    pub fn validate(
        mut self,
        validate: impl Fn(&TableCell, &str) -> Result<(), Error> + 'static,
    ) -> Self {
        Rc::make_mut(&mut self.policy).validate = Some(Rc::new(validate));
        self
    }
    pub fn content(mut self, content: impl Fn(&TableCell, &str) -> View + 'static) -> Self {
        Rc::make_mut(&mut self.policy).content = Some(Rc::new(content));
        self
    }
}
pub(super) struct EditSession {
    pub id: u64,
    pub edit: CellEdit,
    pub text: State<String>,
    pub error: State<Option<String>>,
    pub focus: State<bool>,
    pub pin: RefCell<Option<RowPin>>,
}
impl PartialEq for EditSession {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TableEditInfo {
    pub cell: TableCell,
    pub text: String,
    pub error: Option<String>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TableCommit {
    pub cell: TableCell,
    pub text: String,
}
pub(super) type CommitHandler = Rc<dyn Fn(TableCommit)>;
/// A weak handle to one edit. A retired handle cannot act on a later editor.
#[derive(Clone)]
pub struct TableEdit {
    pub(super) owner: Weak<TableData>,
    pub(super) id: u64,
}
impl TableEdit {
    fn session(&self) -> Option<Rc<EditSession>> {
        self.owner
            .upgrade()?
            .active
            .get()
            .filter(|s| s.id == self.id)
    }
    pub fn info(&self) -> Option<TableEditInfo> {
        let s = self.session()?;
        Some(TableEditInfo {
            cell: TableCell::new(&s.edit.row, &s.edit.column),
            text: s.text.get(),
            error: s.error.get(),
        })
    }
    pub fn is_current(&self) -> bool {
        self.session().is_some()
    }
    pub fn set_text(&self, text: impl Into<String>) -> bool {
        let Some(s) = self.session() else {
            return false;
        };
        let text = text.into();
        if s.text.get() != text {
            s.text.set(text);
            if let Some(model) = self.owner.upgrade()
                && TableModel(model).edit_is_current(&s.edit)
            {
                s.error.set(None);
            }
        }
        true
    }
    pub fn commit(&self) -> Result<bool, Error> {
        let Some(owner) = self.owner.upgrade() else {
            return Ok(false);
        };
        let model = TableModel(owner);
        if !self.is_current() {
            return Ok(false);
        }
        model.commit_edit()
    }
    pub fn cancel(&self) -> bool {
        self.owner.upgrade().is_some_and(|owner| {
            if !self.is_current() {
                false
            } else {
                TableModel(owner).cancel_edit()
            }
        })
    }
    pub fn focus(&self) -> bool {
        let Some(s) = self.session() else {
            return false;
        };
        s.focus.set(true);
        true
    }
    pub(crate) fn wants_focus(&self) -> bool {
        self.session().is_some_and(|s| s.focus.get())
    }
    pub(crate) fn take_focus(&self) -> bool {
        let Some(s) = self.session() else {
            return false;
        };
        let requested = s.focus.get();
        s.focus.set(false);
        requested
    }
    pub(crate) fn multiline(&self) -> bool {
        let Some(owner) = self.owner.upgrade() else {
            return false;
        };
        let Some(s) = self.session() else {
            return false;
        };
        owner
            .columns
            .get()
            .iter()
            .any(|c| c.value.id == s.edit.column && c.value.policy.multiline)
    }
}
impl TableModel {
    pub fn is_editable(&self, cell: &TableCell) -> bool {
        self.0
            .columns
            .get()
            .iter()
            .any(|c| c.value.id == cell.column && c.value.policy.editable)
            && self
                .row_index(&cell.row)
                .and_then(|i| self.0.list.item(i))
                .is_some_and(|r| r.enabled)
    }
    pub fn active_edit(&self) -> Option<TableEdit> {
        self.0.active.get().map(|s| TableEdit {
            owner: Rc::downgrade(&self.0),
            id: s.id,
        })
    }
    pub fn edit_error(&self) -> Option<String> {
        self.0
            .active
            .get()
            .and_then(|s| s.error.get())
            .or_else(|| self.0.last_edit_error.get())
    }
    pub fn start_edit(&self, cell: TableCell) -> Result<TableEdit, Error> {
        crate::state::check_write();
        if let Some(active) = self.active_edit() {
            if active.info().is_some_and(|i| i.cell == cell) {
                active.focus();
                return Ok(active);
            }
            return Err(Error("Finish or cancel the active cell edit first".into()));
        }
        let edit = self
            .edit_cell(&cell)
            .ok_or_else(|| Error("Cell is missing, disabled or read-only".into()))?;
        crate::state::batch(|| {
            let pin = self.0.list.pin(&cell.row)?;
            let id = super::list::next_token();
            let session = Rc::new(EditSession {
                id,
                text: State::new(edit.text.clone()),
                edit,
                error: State::new(None),
                focus: State::new(true),
                pin: RefCell::new(Some(pin)),
            });
            self.0.last_edit_error.set(None);
            self.0.active.set(Some(session));
            if self.options().selection_mode == crate::TableSelectionMode::Cells {
                self.select_cell(cell.clone(), crate::ListSelection::Replace);
            } else {
                self.select_row(&cell.row, crate::ListSelection::Replace);
            }
            if let Some(index) = self.row_index(&cell.row) {
                self.0.list.reveal(index);
            }
            Ok(TableEdit {
                owner: Rc::downgrade(&self.0),
                id,
            })
        })
    }
    pub fn cancel_edit(&self) -> bool {
        if self.0.active.get().is_none() {
            return false;
        }
        let session = self.0.active.get().unwrap();
        crate::state::batch(|| {
            self.0.active.set(None);
            let pin = session.pin.borrow_mut().take();
            drop(pin);
        });
        true
    }
    pub fn commit_edit(&self) -> Result<bool, Error> {
        let Some(session) = self.0.active.get() else {
            return Ok(false);
        };
        let mut edit = session.edit.clone();
        edit.text = session.text.get();
        match self.commit(edit) {
            Ok(()) => Ok(true),
            Err(error) => {
                if self.0.active.get().is_some_and(|s| s.id == session.id) {
                    session.error.set(Some(error.to_string()));
                }
                Err(error)
            }
        }
    }
    pub fn on_commit(self, handler: impl Fn(TableCommit) + 'static) -> Self {
        crate::state::check_write();
        let old = self.0.on_commit.replace(Some(Rc::new(handler)));
        drop(old);
        self
    }
    pub fn on_commit_with_model(self, handler: impl Fn(&Self, TableCommit) + 'static) -> Self {
        let weak = Rc::downgrade(&self.0);
        self.on_commit(move |c| {
            if let Some(owner) = weak.upgrade() {
                handler(&Self(owner), c)
            }
        })
    }
    pub(super) fn value_token(&self, cell: &TableCell) -> u64 {
        *self
            .0
            .versions
            .borrow_mut()
            .entry(cell.clone())
            .or_insert_with(super::list::next_token)
    }
    pub(super) fn edit_is_current(&self, edit: &CellEdit) -> bool {
        self.row_token(&edit.row) == Some(edit.row_token)
            && self.column_token(&edit.column) == Some(edit.column_token)
            && self.0.columns.get().iter().any(|c| {
                c.value.id == edit.column
                    && c.policy_version == edit.policy_token
                    && c.value.policy.editable
            })
            && self
                .0
                .versions
                .borrow()
                .get(&TableCell::new(&edit.row, &edit.column))
                == Some(&edit.value_token)
    }
    pub(super) fn update_versions(&self, rows: &[super::TableRow]) {
        let source = self.0.source.get();
        let before = self.0.index.borrow();
        let next: HashMap<_, _> = rows.iter().map(|r| (r.key.as_str(), r)).collect();
        self.0.versions.borrow_mut().retain(|cell, version| {
            let Some(row) = next.get(cell.row.as_str()) else {
                return false;
            };
            if let Some(old) = before.get(&cell.row).and_then(|i| source.get(*i))
                && (old.enabled != row.enabled
                    || old.cells.get(&cell.column) != row.cells.get(&cell.column))
            {
                *version = super::list::next_token();
            }
            true
        });
    }
    pub(super) fn reconcile_edit(&self) {
        let columns = self.0.columns.get();
        self.0.versions.borrow_mut().retain(|cell, _| {
            self.row_index(&cell.row).is_some() && columns.iter().any(|c| c.value.id == cell.column)
        });
        let Some(session) = self.0.active.get() else {
            return;
        };
        let valid = session.pin.borrow().as_ref().is_some_and(RowPin::is_valid)
            && self.row_token(&session.edit.row) == Some(session.edit.row_token)
            && columns.iter().any(|c| {
                c.value.id == session.edit.column
                    && c.token == session.edit.column_token
                    && c.policy_version == session.edit.policy_token
                    && c.value.policy.editable
            });
        if !valid {
            self.cancel_edit();
            self.0.last_edit_error.set(Some(
                "The edited row, column or editing policy was retired".into(),
            ));
        } else if !self.edit_is_current(&session.edit) {
            session.error.set(Some(
                "Cell changed while editing; cancel and reopen to use the new value".into(),
            ));
        }
    }
    pub(super) fn commit_checked(&self, edit: CellEdit) -> Result<(), Error> {
        crate::state::check_write();
        let cell = TableCell::new(&edit.row, &edit.column);
        if self.row_token(&edit.row) != Some(edit.row_token)
            || self.column_token(&edit.column) != Some(edit.column_token)
        {
            return Err(Error("Row or column was removed/replaced".into()));
        }
        if !self.edit_is_current(&edit) {
            return Err(Error(
                "Cell changed while editing or its editing policy was replaced".into(),
            ));
        }
        if !self
            .0
            .list
            .index_of(&edit.row)
            .and_then(|i| self.0.list.item(i))
            .is_some_and(|r| r.enabled)
        {
            return Err(Error("Cell is disabled".into()));
        }
        let policy = self
            .0
            .columns
            .get()
            .iter()
            .find(|c| c.value.id == edit.column)
            .unwrap()
            .value
            .policy
            .clone();
        if edit.text.contains(['\0', '\r']) || !policy.multiline && edit.text.contains('\n') {
            return Err(Error(
                "Cell text contains unsupported control characters".into(),
            ));
        }
        if policy.max_length > 0
            && crate::text::grapheme_boundaries(&edit.text)
                .len()
                .saturating_sub(1)
                > policy.max_length
        {
            return Err(Error("Cell text exceeds its maximum length".into()));
        }
        if let Some(validate) = &policy.validate {
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                crate::state::read_only(|| validate(&cell, &edit.text))
            }))
            .map_err(|_| Error("Table validator panicked or changed state".into()))??;
        }
        if !self.edit_is_current(&edit) {
            return Err(Error("Cell changed during validation".into()));
        }
        let mut source = (*self.0.source.get()).clone();
        let index = *self
            .0
            .index
            .borrow()
            .get(&edit.row)
            .ok_or_else(|| Error("Row was removed".into()))?;
        source[index]
            .cells
            .insert(edit.column.clone(), edit.text.clone());
        crate::state::batch(|| {
            self.0
                .versions
                .borrow_mut()
                .insert(cell.clone(), super::list::next_token());
            if self
                .0
                .active
                .get()
                .is_some_and(|s| s.edit.row == edit.row && s.edit.column == edit.column)
            {
                let active = self.0.active.get().unwrap();
                self.0.active.set(None);
                let pin = active.pin.borrow_mut().take();
                drop(pin);
            }
            self.0.last_edit_error.set(None);
            self.set_rows(source)
        })?;
        let callback = self.0.on_commit.borrow().clone();
        if let Some(callback) = callback {
            callback(TableCommit {
                cell,
                text: edit.text,
            });
        }
        Ok(())
    }
}
