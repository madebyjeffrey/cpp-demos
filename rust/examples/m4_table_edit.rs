//! Integrated validated text edits, custom display, retained buffers and conflict checks.
mod common;
use scars_window::ui::*;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let status = State::new(
        "Return or double-click an editable cell. Escape cancels; Tab saves and leaves the table."
            .to_owned(),
    );
    let table = TableModel::with_options(
        vec![
            TableColumn::new("name", "Document")
                .width(280.)
                .editable(true)
                .max_length(48)
                .validate(|_, s| {
                    if s.trim().is_empty() {
                        Err(Error("A document name is required".into()))
                    } else {
                        Ok(())
                    }
                }),
            TableColumn::new("notes", "Notes · multiline")
                .width(420.)
                .editable(true)
                .multiline(true),
            TableColumn::new("state", "Status · read-only")
                .width(150.)
                .content(|_, s| View::label(format!("● {s}")).label_style(LabelStyle::Secondary)),
        ],
        (0..100_000)
            .map(|i| {
                TableRow::new(i.to_string())
                    .cell("name", format!("Document {i:06}"))
                    .cell("notes", "Draft notes")
                    .cell("state", "Ready")
            })
            .collect(),
        TableOptions {
            row_height: 64.,
            selection_mode: TableSelectionMode::Cells,
            ..Default::default()
        },
    )?
    .on_commit(status.handler(|s, event: TableCommit| {
        s.set(format!(
            "Saved {} / {}: {}",
            event.cell.row, event.cell.column, event.text
        ))
    }));
    let edit = table.start_edit(TableCell::new("0", "name"))?;
    if std::env::args().any(|a| a == "--invalid") {
        edit.set_text("");
        let _ = edit.commit();
    }
    common::run("SCARS · Table editing", (940, 660), move || {
        let save = table.clone();
        let cancel = table.clone();
        let first = table.clone();
        let last = table.clone();
        let focus = table.clone();
        let conflict = table.clone();
        let active = table.active_edit();
        view! {column(padding:28,gap:16){
            label("Edit a cell. Keep the draft.",label_style:LabelStyle::Heading,shrink:0);
            label("The active row stays pinned through scrolling and sorting. Notes accept line breaks; Command/Control-Return saves them. Status is custom read-only content.",wrap:true,label_style:LabelStyle::Secondary,shrink:0);
            row(shrink:0){
                button("Save",enabled:active.is_some(),on_click:move||{let _=save.commit_edit();});
                button("Cancel",enabled:active.is_some(),on_click:move||{cancel.cancel_edit();});
                button("First rows",on_click:move||{first.reveal_row("0");});
                button("Last rows",on_click:move||{last.reveal_row("99999");});
                button("Back to edit",enabled:active.is_some(),on_click:move||{if let Some(edit)=focus.active_edit(){edit.focus();}});
                button("Simulate conflict",enabled:active.is_some(),on_click:move||{if let Some(info)=conflict.active_edit().and_then(|e|e.info()){conflict.set_text(&info.cell.row,&info.cell.column,"Remote update").unwrap();}});
            }
            embed(table.view(),grow:1,key:"documents");
            label(format!("{} realized rows · {} · 100,000 total",table.realized_row_count(),active.and_then(|e|e.info()).map_or("No active edit".into(),|i|format!("Editing {} / {}",i.cell.row,i.cell.column))),wrap:true,label_style:LabelStyle::Micro,shrink:0);
            label(status.get(),wrap:true,label_style:LabelStyle::Status,shrink:0);
        }}
    })
}
