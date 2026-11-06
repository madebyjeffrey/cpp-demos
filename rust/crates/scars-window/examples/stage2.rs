//! Basic controls, reactive bindings, keyboard focus, and native IME input.
mod common;
use scars_window::ui::*;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let name = State::new("Alex Morgan".to_owned());
    let notifications = State::new(true);
    let sync = State::new(false);
    let mode = State::new("balanced".to_owned());
    let quality = State::new(0.65);
    let saved = State::new(0);
    common::run("Stage 2 · Preferences", (720, 620), move || {
        view! {
            column(padding: 28, gap: 20) {
                label("Make it yours", font_size: 28, bold: true);
                label("Rust components · shared state · native text input");
                row(gap: 24, grow: 1) {
                    column(grow: 1, width: percent(55.), gap: 14) {
                        section("PROFILE", gap: 12) {
                            text_field("Your name", bind: name, accessible_label: "Display name");
                            label(format!("Hello, {}", name.get()));
                        }
                        separator();
                        section("WORKFLOW", gap: 8) {
                            checkbox("Desktop notifications", bind: notifications);
                            switch("Sync across devices", bind: sync);
                            toggle("Focus mode", bind: sync);
                        }
                        spacer(grow: 1);
                        button("Save preferences", on_click: saved.action(|n| n.update(|n| *n += 1)));
                        label(format!("Saved {} times", saved.get()));
                    }
                    panel(padding: 18, width: percent(42.), gap: 16) {
                        radio_group("Rendering", bind: mode, options: vec![
                            OptionItem::new("fast", "Performance"),
                            OptionItem::new("balanced", "Balanced"),
                            OptionItem::new("best", "Best quality")
                        ]);
                        label("Quality", bold: true);
                        slider(bind: quality, min: 0, max: 1, step: 0.05);
                        progress(bind: quality);
                        label(format!("{}%", (quality.get() * 100.).round()));
                        label("Tab to move focus. Use arrows on sliders and selection controls.", wrap: true);
                    }
                }
            }
        }
    })
}
