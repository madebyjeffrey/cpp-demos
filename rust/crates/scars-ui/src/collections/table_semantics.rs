//! Logical table metadata and guarded actions, independent of widget realization.
use crate::{Error, Host, ListSelection, NodeId, SortOrder, TableCell, TableModel, TableSelection};
use std::rc::Rc;

/// Callback-free column metadata in current display order.
#[derive(Clone, Debug, PartialEq)]
pub struct TableColumnInfo {
    pub id: String,
    pub title: String,
    pub index: usize,
    pub token: u64,
    pub width: f64,
    pub resizable: bool,
    pub sortable: bool,
    pub editable: bool,
    pub sort: Option<SortOrder>,
    /// Zero-based precedence in a multi-column sort.
    pub sort_priority: Option<usize>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TableCellInfo {
    pub cell: TableCell,
    pub row_index: usize,
    pub column_index: usize,
    pub row_token: u64,
    pub column_token: u64,
    /// Active draft text while editing; committed display text otherwise.
    pub text: String,
    pub enabled: bool,
    pub selected: bool,
    pub editable: bool,
    pub editing: bool,
    pub error: Option<String>,
}

/// Immutable display order and lifetimes. Retains data, never validators or builders.
#[derive(Clone)]
pub struct TableSnapshot {
    rows: crate::ListSnapshot,
    columns: Rc<Vec<TableColumnInfo>>,
}
impl TableSnapshot {
    pub fn rows(&self) -> &crate::ListSnapshot {
        &self.rows
    }
    pub fn column_count(&self) -> usize {
        self.columns.len()
    }
    pub fn column(&self, index: usize) -> Option<&TableColumnInfo> {
        self.columns.get(index)
    }
    pub fn cell(&self, row: usize, column: usize) -> Option<TableCell> {
        Some(TableCell::new(
            &self.rows.item(row)?.key,
            &self.column(column)?.id,
        ))
    }
    pub fn text(&self, row: usize, column: usize) -> Option<&str> {
        self.column(column)?;
        Some(
            self.rows
                .item(row)?
                .cells
                .get(column)
                .map_or("", String::as_str),
        )
    }
}
impl TableModel {
    pub fn snapshot(&self) -> TableSnapshot {
        let sort = self.sort();
        let columns = self
            .0
            .columns
            .get()
            .iter()
            .enumerate()
            .map(|(index, c)| {
                let priority = sort.iter().position(|s| s.column == c.value.id);
                TableColumnInfo {
                    id: c.value.id.clone(),
                    title: c.value.title.clone(),
                    index,
                    token: c.token,
                    width: c.value.width,
                    resizable: c.value.resizable,
                    sortable: c.value.sortable,
                    editable: c.value.is_editable(),
                    sort: priority.map(|i| sort[i].order),
                    sort_priority: priority,
                }
            })
            .collect();
        TableSnapshot {
            rows: self.0.list.snapshot(),
            columns: Rc::new(columns),
        }
    }
    pub fn cell_info(&self, cell: &TableCell) -> Option<TableCellInfo> {
        let row_index = self.row_index(&cell.row)?;
        let column_index = self.column_index(&cell.column)?;
        let editing = self
            .active_edit()
            .and_then(|e| e.info())
            .filter(|e| e.cell == *cell);
        let enabled = self.0.list.item(row_index)?.enabled;
        Some(TableCellInfo {
            cell: cell.clone(),
            row_index,
            column_index,
            row_token: self.row_token(&cell.row)?,
            column_token: self.column_token(&cell.column)?,
            text: editing
                .as_ref()
                .map(|e| e.text.clone())
                .unwrap_or(self.text(&cell.row, &cell.column)?),
            enabled,
            selected: if self.options().selection_mode == crate::TableSelectionMode::Rows {
                self.0.list.is_selected(&cell.row)
            } else {
                self.is_selected(cell)
            },
            editable: self.is_editable(cell),
            editing: editing.is_some(),
            error: editing.and_then(|e| e.error),
        })
    }
}
impl Host {
    fn semantic_table(&self, parent: NodeId) -> Option<TableModel> {
        self.table(parent).filter(|_| self.list(parent).is_some())
    }
    fn current_table_cell(
        &self,
        parent: NodeId,
        cell: &TableCell,
        row: u64,
        column: u64,
    ) -> Option<TableModel> {
        self.semantic_table(parent).filter(|t| {
            t.row_token(&cell.row) == Some(row) && t.column_token(&cell.column) == Some(column)
        })
    }
    /// Replace logical selection atomically without taking focus from another control.
    pub fn set_table_selection(
        &self,
        parent: NodeId,
        selection: TableSelection,
    ) -> Result<bool, Error> {
        self.refresh()?;
        let Some(model) = self.semantic_table(parent) else {
            return Ok(false);
        };
        model.set_selection(selection)?;
        let inside = self
            .focused()
            .is_some_and(|id| Self::within(&self.0.mounted.borrow().nodes, id, parent));
        if inside && let Some(cell) = model.cursor() {
            model.reveal_row(&cell.row);
            self.0.interaction.borrow_mut().pending_row = Some((parent, cell.row));
        }
        self.redraw();
        Ok(true)
    }
    /// Select or focus a current logical cell, revealing only its row.
    pub fn select_table_cell(
        &self,
        parent: NodeId,
        cell: TableCell,
        row_token: u64,
        column_token: u64,
        operation: ListSelection,
    ) -> Result<bool, Error> {
        self.refresh()?;
        let Some(model) = self.current_table_cell(parent, &cell, row_token, column_token) else {
            return Ok(false);
        };
        self.table_select_cell(
            parent,
            &super::TableCellAction {
                model,
                cell,
                row_token,
                column_token,
            },
            operation,
        )
    }
    pub fn edit_table_cell(
        &self,
        parent: NodeId,
        cell: TableCell,
        row_token: u64,
        column_token: u64,
    ) -> Result<bool, Error> {
        self.refresh()?;
        if self
            .current_table_cell(parent, &cell, row_token, column_token)
            .is_none()
        {
            return Ok(false);
        }
        self.start_table_cell_edit_checked(parent, cell, Some((row_token, column_token)))
    }
    /// Apply a user value through the same draft/validation/commit path as keyboard editing.
    pub fn set_table_cell_text(
        &self,
        parent: NodeId,
        cell: TableCell,
        row_token: u64,
        column_token: u64,
        text: &str,
    ) -> Result<bool, Error> {
        if !self.edit_table_cell(parent, cell.clone(), row_token, column_token)? {
            return Ok(false);
        }
        // Starting this editor may have been blocked by another invalid draft.
        let Some(edit) = self
            .current_table_cell(parent, &cell, row_token, column_token)
            .and_then(|t| t.active_edit())
            .filter(|e| e.info().is_some_and(|i| i.cell == cell))
        else {
            return Ok(false);
        };
        edit.set_text(text);
        let result = edit.commit();
        self.redraw();
        result
    }
    pub fn sort_table_column(
        &self,
        parent: NodeId,
        column: &str,
        token: u64,
        add: bool,
    ) -> Result<bool, Error> {
        self.refresh()?;
        let Some(model) = self
            .semantic_table(parent)
            .filter(|t| t.column_token(column) == Some(token))
        else {
            return Ok(false);
        };
        if !model.columns().iter().any(|c| c.id == column && c.sortable) {
            return Ok(false);
        }
        model.toggle_sort(column, add)?;
        self.redraw();
        Ok(true)
    }
}
