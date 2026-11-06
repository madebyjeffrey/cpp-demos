//! Date policy and keyboard navigation without application-side calendar plumbing.
mod common;
use scars_window::ui::*;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let value = State::new(Date::new(2024, 2, 28)?);
    let readonly = State::new(false);
    let blur = State::new(true);
    let submits = State::new(0usize);
    let status = State::new(String::new());
    let picker = DatePicker::new(value.binding())
        .locale(EditLocale::new("en-GB")?)
        .bounds(Date::new(2024, 2, 1)?, Date::new(2024, 4, 30)?)
        .disabled_dates(|d| d.weekday().to_monday_zero_offset() >= 5)
        .validate(|d| {
            if d.day() == 13 {
                Err(Error("The 13th is reserved".into()))
            } else {
                Ok(())
            }
        })
        .on_submit(submits.action(|n| n.update(|n| *n += 1)))
        .on_validation(status.handler(|s, v: Validation| {
            s.set(format!(
                "{:?} · {}",
                v.status,
                v.message.unwrap_or_default()
            ))
        }));
    common::run("SCARS · Date policies", (640, 860), move || {
        view! {
            scroll(padding:24,gap:16) {
                label("Date policies",label_style:LabelStyle::Heading);
                label("February–April 2024. Weekends unavailable; the 13th is reserved. Arrows move the calendar cursor, Page Up/Down changes month, Enter/Space chooses a day.",wrap:true);
                checkbox("Read only",bind:readonly);
                checkbox("Commit field on blur",bind:blur);
                embed(picker.clone().read_only(readonly.get()).commit_on_blur(blur.get()).view());
                label(format!("Committed: {} · Cursor: {}",value.get(),picker.calendar().cursor));
                label(format!("Submissions: {} · {}",submits.get(),status.get()),wrap:true);
                button("External update: 15 March",on_click:value.action(|v|v.set(Date::new(2024,3,15).unwrap())));
            }
        }
    })
}
