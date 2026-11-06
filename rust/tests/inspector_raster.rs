use scars_window::{skia, ui::*};
fn pixels(host: &Host, renderer: &Renderer) -> Vec<u8> {
    host.layout(Size::new(260., 180.), renderer).unwrap();
    let mut surface = skia::surfaces::raster_n32_premul((260, 180)).unwrap();
    surface.canvas().clear(skia::Color::BLACK);
    renderer.paint(host, surface.canvas(), 0.).unwrap();
    let mut pixels = vec![0u8; 260 * 180 * 4];
    assert!(surface.read_pixels(
        &skia::ImageInfo::new(
            (260, 180),
            skia::ColorType::RGBA8888,
            skia::AlphaType::Unpremul,
            None
        ),
        &mut pixels,
        260 * 4,
        (0, 0)
    ));
    pixels
}
#[test]
fn inspector_highlights_respect_effective_clipping_and_hidden_selection() {
    let inspector = Inspector::new();
    let hidden = State::new(false);
    let host = Host::new({
        let h = hidden.clone();
        move || {
            View::column(vec![
                View::scroll(vec![
                    View::button("Target", || {})
                        .height(100)
                        .shrink(0.)
                        .hidden(h.get())
                        .key("target"),
                ])
                .width(200)
                .height(30),
            ])
            .padding(10.)
        }
    });
    host.set_inspector(inspector.clone());
    let renderer = Renderer::default();
    let before = pixels(&host, &renderer);
    let target = host
        .snapshot()
        .unwrap()
        .into_iter()
        .find(|n| n.key.as_deref() == Some("target"))
        .unwrap();
    assert!(target.bounds.height > target.clip.height);
    inspector.select(target.id);
    let after = pixels(&host, &renderer);
    let mut inside_changed = false;
    for y in 0..180 {
        for x in 0..260 {
            let index = (y * 260 + x) * 4;
            if !target.clip.contains(Point {
                x: x as f64 + 0.5,
                y: y as f64 + 0.5,
            }) {
                assert_eq!(
                    before[index..index + 4],
                    after[index..index + 4],
                    "outline escaped clip at {x},{y}"
                );
            } else {
                inside_changed |= before[index..index + 4] != after[index..index + 4];
            }
        }
    }
    assert!(inside_changed);
    hidden.set(true);
    let selected_hidden = pixels(&host, &renderer);
    assert!(inspector.selected.get().unwrap().hidden);
    inspector.selected.set(None);
    let unselected_hidden = pixels(&host, &renderer);
    assert_eq!(selected_hidden, unselected_hidden);
}
