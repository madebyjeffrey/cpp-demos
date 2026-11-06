use scars_ui::text::{byte_offset, utf16_offset};
use scars_window::skia as skia_safe;
use skia_safe::{
    FontMgr,
    textlayout::{
        FontCollection, ParagraphBuilder, ParagraphStyle, TextStyle, TypefaceFontProvider,
    },
};

#[test]
fn paragraph_geometry_and_native_indices_work_without_a_window_or_gpu() {
    let bytes = include_bytes!("../../../../demo/assets/Roboto-Regular.ttf");
    let face = FontMgr::new()
        .new_from_data(skia_safe::Data::new_copy(bytes), None)
        .expect("bundled font");
    let mut provider = TypefaceFontProvider::new();
    provider.register_typeface(face, Some("Fixture"));
    let mut fonts = FontCollection::new();
    fonts.set_asset_font_manager(Some(provider.into()));
    let mut text_style = TextStyle::new();
    text_style
        .set_font_families(&["Fixture"])
        .set_font_size(20.)
        .set_color(skia_safe::Color::BLACK);
    let mut style = ParagraphStyle::new();
    style.set_text_style(&text_style);
    let text = "café é\nsecond line";
    let mut builder = ParagraphBuilder::new(&style, fonts);
    builder.add_text(text);
    let mut paragraph = builder.build();
    paragraph.layout(400.);
    assert_eq!(paragraph.line_number(), 2);
    assert_eq!(paragraph.unresolved_glyphs(), Some(0));
    let mut surface = skia_safe::surfaces::raster_n32_premul((400, 100)).unwrap();
    surface.canvas().clear(skia_safe::Color::WHITE);
    paragraph.paint(surface.canvas(), (0., 0.));
    let start = utf16_offset(text, "caf".len()).unwrap();
    let end = utf16_offset(text, "café".len()).unwrap();
    let boxes = paragraph.get_rects_for_range(
        start..end,
        skia_safe::textlayout::RectHeightStyle::Tight,
        skia_safe::textlayout::RectWidthStyle::Tight,
    );
    assert!(!boxes.is_empty() && boxes[0].rect.width() > 0.);
    let info = paragraph.get_glyph_info_at_utf16_offset(start).unwrap();
    assert!(info.grapheme_layout_bounds.width() > 0.);
    assert_eq!(byte_offset(text, start), Some(3));
    let mut pixels = vec![0; 400 * 100 * 4];
    assert!(surface.read_pixels(
        &skia_safe::ImageInfo::new_n32_premul((400, 100), None),
        &mut pixels,
        400 * 4,
        (0, 0)
    ));
    assert!(
        pixels
            .as_chunks::<4>()
            .0
            .iter()
            .any(|p| p[0] < 200 && p[3] == 255)
    );
}
