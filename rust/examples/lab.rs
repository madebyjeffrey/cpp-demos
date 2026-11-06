//! Full catalog. Try --page values --light --size 480x740, or --list.
#[path = "lab/view.rs"]
mod catalog;
mod common;
#[path = "lab/model.rs"]
mod model;
use model::Lab;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    let lab = Lab::new()?;
    if args.iter().any(|a| a == "--list") {
        for (id, page) in catalog::SAMPLES {
            println!("{page}: {id}");
        }
        return Ok(());
    }
    if let Some(i) = args.iter().position(|a| a == "--page") {
        let page = args.get(i + 1).ok_or("--page needs a catalog page")?;
        if !catalog::PAGES.iter().any(|(id, _)| id == page) {
            return Err("Unknown catalog page; use --list".into());
        }
        lab.pages.select(page);
    }
    if let Some(i) = args.iter().position(|a| a == "--state") {
        match args.get(i + 1).map(String::as_str) {
            Some("disabled") => lab.enabled.set(false),
            Some("readonly") => lab.set_readonly(true),
            Some("error") => {
                lab.text.set(String::new());
                lab.number.binding().set("invalid".into());
                lab.date.binding().set("30.02.2026".into());
            }
            Some("default") => {}
            _ => return Err("--state needs default, disabled, readonly or error".into()),
        }
    }
    let presentation = if let Some(i) = args.iter().position(|a| a == "--show") {
        let kind = args
            .get(i + 1)
            .ok_or("--show needs a presentation")?
            .clone();
        lab.pages.select(match kind.as_str() {
            "menu" | "dropdown" | "popover" | "alert" => "flow",
            "combo" => "data",
            "color" => "values",
            _ => return Err("--show needs menu, dropdown, popover, alert, combo or color".into()),
        });
        Some(kind)
    } else {
        None
    };
    let preview = lab.clone();
    common::run_prepared(
        "SCARS · Component lab",
        (1120, 900),
        Some(lab.inspector.clone()),
        move || catalog::view(&lab),
        move |host| {
            if let Some(kind) = presentation {
                preview.present(&kind, host)?;
            }
            Ok(())
        },
    )
}
