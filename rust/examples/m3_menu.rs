//! M3 menus: one menu tree and a split action that remembers the chosen leaf.
mod common;
use scars_window::ui::*;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let menu = Menu::new();
    let export = SplitAction::new();
    let guides = State::new(true);
    let quality = State::new("balanced".to_owned());
    let status =
        State::new("Choose an export format, then use its primary button again.".to_owned());
    if std::env::args().any(|s| s == "--render") {
        menu.show();
    }
    common::run("SCARS · Menus", (840, 600), move || {
        let items = vec![
            MenuAction::new("Save", status.action(|s| s.set("Saved".into()))).shortcut("⌘S"),
            MenuAction::new("Print", || {})
                .enabled(false)
                .shortcut("⌘P"),
            MenuAction::separator(),
            MenuAction::check("Show guides", guides.binding()),
            MenuAction::submenu(
                "Quality",
                vec![
                    MenuAction::radio("Fast", quality.binding(), "fast"),
                    MenuAction::radio("Balanced", quality.binding(), "balanced"),
                    MenuAction::radio("Best", quality.binding(), "best"),
                ],
            ),
        ];
        let (export_button, export_menu) = export.view(
            "export",
            vec![
                MenuAction::new(
                    "Export PDF",
                    status.action(|s| s.set("Exported PDF".into())),
                )
                .key("pdf"),
                MenuAction::submenu(
                    "Image",
                    vec![
                        MenuAction::new(
                            "Export PNG",
                            status.action(|s| s.set("Exported PNG".into())),
                        )
                        .key("png"),
                        MenuAction::new(
                            "Export JPEG",
                            status.action(|s| s.set("Exported JPEG".into())),
                        )
                        .key("jpeg"),
                    ],
                ),
            ],
        );
        let body = view! {column(padding:32,gap:20){
            label("Menus that remember",label_style:LabelStyle::Heading);
            row{embed(menu.button("Document ▾","document"));}
            label("Export",label_style:LabelStyle::Section);
            embed(export_button,width:300);
            label(status.get(),wrap:true,label_style:LabelStyle::Status);
            label(format!("Guides: {} · Quality: {}",guides.get(),quality.get()),label_style:LabelStyle::Secondary);
            spacer(grow:1);
            label("↑/↓ or type to choose · → opens a submenu · ←/Escape returns · Enter activates · Tab dismisses",wrap:true,label_style:LabelStyle::Micro);
        }};
        View::layer(vec![body, menu.view("document", items), export_menu])
    })
}
