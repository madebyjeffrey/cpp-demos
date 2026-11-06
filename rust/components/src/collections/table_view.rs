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
                            let read = self.clone();
                            let read_cell = cell.clone();
                            let mut view = View::button(
                                self.text(&item.key, &c.value.id).unwrap_or_default(),
                                || {},
                            )
                            .width(c.value.width)
                            .height(options.row_height)
                            .shrink(0.)
                            .text_align(c.value.alignment)
                            .text_overflow(crate::TextOverflow::Ellipsis)
                            .key(format!("{}-{}", c.value.id, c.token));
                            let d = Rc::make_mut(&mut view.0);
                            d.style.radius = 0.;
                            d.data = crate::view::Data::Bool(Binding::new(
                                move || {
                                    read.options().selection_mode == TableSelectionMode::Cells
                                        && read.is_selected(&read_cell)
                                },
                                |_| {},
                            ));
                            d.table_cell = Some(TableCellAction {
                                model: self.clone(),
                                cell,
                                row_token: self.row_token(&item.key).unwrap(),
                                column_token: c.token,
                            });
                            view
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
        View::scroll(vec![content])
            .scroll_axes(crate::ScrollAxes::Horizontal)
            .scrollbars(true)
            .height(options.header_height + options.visible_rows as f64 * options.row_height)
            .gap(0.)
    }
}
impl PartialEq for TableModel {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}
impl Eq for TableModel {}
