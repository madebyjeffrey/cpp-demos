//! Transactional alerts. Add --sheet for a native parent-modal macOS sheet.
mod common;
use scars_window::ui::*;
fn confirmation(status: &State<String>) -> Alert {
    Alert::new(
        "Save changes to Kestrel?",
        vec![
            AlertAction::new("cancel", "Cancel").role(AlertActionRole::Cancel),
            AlertAction::new("discard", "Discard changes").role(AlertActionRole::Destructive),
            AlertAction::new("save", "Save").role(AlertActionRole::Default),
        ],
        status.handler(|s, result: AlertResult| {
            s.set(format!(
                "Completed once: {} ({:?})",
                result.action.as_deref().unwrap_or("no action"),
                result.reason
            ))
        }),
    )
    .unwrap()
}
fn content() -> View {
    view! {label("Return saves. Escape cancels. Discard is explicit. Every path produces one completion result.",wrap:true,label_style:LabelStyle::Secondary);}
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let status = State::new("Changes have not been saved.".to_owned());
    if std::env::args().any(|a| a == "--sheet") {
        if std::env::args().any(|a| a == "--render") {
            let alert = confirmation(&status);
            return common::run("SCARS · Sheet content", (460, 300), move || {
                alert.sheet_view(content())
            });
        }
        return native_sheet(status);
    }
    let active = State::new(confirmation(&status));
    common::run("SCARS · Alert transactions", (760, 460), move || {
        let open = active.clone();
        let status_for_open = status.clone();
        view! {layer {
            column(padding:28,gap:18){
                label("One question. One result.",label_style:LabelStyle::Heading);
                label("An alert traps focus until it completes. The original control regains focus when it closes.",wrap:true);
                button("Show confirmation",on_click:move||open.set(confirmation(&status_for_open)));
                label(status.get(),wrap:true,label_style:LabelStyle::Status);
            }
            embed(active.get().view(content()));
        }}
    })
}
#[cfg(target_os = "macos")]
fn native_sheet(status: State<String>) -> Result<(), Box<dyn std::error::Error>> {
    use scars_window::{Appearance, Application, Window};
    let app = Application::named("SCARS · Native sheets")?;
    let initial = status.clone();
    let window = app.open_window(Window::builder("Sheet parent").size(760.,460.).theme_toggle(if std::env::args().any(|a| a == "--light") { Appearance::Light } else { Appearance::Dark })
        .view_with_surface(move |surface| {
            let surface = surface.clone(); let completion = status.clone(); let errors = status.clone();
            view! {column(padding:28,gap:18){
                label("A native sheet, portable content",label_style:LabelStyle::Heading);
                label("This parent is isolated while the sheet is open. Other application windows remain usable.",wrap:true);
                button("Show confirmation sheet",on_click:move||{
                    if let Err(error)=surface.show_sheet(confirmation(&completion),content){errors.set(error.to_string());}
                });
                label(status.get(),wrap:true,label_style:LabelStyle::Status);
            }}
        }))?;
    // Opening immediately makes --sheet --smoke exercise an actual attached sheet.
    window.show_sheet(confirmation(&initial), content)?;
    if std::env::args().any(|a| a == "--smoke") {
        app.quit_after(std::time::Duration::from_secs(1));
    }
    app.run()?;
    Ok(())
}
#[cfg(not(target_os = "macos"))]
fn native_sheet(_: State<String>) -> Result<(), Box<dyn std::error::Error>> {
    Err("Native sheets require macOS".into())
}
