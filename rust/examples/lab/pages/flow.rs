use super::*;
pub fn view(m: &Lab) -> View {
    let (choice, choice_menu) = dropdown(
        "Mode",
        &m.choice,
        vec![OptionItem::new("a", "Alpha"), OptionItem::new("b", "Beta")],
        &m.dropdown,
        "lab-choice",
    );
    let (primary, export_menu) = m.export.view(
        "lab-export",
        vec![
            MenuAction::new(
                "Export text",
                m.status.action(|s| s.set("Exported text".into())),
            )
            .key("text"),
            MenuAction::submenu(
                "Image",
                vec![
                    MenuAction::new(
                        "Export PNG",
                        m.status.action(|s| s.set("Exported PNG".into())),
                    )
                    .key("png"),
                ],
            ),
        ],
    );
    let menu_body = view! {column(gap:12){embed(m.menu.button("Actions","lab-actions"));embed(choice);embed(primary);}};
    let menus = View::layer(vec![
        menu_body,
        choice_menu,
        export_menu,
        m.menu.view(
            "lab-actions",
            vec![
                MenuAction::new(
                    "Run action",
                    m.status.action(|s| s.set("Menu action ran".into())),
                )
                .shortcut("⌘R"),
                MenuAction::check("Enabled", m.checked.binding()),
                MenuAction::separator(),
                MenuAction::submenu(
                    "Mode",
                    vec![
                        MenuAction::radio("Alpha", m.choice.binding(), "a"),
                        MenuAction::radio("Beta", m.choice.binding(), "b"),
                    ],
                ),
                MenuAction::new("Unavailable", || {}).enabled(false),
            ],
        ),
    ]);
    let (open, close, confirm) = (m.popup.clone(), m.popup.clone(), m.clone());
    let overlays=view!{layer{
        column(gap:12){
            button("Open popover",key:"lab-popover",on_click:move||open.show(),tooltip:"Shows a fitted, focus-scoped popover");
            button("Ask for confirmation",on_click:move||confirm.confirm(),key:"confirm");
            label("Right-click this paragraph for a context menu. Hover the button for a tooltip.",wrap:true);
        }
        embed(m.popup.popover("lab-popover",view!{group_box("Details",padding:16,gap:12){label("A portable popover",wrap:true);button("Close",on_click:move||close.dismiss());}}));
        embed(m.active.get().map_or_else(||View::fragment(vec![]),|a|a.view(View::label("Return applies; Escape cancels.").wrap(true))));
    }}.context_menu(&m.context,vec![MenuAction::new("Context action",m.status.action(|s|s.set("Context action ran".into())))]);
    let split = m
        .split
        .view(
            View::label("First pane").wrap(true),
            m.vertical.view(
                View::label("Top").wrap(true),
                View::label("Bottom").wrap(true),
            ),
        )
        .height(190);
    let tabs = m
        .tabs
        .view(vec![
            Tab::new(
                "first",
                "Editable",
                View::text_field("Retained text", m.text.binding())
                    .read_only(m.readonly.get())
                    .height(34)
                    .key("retained-name"),
            ),
            Tab::new("second", "Preview", View::label(m.text.get()).wrap(true)),
            Tab::new("off", "Disabled", View::label("Unavailable")).enabled(false),
        ])
        .height(150);
    let wizard = wizard_view(
        &m.wizard,
        vec![
            (
                "name",
                View::text_field("Project name", m.text.binding())
                    .read_only(m.readonly.get())
                    .key("wizard-name"),
            ),
            (
                "review",
                View::label(format!("Create {}?", m.text.get())).wrap(true),
            ),
        ],
    );
    grid(vec![
        card(m, "menus", "Menus and split actions", menus),
        card(m, "overlays", "Popovers, context and alerts", overlays),
        card(m, "splits", "Two-axis split panes", split),
        card(m, "tabs", "Retained tab pages", tabs),
        card(m, "wizard", "Validated wizard", wizard),
    ])
}
