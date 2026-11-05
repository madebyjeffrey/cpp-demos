//! Scroll, tabs, popovers, menus, alerts and keyboard focus restoration.
mod common;
use scars_window::ui::*;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let selected = State::new("library".to_owned());
    let menu = Popup::new();
    let about = Popup::new();
    let ratio = State::new(0.35);
    let choice = State::new("recent".to_owned());
    let dropdown_popup = Popup::new();
    common::run("Stage 4 · Workspace", (820, 650), move || {
        let (choice_button, choice_menu) = dropdown(
            "Sort",
            &choice,
            vec![
                OptionItem::new("recent", "Recent"),
                OptionItem::new("name", "Name"),
            ],
            &dropdown_popup,
            "sort",
        );
        let open = about.clone();
        let context = menu.clone();
        let sidebar = view! {panel(padding:16,gap:14){label("COLLECTIONS",bold:true);label("Personal");label("Team library");label("Archived");}};
        let library = view! {column(grow:1){
            row {label("Library",font_size:22,bold:true,grow:1);embed(choice_button);}
            scroll(grow:1,on_context:move||context.show()){
                for i in 0..30 {button(format!("Document {:02} · Project notes",i+1),key:format!("doc-{i}"),on_click:||{},tooltip:"Open document");}
            }
        }};
        let preferences = view! {panel(padding:20){label("Workspace preferences",font_size:22);label("Tabs mount only their active content.");}};
        let pages = tabs(
            &selected,
            vec![
                ("library".into(), "Library".into(), library),
                ("preferences".into(), "Preferences".into(), preferences),
            ],
        );
        view! {layer{
            column(padding:24,gap:18){
                row(shrink:0){label("Your workspace",font_size:28,bold:true,grow:1);button("Actions ▾",key:"actions",on_click:menu.open.action(|v|v.update(|v|*v = !*v)));}
                split_view(&ratio,sidebar,pages,grow:1);
                label("Scroll the library. Tab and arrows work in menus. Escape dismisses overlays.",shrink:0);
            }
            embed(menu.menu("actions",vec![MenuAction::new("About this demo",move||open.show()),MenuAction::new("Unavailable action",||{}).enabled(false)]));
            embed(choice_menu);
            embed(about.modal("A portable workspace",View::label("Scrolling, selection and focus are managed by the Rust core.").wrap(true)));
        }}
    })
}
