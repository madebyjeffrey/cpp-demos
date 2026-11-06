//! No window required: inspect reactive descriptions before stage 2 adds widgets.
use scars_ui::{Host, Kind, State, text, view};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let show = State::new(true);
    let items = State::new(vec!["Alpha", "Beta"]);
    let host = Host::new({
        let show = show.clone();
        let items = items.clone();
        move || {
            view! {
                column {
                    label("Stage 1 — reactive foundation");
                    if show.get() { label("Conditional description", key: "details"); }
                    for item in items.get() { label(item, key: item); }
                button("Toggle details", key: "toggle", on_click: {
                        let show = show.clone();
                        move || show.update(|value| *value = !*value)
                    });
                }
            }
        }
    });

    for caption in ["Initial", "Reordered", "Action dispatched"] {
        if caption == "Reordered" {
            items.set(vec!["Beta", "Alpha"]);
        }
        if caption == "Action dispatched" {
            let button = host
                .snapshot()?
                .into_iter()
                .find(|n| n.kind == Kind::Button)
                .unwrap();
            host.activate(button.id)?;
        }
        println!("\n{caption} (watch Alpha/Beta keep their IDs):");
        for node in host.snapshot()?.into_iter().filter(|n| !n.text.is_empty()) {
            println!("  {}: {}", node.id, node.text);
        }
    }
    println!(
        "\nICU4X de-DE: {}",
        text::IntegerFormatter::new("de-DE")?.format(1_234_567)
    );
    println!(
        "Grapheme boundaries: {:?}",
        text::grapheme_boundaries("A👩‍👩‍👧‍👦é")
    );
    Ok(())
}
