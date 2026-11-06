//! CPU preview rendering uses logical geometry and an explicit pixel density.
use scars_window::{skia, ui::*};

pub fn render(
    host: &Host,
    renderer: &Renderer,
    size: (i32, i32),
    scale: i32,
) -> Result<skia::Surface, Error> {
    if size.0 <= 0 || size.1 <= 0 || !matches!(scale, 1 | 2) {
        return Err(Error(
            "Use positive logical dimensions and raster scale 1 or 2".into(),
        ));
    }
    let pixels = size
        .0
        .checked_mul(scale)
        .zip(size.1.checked_mul(scale))
        .ok_or_else(|| Error("Raster dimensions overflow".into()))?;
    let mut surface = skia::surfaces::raster_n32_premul(pixels)
        .ok_or_else(|| Error("Raster surface unavailable".into()))?;
    surface
        .canvas()
        .clear(skia::Color::new(host.theme().window));
    surface.canvas().scale((scale as f32, scale as f32));
    renderer.paint(host, surface.canvas(), 0.)?;
    Ok(surface)
}
