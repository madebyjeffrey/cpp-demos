use scars_window::{skia, ui::*};
#[test]
fn straight_alpha_swatches_composite_over_a_checkerboard_without_changing_rgb_payload() {
    let value = State::new(Color::rgba(255, 0, 0, 0));
    let picker = ColorPicker::new(value.binding());
    let host = Host::new(move || picker.view());
    let renderer = Renderer::default();
    let mut surface = skia::surfaces::raster_n32_premul((480, 480)).unwrap();
    let mut samples = vec![];
    for alpha in [0, 128, 255] {
        value.set(Color::rgba(255, 0, 0, alpha));
        host.layout(Size::new(480., 480.), &renderer).unwrap();
        let preview = host
            .scene()
            .unwrap()
            .into_iter()
            .find(|n| n.color_preview.is_some())
            .unwrap();
        surface.canvas().clear(skia::Color::BLACK);
        renderer.paint(&host, surface.canvas(), 0.).unwrap();
        let mut pixels = vec![0u8; 480 * 480 * 4];
        assert!(surface.read_pixels(
            &skia::ImageInfo::new(
                (480, 480),
                skia::ColorType::RGBA8888,
                skia::AlphaType::Unpremul,
                None
            ),
            &mut pixels,
            480 * 4,
            (0, 0)
        ));
        let point = |dx: usize| {
            let x = preview.bounds.x as usize + dx;
            let y = preview.bounds.y as usize + 4;
            pixels[(y * 480 + x) * 4..(y * 480 + x + 1) * 4].to_vec()
        };
        samples.push((point(4), point(12)));
        assert_eq!(value.get().channels(), [255, 0, 0, alpha]);
    }
    assert_eq!(
        samples[0],
        (vec![191, 191, 191, 255], vec![255, 255, 255, 255])
    );
    assert_eq!(samples[2], (vec![255, 0, 0, 255], vec![255, 0, 0, 255]));
    for (actual, expected) in samples[1].0.iter().zip([223u8, 95, 95, 255]) {
        assert!(actual.abs_diff(expected) <= 1);
    }
}
