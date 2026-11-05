#![allow(dead_code)]
use scars_window::{skia, ui::*};
/// Every milestone supports `--render file.png` and `--smoke`.
pub fn run(
    title: &str,
    size: (i32, i32),
    builder: impl Fn() -> View + 'static,
) -> Result<(), Box<dyn std::error::Error>> {
    run_inspected(title, size, None, builder)
}
pub fn run_inspected(
    title: &str,
    size: (i32, i32),
    inspector: Option<Inspector>,
    builder: impl Fn() -> View + 'static,
) -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    if let Some(i) = args.iter().position(|v| v == "--render") {
        let path = args.get(i + 1).ok_or("--render needs a PNG path")?;
        let host = Host::new(builder);
        if let Some(inspector) = inspector.clone() {
            host.set_inspector(inspector);
        }
        if args.iter().any(|v| v == "--light") {
            host.set_theme(Theme::light());
        }
        let renderer = Renderer::default();
        host.layout(Size::new(size.0 as f64, size.1 as f64), &renderer)?;
        if args.iter().any(|v| v == "--inspect") {
            for node in host.scene()? {
                println!("{:?} {} {:?}", node.kind, node.text, node.bounds);
            }
        }
        let mut surface =
            skia::surfaces::raster_n32_premul(size).ok_or("Raster surface unavailable")?;
        surface
            .canvas()
            .clear(skia::Color::new(host.theme().window));
        renderer.paint(&host, surface.canvas(), 0.)?;
        let png = surface
            .image_snapshot()
            .encode(None, skia::EncodedImageFormat::PNG, None)
            .ok_or("PNG encoding failed")?;
        std::fs::write(path, png.as_bytes())?;
        return Ok(());
    }
    #[cfg(target_os = "macos")]
    {
        use scars_window::{Appearance, Application, Window};
        let app = Application::named(title)?;
        let window = app.open_window(
            Window::builder(title)
                .size(size.0 as f64, size.1 as f64)
                .theme_toggle(if args.iter().any(|v| v == "--light") {
                    Appearance::Light
                } else {
                    Appearance::Dark
                })
                .full_screen_button()
                .view(builder),
        )?;
        if let Some(inspector) = inspector {
            window.view_host().unwrap().set_inspector(inspector);
        }
        if args.iter().any(|v| v == "--smoke") {
            app.quit_after(std::time::Duration::from_secs(1));
        }
        app.run()?;
    }
    Ok(())
}
