//! Virtual rows, sorting, tree expansion, editable combo, and conflict-checked cells.
mod common;
use scars_window::ui::*;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let table = TableModel::new(
        vec!["Document".into(), "Status".into()],
        (0..100_000)
            .map(|i| ListItem {
                key: format!("doc-{i}"),
                cells: vec![
                    format!("Document {i:06}"),
                    if i % 3 == 0 { "Draft" } else { "Ready" }.into(),
                ],
                enabled: true,
            })
            .collect(),
    );
    let tree = TreeModel::new();
    tree.set_nodes(vec![TreeNode::new("workspace", "Workspace").children(
        vec![
            TreeNode::new("design", "Design"),
            TreeNode::new("engineering", "Engineering"),
        ],
    )])?;
    let query = State::new(String::new());
    let popup = Popup::new();
    let editor = Popup::new();
    let draft = State::new(String::new());
    let edit = State::new(None::<CellEdit>);
    let message = State::new(String::new());
    common::run("Stage 5 · Collections", (900, 650), move || {
        let (combo, combo_menu) = combo_box(
            query.binding(),
            &["Personal".into(), "Team".into(), "Archive".into()],
            &popup,
            "workspace-choice",
        );
        let begin = {
            let table = table.clone();
            let editor = editor.clone();
            let edit = edit.clone();
            let draft = draft.clone();
            move || {
                if let Some(key) = table.rows.selected().first()
                    && let Some(cell) = table.begin_edit(key, 0)
                {
                    draft.set(cell.text.clone());
                    edit.set(Some(cell));
                    editor.show();
                }
            }
        };
        let save = {
            let (table, editor, edit, draft, message) = (
                table.clone(),
                editor.clone(),
                edit.clone(),
                draft.clone(),
                message.clone(),
            );
            move || {
                if let Some(mut cell) = edit.get() {
                    cell.text = draft.get();
                    match table.commit(cell) {
                        Ok(()) => {
                            message.set("Saved".into());
                            editor.dismiss();
                        }
                        Err(e) => message.set(e.to_string()),
                    }
                }
            }
        };
        view! {layer{
            column(padding:24,gap:18){
                label("A hundred thousand documents",font_size:28,bold:true,shrink:0);
                row(shrink:0){embed(combo,grow:1);button("Edit selected…",on_click:begin);}
                row(grow:1,gap:20){panel(width:210,padding:12){label("BROWSE",bold:true);embed(tree.view(),grow:1);}embed(table.view(),grow:1);}
                label("Only visible rows are mounted. Click a column to sort. Home/End jumps through the collection.",shrink:0);
                label(message.get(),shrink:0);
            }
            embed(combo_menu);
            embed(editor.modal("Edit document",view!{column{ text_field("Name",bind:draft);button("Save",on_click:save);}}));
        }}
    })
}
