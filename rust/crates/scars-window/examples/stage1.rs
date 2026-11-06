//! Stage 1 diagnostic: Space activates an action; R reorders keyed descriptions.
//! Run with `cargo run --example stage1`.
#[cfg(target_os = "macos")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use scars_ui::{Host, Kind, State, view};
    use scars_window::{Appearance, Application, Event, Key, Window, skia};
    use skia::textlayout::{FontCollection, ParagraphBuilder, ParagraphStyle, TextStyle};

    let count = State::new(0);
    let items = State::new(vec!["Alpha", "Beta"]);
    let host = Host::new({
        let items = items.clone();
        move || {
            view! {
                column {
                    label(format!("Actions: {}", count.get()));
                    if count.get() > 0 { label("Reactive conditional is now visible", key: "details"); }
                    for item in items.get() { label(item, key: item); }
                button("Increment", key: "increment", on_click: {
                        let count = count.clone();
                        move || count.update(|value| *value += 1)
                    });
                }
            }
        }
    });
    let mut fonts = FontCollection::new();
    fonts.set_default_font_manager(skia::FontMgr::new(), None);
    let draw = host.clone();
    let actions = host.clone();
    let app = Application::named("SCARS foundation")?;
    let window = app.open_window(
        Window::builder("Stage 1 — reactive descriptions")
            .theme_toggle(Appearance::Dark)
            .full_screen_button()
            .on_draw(move |window, frame| {
                let light = window.effective_appearance() == Appearance::Light;
                frame.clear(if light { [0.95, 0.96, 0.98, 1.] } else { [0.04, 0.06, 0.10, 1.] });
                let nodes = draw.snapshot().expect("valid foundation descriptions");
                let lines = nodes.iter().filter(|n| !n.text.is_empty())
                    .map(|n| format!("{}  #{}", n.text, n.id)).collect::<Vec<_>>().join("\n");
                let mut text = TextStyle::new();
                text.set_font_size(18.).set_color(if light { skia::Color::BLACK } else { skia::Color::WHITE });
                let mut style = ParagraphStyle::new();
                style.set_text_style(&text);
                let mut builder = ParagraphBuilder::new(&style, fonts.clone());
                builder.add_text(format!("STAGE 1\nSpace: activate • R: reorder • Escape: close\n\n{lines}\n\nIDs follow Alpha/Beta when reordered.\nDescriptions only; widget layout and input arrive in stage 2."));
                let mut paragraph = builder.build();
                paragraph.layout((frame.info.logical_size.width - 48.) as f32);
                paragraph.paint(frame.canvas(), (24., 24.));
            })
            .on_event(move |window, event| {
                if let Event::Key(key) = event {
                    if !key.down || key.repeat { return; }
                    match key.key {
                        Key::Space => {
                            let button = actions.snapshot().unwrap().into_iter().find(|n| n.kind == Kind::Button).unwrap();
                            actions.activate(button.id).unwrap();
                        }
                        Key::R => items.update(|items| items.reverse()),
                        Key::Escape => window.request_close(),
                        _ => (),
                    }
                }
            }),
    )?;
    host.on_invalidate({
        let window = window.downgrade();
        move || {
            if let Some(window) = window.upgrade() {
                window.request_redraw();
            }
        }
    });
    if std::env::args().any(|arg| arg == "--smoke") {
        app.quit_after(std::time::Duration::from_secs(1));
    }
    app.run()?;
    Ok(())
}

#[cfg(not(target_os = "macos"))]
fn main() {
    eprintln!("For the portable example: cargo run -p scars-ui --example foundation");
}
