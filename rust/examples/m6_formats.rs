//! Small editing lab: explicit ASCII/ISO, precise values, no locale inference.
mod common;
use scars_window::ui::*;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let exact = State::new(9_007_199_254_740_993i64);
    let number = Numeric::new(exact.binding(), i64::MIN, i64::MAX, 1);
    let date = State::new(Date::new(2024, 2, 29)?);
    let calendar = DatePicker::new(date.binding());
    let time = State::new(Time::new(14, 30, 0, 123_456_789)?);
    let clock = time_draft(time.binding());
    let local = State::new(date.get().to_datetime(time.get()));
    let timestamp = datetime_draft(local.binding());
    common::run("SCARS · Editable formats", (660, 700), move || {
        view! {
            scroll(padding:24,gap:16) {
                label("Explicit editing formats",label_style:LabelStyle::Heading);
                label("Numbers use ASCII digits without grouping. Date and time fields use ISO syntax; offsets belong in a zoned editor.",wrap:true);
                numeric_field("Exact integer",&number);
                label(format!("Committed integer: {}",exact.get()));
                embed(value_field("Date · YYYY-MM-DD",&calendar.draft));
                embed(value_field("Time · HH:MM[:SS[.fraction]]",&clock));
                embed(value_field("Local · YYYY-MM-DDTHH:MM[:SS]",&timestamp));
                label(format!("Committed: {} · {} · {}",date.get(),time.get(),local.get()),wrap:true);
            }
        }
    })
}
