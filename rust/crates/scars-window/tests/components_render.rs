use scars_window::{skia, ui::*};
#[test]
fn themes_images_and_text_render_without_a_gpu() {
    let renderer =
        Renderer::with_font(include_bytes!("../../../../demo/assets/Roboto-Regular.ttf")).unwrap();
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

#[test]
fn paragraph_limits_alignment_and_line_metrics_use_the_shaping_cache_correctly() {
    let renderer =
        Renderer::with_font(include_bytes!("../../../../demo/assets/Roboto-Regular.ttf")).unwrap();
    let base = TextStyle {
        size: 20.,
        ..TextStyle::default()
    };
    let right = Measure::text(
        &renderer,
        "ABC",
        TextStyle {
            align: Align::End,
            ..base
        },
        140.,
    );
    let left = Measure::text(&renderer, "ABC", base, 140.);
    assert!((right.range_rects)(0..3)[0].x > 80.);
    assert!((left.range_rects)(0..3)[0].x < 1.);
    let words =
        "A deliberately long paragraph that takes several lines to fit into the assigned width.";
    let full = Measure::text(
        &renderer,
        words,
        TextStyle {
            wrap: true,
            line_height: 26.,
            ..base
        },
        100.,
    );
    let limited = Measure::text(
        &renderer,
        words,
        TextStyle {
            wrap: true,
            line_height: 26.,
            max_lines: 2,
            overflow: TextOverflow::Ellipsis,
            ..base
        },
        100.,
    );
    assert!(full.size.height > limited.size.height);
    assert!(limited.size.height <= 53.);
    assert!(limited.size.width <= 100.);
    let single = Measure::text(
        &renderer,
        words,
        TextStyle {
            overflow: TextOverflow::Ellipsis,
            ..base
        },
        100.,
    );
    assert!(single.size.width <= 100.);
    assert!(single.size.height < 30.);
    let spaced = Measure::text(
        &renderer,
        "ABC",
        TextStyle {
            letter_spacing: 4.,
            ..base
        },
        140.,
    );
    assert!(spaced.size.width > left.size.width + 8.);
}
