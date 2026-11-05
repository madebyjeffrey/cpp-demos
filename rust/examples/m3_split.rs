//! M3 split views: retained panes, both axes, collapse/restore and keyboard resizing.
mod common;
use scars_window::ui::*;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let workspace = Split::new(0.28).limits(150., 240.);
    let detail = Split::new(0.65)
        .orientation(Orientation::Vertical)
        .limits(100., 70.);
    let notes=State::new("Try editing this text, then hide and restore the sidebar or lower pane. Drag either divider, or focus one and use arrow keys.".to_owned());
    let title = State::new("Working notes".to_owned());
    common::run("SCARS · Split workspace", (900, 680), move || {
        let toggle = workspace.clone();
        let lower = detail.clone();
        let sidebar = view! {panel(padding:16,gap:14){
            label("Library",label_style:LabelStyle::Heading);
            text_field("Name",bind:title,key:"name");
            scroll(grow:1){for i in 1..=30{button(format!("Document {i}"),key:format!("doc-{i}"),on_click:||{});}}
        }};
        let editor = view! {panel(padding:18,gap:12){label(title.get(),label_style:LabelStyle::Heading);text_editor("Notes",bind:notes,key:"notes",grow:1);}};
        let preview = view! {panel(padding:18,gap:12){label("Details",label_style:LabelStyle::Section);label("Each pane keeps its mounted controls while collapsed. Escape restores a collapsed pane; during a drag it rolls back the resize.",wrap:true,label_style:LabelStyle::Secondary);}};
        view! {column(padding:24,gap:16){
            row(shrink:0){label("A workspace that fits",label_style:LabelStyle::Heading,grow:1);button("Sidebar",on_click:move||{if toggle.collapsed_pane()==SplitPane::First{toggle.restore();}else{toggle.collapse(SplitPane::First);}});button("Details",on_click:move||{if lower.collapsed_pane()==SplitPane::Second{lower.restore();}else{lower.collapse(SplitPane::Second);}});}
            split(&workspace,sidebar,detail.view(editor,preview),grow:1);
            label("Drag either divider · Tab/arrow keys resize · Shift moves farther · Enter/double-click collapses the smaller pane",wrap:true,label_style:LabelStyle::Micro,shrink:0);
        }}
    })
}
