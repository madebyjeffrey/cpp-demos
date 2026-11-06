//! Retained tab pages: edit, scroll, switch and undo; arrow/Control-Tab navigation.
mod common;
use scars_window::ui::*;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let pages = Tabs::new("draft");
    let name = State::new("Working draft".to_owned());
    let notes=State::new("Edit these notes, switch tabs and return. Selection, undo history and the list's scroll position stay with this page.".to_owned());
    let available = State::new(false);
    common::run("SCARS · Retained tabs", (900, 660), move || {
        let editor = view! {panel(padding:18,gap:14){
            text_field("Document name",bind:name,key:"name");
            text_editor("Notes",bind:notes,key:"notes",height:130);
            scroll(grow:1,key:"documents"){for i in 1..=50{button(format!("Reference {i}"),on_click:||{},key:format!("reference-{i}"),shrink:0);}}
        }};
        let preview = view! {panel(padding:22,gap:14){label(name.get(),label_style:LabelStyle::Heading);label(notes.get(),wrap:true);spacer(grow:1);label("The editor page stays mounted while you read this preview.",wrap:true,label_style:LabelStyle::Secondary);}};
        view! {column(padding:28,gap:16){
            label("A place for every draft",label_style:LabelStyle::Heading,shrink:0);
            row(shrink:0){label("Edit and scroll, then switch pages and return.",label_style:LabelStyle::Secondary,grow:1);checkbox("Enable Activity",bind:available);}
            tab_view(&pages,vec![Tab::new("draft","Draft",editor),Tab::new("preview","Preview",preview),Tab::new("activity","Activity",View::label("No recent activity")).enabled(available.get())],grow:1,accessible_label:"Document pages");
            label("Arrows/Home/End choose a tab · Tab enters its remembered control · Control-Tab cycles the nearest group",wrap:true,label_style:LabelStyle::Micro,shrink:0);
        }}
    })
}
