//! Validated setup flow with stable steps, retained editing and review navigation.
mod common;
use scars_window::ui::*;
fn required(value: &State<String>, message: &str) -> impl Fn() -> Result<(), String> + 'static {
    let value = value.clone();
    let message = message.to_owned();
    move || {
        if value.get().trim().is_empty() {
            Err(message.clone())
        } else {
            Ok(())
        }
    }
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let name = State::new(String::new());
    let destination = State::new(String::new());
    let result = State::new(None::<String>);
    let activity = State::new("Complete the first step to continue.".to_owned());
    let setup = Wizard::new(
        "identity",
        vec![
            WizardStep::new("identity", "Identity")
                .validate(required(&name, "Enter a project name.")),
            WizardStep::new("destination", "Destination")
                .validate(required(&destination, "Choose a destination folder.")),
            WizardStep::new("review", "Review"),
        ],
    )?
    .on_finish(result.action(|s| s.set(Some("Project created.".into()))))
    .on_cancel(result.action(|s| s.set(Some("Setup cancelled.".into()))))
    .on_change(activity.handler(|s, c: WizardChange| s.set(format!("{} → {}", c.from, c.to))));
    common::run("SCARS · Validated wizard", (860, 600), move || {
        let pages = vec![
            (
                "identity",
                view! {panel(padding:22,gap:16){label("Name your project",label_style:LabelStyle::Heading);text_field("Project name",bind:name,key:"project-name",autofocus:true);label("Next stays disabled until the current step is valid. Editing and undo survive Back and review navigation.",wrap:true,label_style:LabelStyle::Secondary);}},
            ),
            (
                "destination",
                view! {panel(padding:22,gap:16){label("Where should it live?",label_style:LabelStyle::Heading);text_field("Destination folder",bind:destination,key:"folder");label("Back is available even when this step is incomplete.",wrap:true,label_style:LabelStyle::Secondary);}},
            ),
            (
                "review",
                view! {panel(padding:22,gap:16){label("Ready to create",label_style:LabelStyle::Heading);label(format!("Project: {}",name.get()));label(format!("Folder: {}",destination.get()));label("Finish validates every step again. Use the step buttons to review earlier answers.",wrap:true,label_style:LabelStyle::Secondary);}},
            ),
        ];
        let again = setup.clone();
        let reset = result.clone();
        view! {column(padding:28,gap:18){
            label("A setup flow that checks its work",label_style:LabelStyle::Heading,shrink:0);
            if let Some(message)=result.get(){panel(padding:24,grow:1){label(message,label_style:LabelStyle::Status);button("Review again",on_click:move||{again.go_to("identity");reset.set(None);});}}
            else{wizard_view(&setup,pages,grow:1);}
            label(activity.get(),wrap:true,label_style:LabelStyle::Micro,shrink:0);
        }}
    })
}
