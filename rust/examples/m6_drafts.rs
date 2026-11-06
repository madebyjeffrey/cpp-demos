//! A shared application acceptance policy across the six typed editors.
mod common;
use scars_window::ui::*;
fn gated<T: Clone + PartialEq + 'static>(value: &State<T>, allow: &State<bool>) -> Binding<T> {
    let (read, write, allow) = (value.clone(), value.clone(), allow.clone());
    Binding::new(
        move || read.get(),
        move |v| {
            if allow.get() {
                write.set(v)
            }
        },
    )
}
fn sample(body: View, value: impl std::fmt::Display, validation: Validation) -> View {
    view! { column(gap:12) {
        label(format!("Committed: {value}"),wrap:true);
        label(format!("{:?} · {}",validation.status,if validation.dirty {"edited"} else {"unchanged"}));
        embed(body);
    } }
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let allow = State::new(true);
    let n = State::new(3i64);
    let d = State::new("2026-11-06".parse::<Date>()?);
    let t = State::new("12:30".parse::<Time>()?);
    let dt = State::new("2026-11-06T12:30".parse::<DateTime>()?);
    let z = State::new("2026-11-06T19:30:00Z".parse::<Timestamp>()?);
    let c = State::new(Color::rgba(68, 136, 204, 180));
    let number = Numeric::new(gated(&n, &allow), 0, 10, 1).commit_on_blur(false);
    let date = DatePicker::new(gated(&d, &allow)).commit_on_blur(false);
    let time = TimePicker::new(gated(&t, &allow)).commit_on_blur(false);
    let datetime = DateTimePicker::new(gated(&dt, &allow)).commit_on_blur(false);
    let zoned = ZonedPicker::new(gated(&z, &allow), "America/Edmonton")?.commit_on_blur(false);
    let color = ColorPicker::new(gated(&c, &allow));
    let tabs = Tabs::new("number");
    common::run("SCARS · Shared drafts", (760, 940), move || {
        let body = tabs.view(vec![
            Tab::new(
                "number",
                "Number",
                sample(
                    numeric_field("Count (0–10)", &number),
                    n.get(),
                    number.validation(),
                ),
            ),
            Tab::new(
                "date",
                "Date",
                sample(date.view(), d.get(), date.validation()),
            ),
            Tab::new(
                "time",
                "Time",
                sample(time.view(), t.get(), time.validation()),
            ),
            Tab::new(
                "datetime",
                "Datetime",
                sample(datetime.view(), dt.get(), datetime.validation()),
            ),
            Tab::new(
                "zone",
                "Zoned",
                sample(zoned.view(), z.get(), zoned.validation()),
            ),
            Tab::new(
                "color",
                "Color",
                sample(color.view(), c.get(), color.validation()),
            ),
        ]);
        view! { scroll { column(padding:24,gap:16) {
            label("Shared draft lifecycle",label_style:LabelStyle::Heading);
            label("Edits wait for Enter or Apply. Escape cancels. Turn acceptance off, change a value, then apply: the text stays with an error. Turn acceptance on and retry.",wrap:true);
            checkbox("Application accepts updates",bind:allow);
            embed(body);
        } } }
    })
}
