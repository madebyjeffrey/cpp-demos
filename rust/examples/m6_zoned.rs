//! Explicit wall time, instant limits and named-zone changes with one civil draft.
mod common;
use scars_window::ui::*;
fn setup(value: State<Timestamp>, picker: ZonedPicker, instant: &'static str) -> impl Fn() {
    let instant = instant.parse().unwrap();
    move || {
        value.set(instant);
        picker.cancel();
        picker
            .change_zone("America/Edmonton", ZoneChange::KeepInstant)
            .unwrap();
    }
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let value = State::new("2024-11-03T06:30:45.123456789Z".parse::<Timestamp>()?);
    let readonly = State::new(false);
    let picker = ZonedPicker::new(value.binding(), "America/Edmonton")?
        .show_seconds(true)
        .minute_step(15)
        .bounds(
            "2024-01-01T00:00:00Z".parse()?,
            "2024-12-31T23:59:59.999999999Z".parse()?,
        );
    common::run("SCARS · Zoned datetime", (760, 900), move || {
        view! {
            scroll { column(padding:24,gap:16) {
                label("Zoned datetime",label_style:LabelStyle::Heading);
                label("Try 2024-11-03T01:30:45.123456789: it occurs twice in Edmonton. Choose Earlier/Later, then Apply. Apply occurrence changes an existing fold. Gaps stay invalid. Zone changes explicitly keep the instant or wall time.",wrap:true);
                checkbox("Read only",bind:readonly);
                label(format!("Instant: {} · {:?}",value.get(),picker.validation().status),wrap:true);
                row {
                    button("Load spring gap setup",on_click:setup(value.clone(),picker.clone(),"2024-03-10T08:30:45.123456789Z"));
                    button("Load autumn fold setup",on_click:setup(value.clone(),picker.clone(),"2024-11-03T06:30:45.123456789Z"));
                }
                embed(picker.clone().read_only(readonly.get()).view());
            } }
        }
    })
}
