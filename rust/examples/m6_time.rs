//! One exact value, two clock policies, no application-side editor plumbing.
mod common;
use scars_window::ui::*;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let value = State::new(Time::new(11, 58, 45, 123456789)?);
    let readonly = State::new(false);
    let submits = State::new(0usize);
    let picker = TimePicker::new(value.binding())
        .hour_cycle(HourCycle::H12)
        .show_seconds(true)
        .minute_step(5)
        .second_step(10)
        .bounds(Time::new(9, 0, 0, 0)?, Time::new(17, 0, 0, 0)?)
        .validate(|t| {
            if t.hour() == 13 {
                Err(Error("13:00–13:59 is reserved".into()))
            } else {
                Ok(())
            }
        })
        .on_submit(submits.action(|s| s.update(|n| *n += 1)));
    let compact = TimePicker::new(value.binding());
    common::run("SCARS · Time policies", (640, 900), move || {
        view! {
            scroll(padding:24,gap:16) {
                label("Time policies",label_style:LabelStyle::Heading);
                label("12-hour clock: 09:00–17:00, excluding 13:00–13:59. Minute steps of 5, second steps of 10. Focus a stepper and use arrow keys.",wrap:true);
                checkbox("Read only",bind:readonly);
                embed(picker.clone().read_only(readonly.get()).view());
                label("24-hour clock: hidden seconds and nanoseconds survive hour/minute edits.",wrap:true);
                embed(compact.clone().read_only(readonly.get()).view());
                label(format!("Exact committed value: {}",value.get()),wrap:true);
                label(format!("12-hour submissions: {}",submits.get()));
                button("External update: noon",on_click:value.action(|v|v.set(Time::new(12,0,45,123456789).unwrap())));
            }
        }
    })
}
