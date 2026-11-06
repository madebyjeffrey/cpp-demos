use crate::{Binding, Kind, TableCell, TableModel, TableSelectionMode, View};
use std::rc::Rc;
#[derive(Clone)]
pub(crate) struct TableCellAction {
    pub model: TableModel,
    pub cell: TableCell,
    pub row_token: u64,
    pub column_token: u64,
}
#[derive(Clone)]
pub(crate) struct TableHeaderAction {
    pub model: TableModel,
    pub column: String,
    pub token: u64,
    pub resize: bool,
}
impl TableModel {
    pub fn view(&self) -> View {
        let columns = self.0.columns.get();
        let options = self.options();
        let width: f64 = columns.iter().map(|c| c.value.width).sum();
        let sort = self.sort();
        let headers = columns
            .iter()
            .map(|c| {
                let order = sort.iter().position(|s| s.column == c.value.id);
                let indicator = order.map_or(String::new(), |i| {
                    format!(
                        " {}{}",
                        if sort[i].order == crate::SortOrder::Ascending {
                            "↑"
                        } else {
                            "↓"
                        },
                        if sort.len() > 1 {
                            (i + 1).to_string()
                        } else {
                            String::new()
                        }
                    )
                });
                let action = TableHeaderAction {
                    model: self.clone(),
                    column: c.value.id.clone(),
                    token: c.token,
                    resize: false,
                };
                let mut header = View::button(format!("{}{indicator}", c.value.title), || {})
                    .width(c.value.width - if c.value.resizable { 8. } else { 0. })
                    .height(options.header_height)
                    .shrink(0.)
                    .key(format!("header-{}-{}", c.value.id, c.token))
                    .accessible_label(format!("{} column", c.value.title));
                Rc::make_mut(&mut header.0).table_header = Some(action.clone());
                let mut children = vec![header];
                if c.value.resizable {
                    let mut resize = View::button("⋮", || {})
                        .width(8)
                        .height(options.header_height)
                        .shrink(0.)
                        .key(format!("resize-{}-{}", c.value.id, c.token))
                        .accessible_label(format!("Resize {} column", c.value.title));
                    let d = Rc::make_mut(&mut resize.0);
                    d.style.padding = 0.0.into();
                    d.table_header = Some(TableHeaderAction {
                        resize: true,
                        ..action
                    });
                    children.push(resize);
                }
                View::row(children)
                    .width(c.value.width)
                    .shrink(0.)
                    .gap(0.)
                    .key(&c.value.id)
            })
            .collect();
        let mut list = self
            .0
            .list
            .view_with(|item| {
                View::row(
                    columns
                        .iter()
                        .map(|c| {
                            let cell = TableCell::new(&item.key, &c.value.id);
                            self.cell_view(cell, c, options.row_height)
                        })
                        .collect(),
                )
                .gap(0.)
            })
            .width(width)
            .grow(1.);
        let d = Rc::make_mut(&mut list.0);
        d.table = Some(self.clone());
        for child in &mut d.children {
            if child.0.kind == Kind::CollectionRow {
                let d = Rc::make_mut(&mut child.0);
                d.style.padding = 0.0.into();
                d.style.radius = 0.;
                d.context = None;
                if options.selection_mode == TableSelectionMode::Cells {
                    d.data = crate::view::Data::None;
                }
            }
        }
        let content = View::column(vec![
            View::row(headers)
                .gap(0.)
                .height(options.header_height)
                .shrink(0.),
            list,
        ])
        .gap(0.)
        .width(width)
        .height(crate::percent(100.))
        .shrink(0.);
        let scroll = View::scroll(vec![content])
            .scroll_axes(crate::ScrollAxes::Horizontal)
            .scrollbars(true)
            .height(options.header_height + options.visible_rows as f64 * options.row_height)
            .gap(0.)
            .grow(1.);
        let error = self.edit_error();
        let mut root = View::column(vec![
            scroll,
            View::label(error.as_deref().unwrap_or(""))
                .label_style(crate::LabelStyle::Error)
                .wrap(true)
                .hidden(error.is_none())
                .shrink(0.)
                .key("$edit-error"),
        ])
        .gap(4.)
        .height(options.header_height + options.visible_rows as f64 * options.row_height);
        Rc::make_mut(&mut root.0).table_root = Some(self.clone());
        root
    }
    fn cell_view(&self, cell: TableCell, column: &super::table::Column, height: f64) -> View {
        let text = self.text(&cell.row, &cell.column).unwrap_or_default();
        let session = self
            .0
            .active
            .get()
            .filter(|s| s.edit.row == cell.row && s.edit.column == cell.column);
        let mut view = if let Some(session) = session {
            let handle = self.active_edit().unwrap();
            let read = handle.clone();
            let write = handle.clone();
            let _ = session.focus.get();
            let mut field = View::text_field(
                "",
                Binding::new(
                    move || read.info().map_or(String::new(), |i| i.text),
                    move |text| {
                        write.set_text(text);
                    },
                ),
            )
            .max_length(column.value.policy.max_length)
            .accessible_label(format!("{} editor", column.value.title))
            .key(format!("edit-{}-{}", cell.column, session.id));
            let d = Rc::make_mut(&mut field.0);
            d.multiline = column.value.policy.multiline;
            d.table_editor = Some(handle);
            if let Some(error) = session.error.get() {
                field = field.tooltip(error).label_style(crate::LabelStyle::Error);
            }
            field
        } else {
            let mut view = if let Some(content) = &column.value.policy.content {
                let body = crate::state::read_only(|| content(&cell, &text));
                let mut wrapper = View::column(vec![body])
                    .justify(crate::Justify::Center)
                    .padding(crate::Insets {
                        left: 14.,
                        right: 14.,
                        top: 0.,
                        bottom: 0.,
                    });
                let d = Rc::make_mut(&mut wrapper.0);
                d.kind = Kind::CollectionRow;
                d.accessible = text.clone();
                wrapper
            } else {
                View::button(text, || {})
            };
            let read = self.clone();
            let target = cell.clone();
            Rc::make_mut(&mut view.0).data = crate::view::Data::Bool(Binding::new(
                move || {
                    read.options().selection_mode == TableSelectionMode::Cells
                        && read.is_selected(&target)
                },
                |_| {},
            ));
            view.key(format!("{}-{}", cell.column, column.token))
                .text_align(column.value.alignment)
                .text_overflow(crate::TextOverflow::Ellipsis)
        }
        .width(column.value.width)
        .height(height)
        .shrink(0.);
        let d = Rc::make_mut(&mut view.0);
        d.style.radius = 0.;
        d.table_cell = Some(TableCellAction {
            model: self.clone(),
            row_token: self.row_token(&cell.row).unwrap(),
            column_token: column.token,
            cell,
        });
        view
    }
}
impl PartialEq for TableModel {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}
impl Eq for TableModel {}
