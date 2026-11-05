use scars_window::{Frame, skia};
use skia::{AlphaType, Color, ColorType, FontMgr, ImageInfo, Paint, Rect};

/// Read real GPU pixels to verify the drawable bridge, BGRA interpretation,
/// top-left origin and logical-to-physical scaling, plus the component dependencies.
pub fn skia_rendering(frame: &Frame<'_>) {
    let canvas = frame.canvas();
    canvas.clear(Color::BLACK);
    let mut red = Paint::default();
    red.set_color(Color::RED);
    canvas.draw_rect(Rect::from_xywh(10., 10., 20., 20.), &red);

    let pixel = |x: f64, y: f64| {
        let mut rgba = [0u8; 4];
        assert!(canvas.read_pixels(
            &ImageInfo::new((1, 1), ColorType::RGBA8888, AlphaType::Unpremul, None),
            &mut rgba,
            4,
            ((x * frame.info.scale) as i32, (y * frame.info.scale) as i32),
        ));
        rgba
    };
    assert_eq!(pixel(15., 15.), [255, 0, 0, 255]);
    assert_eq!(pixel(5., 15.), [0, 0, 0, 255]);
    assert_eq!(pixel(35., 15.), [0, 0, 0, 255]);

    let svg = skia::svg::Dom::from_str(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="100"><rect x="40" y="10" width="20" height="20" fill="blue"/></svg>"#,
        FontMgr::new(),
    )
    .unwrap();
    svg.render(canvas);
    assert_eq!(pixel(45., 15.), [0, 0, 255, 255]);

    use skia::textlayout::{FontCollection, ParagraphBuilder, ParagraphStyle, TextStyle};
    let mut fonts = FontCollection::new();
    fonts.set_default_font_manager(FontMgr::new(), Some("Arial"));
    let mut style = TextStyle::new();
    style.set_color(Color::WHITE).set_font_size(24.);
    let mut paragraph_style = ParagraphStyle::new();
    paragraph_style.set_text_style(&style);
    let mut builder = ParagraphBuilder::new(&paragraph_style, fonts);
    builder.add_text("Skia — café العربية");
    let mut paragraph = builder.build();
    paragraph.layout(500.);
    assert!(paragraph.height() > 0. && paragraph.longest_line() > 0.);
    assert_eq!(paragraph.unresolved_glyphs(), Some(0));
    paragraph.paint(canvas, (10., 70.));
    let bounds = ImageInfo::new((500, 100), ColorType::RGBA8888, AlphaType::Unpremul, None);
    let mut pixels = vec![0u8; bounds.compute_byte_size(500 * 4)];
    assert!(canvas.read_pixels(
        &bounds,
        &mut pixels,
        500 * 4,
        (0, (70. * frame.info.scale) as i32),
    ));
    assert!(pixels.chunks_exact(4).any(|rgba| rgba[0] > 200));

    // Verify image codecs can encode/decode and upload onto the Metal canvas.
    let mut raster = skia::surfaces::raster_n32_premul((4, 4)).unwrap();
    raster.canvas().clear(Color::GREEN);
    let image = raster.image_snapshot();
    for format in [
        skia::EncodedImageFormat::PNG,
        skia::EncodedImageFormat::JPEG,
        skia::EncodedImageFormat::WEBP,
    ] {
        let data = image.encode(None, format, 100).unwrap();
        let decoded = skia::Image::from_encoded(data).unwrap();
        canvas.draw_image_rect(
            decoded,
            None,
            Rect::from_xywh(70., 10., 20., 20.),
            &Paint::default(),
        );
        let rgba = pixel(75., 15.);
        assert!(rgba[0] < 10 && rgba[1] > 245 && rgba[2] < 10);
    }
}
