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
    run_prepared(title, size, inspector, builder, |_| Ok(()))
}
/// Optional CLI preview actions run after geometry exists, before the first paint.
pub fn run_prepared(
    title: &str,
    size: (i32, i32),
    inspector: Option<Inspector>,
    builder: impl Fn() -> View + 'static,
    prepare: impl FnOnce(&Host) -> Result<(), Error>,
) -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    let size = if let Some(i) = args.iter().position(|v| v == "--size") {
        let (width, height) = args
            .get(i + 1)
            .and_then(|v| v.split_once('x'))
            .ok_or("--size needs WIDTHxHEIGHT")?;
        let size = (width.parse::<i32>()?, height.parse::<i32>()?);
        if size.0 <= 0 || size.1 <= 0 {
            return Err("--size must be positive".into());
        }
        size
    } else {
        size
    };
    if let Some(i) = args.iter().position(|v| v == "--render") {
        let path = args.get(i + 1).ok_or("--render needs a PNG path")?;
        let host = Host::new(builder);
        if let Some(inspector) = inspector.clone() {
            host.set_inspector(inspector);
        }
        if let Some(i) = args.iter().position(|v| v == "--inspector-page") {
            host.inspector()
                .show_page(match args.get(i + 1).map(String::as_str) {
                    Some("layout") => InspectorPage::Layout,
                    Some("style") => InspectorPage::Style,
                    Some("details") => InspectorPage::Details,
                    _ => return Err("--inspector-page needs details, layout or style".into()),
                });
        }
        if args.iter().any(|v| v == "--light") {
            host.set_theme(Theme::light());
        }
        let renderer = Renderer::default();
        host.layout(Size::new(size.0 as f64, size.1 as f64), &renderer)?;
        if reveal(&args, &host)? {
            host.layout(Size::new(size.0 as f64, size.1 as f64), &renderer)?;
        }
        prepare(&host)?;
        host.layout(Size::new(size.0 as f64, size.1 as f64), &renderer)?;
        if let Some(i) = args.iter().position(|v| v == "--select-key") {
            let key = args.get(i + 1).ok_or("--select-key needs a mounted key")?;
            let inspector = host.inspector();
            let id = inspector
                .nodes()
                .iter()
                .find(|n| n.key.as_deref() == Some(key))
                .map(|n| n.id)
                .ok_or("No inspector node matches --select-key")?;
            inspector.select(id);
            if args.iter().any(|v| v == "--override-preview") {
                inspector.set_override(
                    id,
                    StyleOverride::default()
                        .width(percent(80.))
                        .padding(12)
                        .radius(12)
                        .bold(true),
                )?;
            }
            host.layout(Size::new(size.0 as f64, size.1 as f64), &renderer)?;
        }
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
        let host = window.view_host().unwrap();
        let bounds = window.content_size();
        let renderer = Renderer::default();
        let viewport = Size::new(bounds.width, bounds.height);
        host.layout(viewport, &renderer)?;
        reveal(&args, &host)?;
        prepare(&host)?;
        host.layout(viewport, &renderer)?;
        if args.iter().any(|v| v == "--smoke") {
            app.quit_after(std::time::Duration::from_secs(1));
        }
        app.run()?;
    }
    Ok(())
}
fn reveal(args: &[String], host: &Host) -> Result<bool, Error> {
    let Some(i) = args.iter().position(|v| v == "--reveal-key") else {
        return Ok(false);
    };
    let key = args
        .get(i + 1)
        .ok_or_else(|| Error("--reveal-key needs a mounted key".into()))?;
    let target = host
        .snapshot()?
        .into_iter()
        .find(|n| n.key.as_ref() == Some(key))
        .ok_or_else(|| Error("No node matches --reveal-key".into()))?;
    host.reveal_rect(target.id, target.bounds);
    Ok(true)
}
