//! Keyed columns, bounded rows, resize/reorder, multi-sort and rectangular selection.
mod common;
use scars_window::ui::*;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let table = TableModel::with_options(
        vec![
            TableColumn::new("name", "Document")
                .width(280.)
                .limits(120., 600.),
            TableColumn::new("team", "Workspace").width(210.),
            TableColumn::new("state", "Status").width(170.),
            TableColumn::new("size", "Size")
                .width(140.)
                .alignment(Align::End),
        ],
        (0..100_000)
            .map(|i| {
                TableRow::new(i.to_string())
                    .cell("name", format!("Document {i:06}"))
                    .cell("team", if i % 3 == 0 { "Personal" } else { "Team" })
                    .cell("state", if i % 7 == 0 { "Draft" } else { "Ready" })
                    .cell("size", format!("{} KB", i % 900 + 1))
                    .enabled(i % 19 != 5)
            })
            .collect(),
        TableOptions {
            selection_mode: TableSelectionMode::Cells,
            multiple_selection: true,
            ..Default::default()
        },
    )?;
    table.select_cell(TableCell::new("0", "name"), ListSelection::Replace);
    common::run("SCARS · Keyed tables", (940, 650), move || {
        let mode = table.clone();
        let order = table.clone();
        let clear = table.clone();
        let cells = table.options().selection_mode == TableSelectionMode::Cells;
        view! {column(padding:28,gap:16){
            label("A table that follows its keys",label_style:LabelStyle::Heading,shrink:0);
            label("Drag header edges to resize and headers to reorder. Click to sort; Shift-click adds a sort column. Header keyboard: arrows resize; Alt-arrows reorder.",wrap:true,label_style:LabelStyle::Secondary,shrink:0);
            row(shrink:0){
                button(if cells{"Switch to row selection"}else{"Switch to cell selection"},on_click:move||mode.set_selection_mode(if cells{TableSelectionMode::Rows}else{TableSelectionMode::Cells},true));
                button("Move document column",on_click:move||{let target=if order.column_index("name")==Some(0){order.columns().len()-1}else{0};order.move_column("name",target).unwrap();});
                button("Clear sort",on_click:move||clear.set_sort(vec![]).unwrap());
            }
            embed(table.view(),grow:1,key:"documents");
            label(format!("100,000 rows · {} selected · cursor {} · {} sort columns",table.selection_count(),table.cursor().map_or("—".into(),|c|format!("{} / {}",c.row,c.column)),table.sort().len()),label_style:LabelStyle::Micro,wrap:true,shrink:0);
            label("Arrows move; Shift extends a row range or cell rectangle. Command/Control toggles pointer selection. Column order never changes what a cell key means.",label_style:LabelStyle::Status,wrap:true,shrink:0);
        }}
    })
}
