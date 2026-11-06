//! One checked setting for numeric editing and Gregorian calendar text.
mod common;
use scars_window::ui::*;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let pages = ["en-US", "en-GB", "de-DE", "fr-FR", "ar-EG"]
        .into_iter()
        .map(|tag| {
            let locale = EditLocale::new(tag)?;
            let exact = State::new(9_007_199_254_740_993i64);
            let amount = State::new(-1234.5);
            let date = State::new(Date::new(2026, 3, 4).unwrap());
            Ok((
                tag,
                Numeric::new(exact.binding(), i64::MIN, i64::MAX, 1).locale(locale.clone()),
                Numeric::new(amount.binding(), -1e10, 1e10, 0.5).locale(locale.clone()),
                DatePicker::new(date.binding()).locale(locale),
            ))
        })
        .collect::<Result<Vec<_>, Error>>()?;
    let args: Vec<_> = std::env::args().collect();
    let selected = args
        .windows(2)
        .find(|p| p[0] == "--locale")
        .map_or("de-DE", |p| p[1].as_str());
    EditLocale::new(selected)?;
    let tabs = Tabs::new(selected);
    common::run("SCARS · Localized values", (720, 920), move || {
        let panels = pages.iter().map(|(tag, exact, amount, date)| {
            Tab::new(*tag, *tag, view! {scroll(padding:24,gap:16){
                label("Localized values",label_style:LabelStyle::Heading);
                label(format!("{} · Enter numbers and dates using this profile. Apply or Enter commits; Revert restores the value.",tag),wrap:true);
                numeric_field("Exact integer",exact);
                numeric_field("Amount",amount);
                embed(date.view());
            }})
        }).collect();
        tabs.view(panels)
            .accessible_label("Editing locale examples")
    })
}
