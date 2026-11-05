use scars_window::{skia, ui::*};
#[test]
fn themes_images_and_text_render_without_a_gpu() {
    let renderer =
        Renderer::with_font(include_bytes!("../../demo/assets/Roboto-Regular.ttf")).unwrap();
    let image = Bitmap::from_rgba(1, 1, &[255, 0, 0, 255], 4).unwrap();
    let host = Host::new(
        move || view! {column(padding:10){label("Hello Rust");image(image,width:20,height:20);}},
    );
    let mut colors = Vec::new();
    for theme in [Theme::default(), Theme::light()] {
        host.set_theme(theme);
        host.layout(Size::new(200., 100.), &renderer).unwrap();
        let mut surface = skia::surfaces::raster_n32_premul((200, 100)).unwrap();
        surface.canvas().clear(skia::Color::new(theme.window));
        renderer.paint(&host, surface.canvas(), 0.).unwrap();
        let mut pixels = vec![0; 200 * 100 * 4];
        assert!(surface.read_pixels(
            &skia::ImageInfo::new(
                (200, 100),
                skia::ColorType::RGBA8888,
                skia::AlphaType::Unpremul,
                None
            ),
            &mut pixels,
            800,
            (0, 0)
        ));
        assert!(pixels.as_chunks::<4>().0.contains(&[255, 0, 0, 255]));
        colors.push(pixels[..4].to_vec());
    }
    assert_ne!(colors[0], colors[1]);
}
