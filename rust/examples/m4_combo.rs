//! A virtual suggestion popup with independent draft and committed selection.
mod common;
use scars_window::ui::*;
fn choices() -> Vec<OptionItem> {
    (0..100_000)
        .map(|i| OptionItem {
            id: i.to_string(),
            label: format!("Document {i:06}"),
            enabled: i % 7 != 3,
        })
        .collect()
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let status = State::new(
        "Type to filter; arrow keys highlight; Return accepts; Escape restores the opening query."
            .to_owned(),
    );
    let combo = ComboBox::new(choices())?
        .on_accept(status.handler(|s, choice: ComboAccept| {
            s.set(format!(
                "Accepted {} ({})",
                choice.text,
                choice.selection.as_deref().unwrap_or("free text")
            ))
        }))
        .on_cancel(status.action(|s| s.set("Canceled; the committed choice is unchanged.".into())));
    let custom = ComboBox::with_options(
        vec![
            OptionItem::new("local", "Local"),
            OptionItem::new("team", "Team"),
        ],
        ComboBoxOptions {
            mode: ComboBoxMode::FreeText,
            auto_highlight: false,
            ..Default::default()
        },
    )?;
    if std::env::args().any(|arg| arg == "--open") {
        combo.show();
    }
    common::run("SCARS · Combo boxes", (840, 620), move || {
        let load = combo.clone();
        let reset = combo.clone();
        let clear = combo.clone();
        let info = combo.info();
        view! {column(padding:28,gap:18){
            label("Choose from a hundred thousand documents",label_style:LabelStyle::Heading,wrap:true,shrink:0);
            label("The draft query and committed choice are independent. Suggestions scroll without a fixed result cap.",wrap:true,label_style:LabelStyle::Secondary,shrink:0);
            row(shrink:0){
                button(if info.loading{"Finish loading"}else{"Simulate loading"},on_click:move||{load.set_loading(!load.info().loading);load.show();});
                button("Replace choices",on_click:move||{reset.set_items(vec![OptionItem::new("new","New document")]).unwrap();reset.set_query("").unwrap();});
                button("Clear selection",on_click:move||{clear.set_selection(None).unwrap();});
            }
            embed(combo.view("Find a document"),shrink:0);
            spacer(grow:1);
            label(format!("{} matches · choice {} · highlighted {}",info.matches,info.selection.as_deref().unwrap_or("—"),info.highlighted.as_deref().unwrap_or("—")),wrap:true,label_style:LabelStyle::Micro,shrink:0);
            label("Free text is also supported",label_style:LabelStyle::Section,shrink:0);
            embed(custom.view("Choose or type a workspace"),shrink:0);
            label(format!("Workspace draft: {}",custom.query()),label_style:LabelStyle::Micro,shrink:0);
            label(status.get(),wrap:true,label_style:LabelStyle::Status,shrink:0);
        }}
    })
}
