//! Portable collection models.
mod combo;
mod combo_input;
pub(crate) mod input;
mod list;
mod pin;
mod tree;
mod tree_input;
use crate::View;
pub use combo::*;
pub use list::*;
pub use pin::RowPin;
pub(crate) use tree::TreeAction;
pub use tree::*;

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
