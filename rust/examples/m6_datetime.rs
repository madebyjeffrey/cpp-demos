//! One shared draft across an overnight range; no separate date/time state.
mod common;
use scars_window::ui::*;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let initial: DateTime = "2026-11-02T23:00:45.123456789".parse()?;
    let value = State::new(initial);
    let readonly = State::new(false);
    let blur = State::new(true);
    let submits = State::new(0usize);
    let picker = DateTimePicker::new(value.binding())
        .bounds("2026-11-02T22:00".parse()?, "2026-11-03T02:00".parse()?)
        .show_seconds(true)
        .minute_step(15)
        .validate(|v| {
            if v.time().minute() == 30 {
                Err(Error("The half hour is reserved".into()))
            } else {
                Ok(())
            }
        })
        .on_submit(submits.action(|s| s.update(|n| *n += 1)));
    common::run("SCARS · Combined datetime", (680, 900), move || {
        view! {
            scroll(padding:24,gap:16) {
                label("Combined datetime",label_style:LabelStyle::Heading);
                label("22:00 on 2 November through 02:00 on 3 November. Edit date and time together, then Apply. Try 2026-11-03T01:00. Hour steps carry across midnight; minute 30 is reserved.",wrap:true);
                row {
                    checkbox("Read only",bind:readonly);
                    checkbox("Commit on blur",bind:blur);
                }
                embed(picker.clone().read_only(readonly.get()).commit_on_blur(blur.get()).view());
                label(format!("Exact committed value: {}",value.get()),wrap:true);
                label(format!("Submissions: {} · {:?}",submits.get(),picker.validation().status));
                button("External reset",on_click:value.action(move |v|v.set(initial)));
            }
        }
    })
}
