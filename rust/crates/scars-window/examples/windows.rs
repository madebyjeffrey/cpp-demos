//! Run with `cargo run --example windows`. `--smoke` exits after one second.
#[cfg(target_os = "macos")]
fn main() -> scars_window::Result<()> {
    use scars_window::*;
    use std::time::Duration;

    const NEW: Command = 1;
    const CLOSE: Command = 2;
    const THEME: Command = 3;
    const MINIMIZE: Command = 4;

    // These menus and shortcuts are choices made by this application.
    fn menus() -> Vec<MenuItem> {
        vec![
            MenuItem::submenu(
                "File",
                vec![
                    MenuItem::action(NEW, "New Window", Shortcut::command(Key::N)),
                    MenuItem::action(CLOSE, "Close", Shortcut::command(Key::W)),
                ],
            ),
            MenuItem::submenu(
                "View",
                vec![MenuItem::action(
                    THEME,
                    "Toggle light / dark",
                    Shortcut::command(Key::L),
                )],
            ),
            MenuItem::submenu(
                "Window",
                vec![MenuItem::action(
                    MINIMIZE,
                    "Minimize",
                    Shortcut::command(Key::M),
                )],
            ),
        ]
    }

    let app = Application::named("Rust Window Lab")?;
    if std::env::args().any(|arg| arg == "--smoke") {
        app.quit_after(Duration::from_secs(1));
    }

    app.run_with_windows(|| {
        Window::builder("Rust Window Lab — native AppKit + Skia")
            .theme_toggle(Appearance::Dark)
            .full_screen_button()
            .menu(menus(), |window, command| match command {
                NEW => window.request_new_window(),
                CLOSE => window.request_close(),
                THEME => window.toggle_theme(),
                MINIMIZE => window.minimize(),
                _ => (),
            })
            .on_draw(|window, frame| {
                frame.clear(match window.effective_appearance() {
                    Appearance::Light => [0.94, 0.95, 0.97, 1.0],
                    _ => [0.035, 0.065, 0.11, 1.0],
                });
                let mut accent = skia::Paint::default();
                accent
                    .set_anti_alias(true)
                    .set_color(skia::Color::from_rgb(75, 140, 245));
                frame.canvas().draw_round_rect(
                    skia::Rect::from_xywh(32., 32., 160., 80.),
                    16.,
                    16.,
                    &accent,
                );
            })
            .on_event(|window, event| {
                if matches!(event, Event::Key(key) if key.down && key.key == Key::Escape) {
                    window.request_close();
                }
            })
    })
}

#[cfg(not(target_os = "macos"))]
fn main() {
    eprintln!("This example requires macOS 26+");
}
