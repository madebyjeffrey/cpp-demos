//! A bounded active edit survives scrolling, sorting and a million-pixel jump.
mod common;
use scars_window::ui::*;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let edit = State::new(None::<RowPin>);
    let draft = State::new(String::new());
    let status = State::new("Return or double-click a row to edit it.".to_string());
    let list = ListModel::new(
        (0..100_000)
            .map(|i| ListItem::new(i.to_string(), format!("Document {i:06}")))
            .collect(),
        56.,
    )
    .on_activate_with_model({
        let edit = edit.clone();
        let draft = draft.clone();
        let status = status.clone();
        move |list, key| {
            if edit.get().is_some_and(|pin| pin.is_valid()) {
                status.set("Save or cancel the current edit first.".into());
                return;
            }
            match list.pin(&key) {
                Ok(pin) => {
                    draft.set(list.item(list.index_of(&key).unwrap()).unwrap().label());
                    edit.set(Some(pin));
                    status.set(
                        "Scroll away and back: selection and undo stay with this edit.".into(),
                    );
                }
                Err(e) => status.set(e.to_string()),
            }
        }
    });
    // Start with one active edit so previews show the retained control.
    list.activate("0");
    common::run("SCARS · Retained row edits", (860, 650), move || {
        let first = list.clone();
        let last = list.clone();
        let restore = list.clone();
        let active = edit.clone();
        let save = list.clone();
        let save_edit = edit.clone();
        let save_draft = draft.clone();
        let save_status = status.clone();
        let cancel = edit.clone();
        let cancel_status = status.clone();
        let current = edit.get().filter(RowPin::is_valid);
        let valid = current.is_some();
        view! {column(padding:28,gap:16){
            label("Scroll away. Keep the edit.",label_style:LabelStyle::Heading,shrink:0);
            label("One explicit pin retains the editor, selection, composition and undo. Other rows remain virtual.",wrap:true,label_style:LabelStyle::Secondary,shrink:0);
            row(shrink:0){
                button("First rows",on_click:move||first.reveal(0));
                button("Last rows",on_click:move||last.reveal(last.len().saturating_sub(1)));
                button("Back to edit",enabled:valid,on_click:move||{if let Some(pin)=active.get().filter(RowPin::is_valid) && let Some(index)=restore.index_of(pin.key()){restore.reveal(index);}});
            }
            embed(list.view_with(|item|{
                if current.as_ref().is_some_and(|pin|pin.key()==item.key){view!{row(align:Align::Center){label(format!("Editing {}",item.key),width:100);text_field("Document name",bind:draft,key:"name",grow:1);}}}
                else{View::label(item.label())}
            }),grow:1,key:"documents");
            row(shrink:0){
                button("Save",enabled:valid,on_click:move||{
                    if let Some(pin)=save_edit.get().filter(RowPin::is_valid){
                        let mut rows=(0..save.len()).filter_map(|i|save.item(i)).collect::<Vec<_>>();
                        if let Some(index)=save.index_of(pin.key()){rows[index].cells=vec![save_draft.get()];save.set_items(rows).unwrap();save_status.set(format!("Saved row {}",pin.key()));save_edit.set(None);}
                    }
                });
                button("Cancel",enabled:valid,on_click:move||{cancel.set(None);cancel_status.set("Edit released; its row can now unrealize.".into());});
                label(format!("{} realized · {} pinned · {} total",list.realized_indices().len(),list.pinned_keys().len(),list.len()),label_style:LabelStyle::Micro);
            }
            label(status.get(),wrap:true,shrink:0,label_style:LabelStyle::Status);
        }}
    })
}
