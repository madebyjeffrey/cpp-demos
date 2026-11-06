//! Ordinary application composition with semantic state supplied by the controls.
mod common;
use scars_window::ui::*;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let name = State::new("New project".to_owned());
    let copies = State::new(3i64);
    let count = Numeric::new(copies.binding(), 0, 10, 1);
    let compact = State::new(false);
    let tabs = Tabs::new("general");
    let menu = Menu::new();
    let active = State::new(None::<Alert>);
    let build = move || {
        let review = active.clone();
        let validation = Validation {
            status: if name.get().is_empty() {
                ValidationStatus::Incomplete
            } else {
                ValidationStatus::Valid
            },
            message: name.get().is_empty().then(|| "Enter a project name".into()),
            dirty: name.get().is_empty(),
        };
        let general = view! {column(gap:12){
         label("Project name");
         text_field("Name",bind:name,accessible_label:"Project name",accessible_help:"Used in the project list",validation:validation.clone());
         label(validation.message.unwrap_or_default(),hidden:!name.get().is_empty(),label_style:LabelStyle::Error);
         numeric_field("Copies",&count);
         checkbox("Compact layout",bind:compact);
        }};
        let content = tabs
            .view(vec![
                Tab::new("general", "General", general),
                Tab::new(
                    "details",
                    "Details",
                    View::label("Names and copy counts remain available when switching tabs.")
                        .wrap(true),
                ),
            ])
            .accessible_label("Project settings");
        let body = view! {column(padding:28,gap:18){
         label("Project settings",label_style:LabelStyle::Heading);
         embed(menu.button("Options","options"));
         embed(content,grow:1);
         button("Review",on_click:move||review.set(Some(Alert::new("Apply settings?",vec![AlertAction::new("cancel","Cancel").role(AlertActionRole::Cancel),AlertAction::new("apply","Apply").role(AlertActionRole::Default)],|_|{}).unwrap())));
        }};
        View::layer(vec![
            body,
            menu.view(
                "options",
                vec![
                    MenuAction::check("Compact layout", compact.binding()),
                    MenuAction::submenu(
                        "More",
                        vec![MenuAction::new("Reset copies", copies.action(|v| v.set(3)))],
                    ),
                ],
            ),
            active.get().map_or_else(
                || View::fragment(vec![]),
                |a| a.view(View::label("Apply these project settings?").wrap(true)),
            ),
        ])
    };
    if std::env::args().any(|a| a == "--semantics") {
        let host = Host::new(build);
        host.layout(Size::new(640., 560.), &Renderer::default())?;
        for n in host
            .snapshot()?
            .iter()
            .filter(|n| n.validation.is_some() || n.menu.is_some() || n.tabs.is_some())
        {
            println!(
                "{:?} {:?}: validation={:?}, menu={:?}, help={:?}",
                n.kind, n.label, n.validation, n.menu, n.help
            )
        }
        return Ok(());
    }
    common::run("SCARS · Project settings", (640, 560), build)
}
