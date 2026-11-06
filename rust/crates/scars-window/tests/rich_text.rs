use scars_window::{skia, ui::*};
fn renderer() -> Renderer {
    Renderer::with_font(include_bytes!("../../../../demo/assets/Roboto-Regular.ttf")).unwrap()
}
fn by_key(host: &Host, key: &str) -> Node {
    host.snapshot()
        .unwrap()
        .into_iter()
        .find(|n| n.key.as_deref() == Some(key))
        .unwrap()
}
#[test]
fn styled_runs_paint_their_own_colors_and_participate_in_shared_shaping() {
    let renderer = renderer();
    let host = Host::new(|| {
        view! {column{rich_text(vec![
            Span::new("RED ").font_size(30.).foreground(0xffff0000).into(),
            Span::new("GREEN").font_size(24.).foreground(0xff00aa00).bold(true).into(),
        ],key:"rich");}}
    });
    host.layout(Size::new(400., 100.), &renderer).unwrap();
    assert!(by_key(&host, "rich").spans.is_some());
    let mut surface = skia::surfaces::raster_n32_premul((400, 100)).unwrap();
    surface.canvas().clear(skia::Color::WHITE);
    renderer.paint(&host, surface.canvas(), 0.).unwrap();
    let mut pixels = vec![0; 400 * 100 * 4];
    assert!(surface.read_pixels(
        &skia::ImageInfo::new(
            (400, 100),
            skia::ColorType::RGBA8888,
            skia::AlphaType::Unpremul,
            None
        ),
        &mut pixels,
        1600,
        (0, 0)
    ));
    assert!(
        pixels
            .as_chunks::<4>()
            .0
            .iter()
            .any(|p| p[0] > 180 && p[1] < 60 && p[2] < 60)
    );
    assert!(
        pixels
            .as_chunks::<4>()
            .0
            .iter()
            .any(|p| p[1] > 120 && p[0] < 60 && p[2] < 60)
    );
}
#[test]
fn inline_controls_wrap_align_hit_test_and_keep_identity_after_rebuilds() {
    let renderer = renderer();
    let count = State::new(0);
    let host = Host::new({
        let count = count.clone();
        move || {
            view! {column(gap:10){
                rich_text(vec!["Read the ".into(),Span::new("important ").font_size(22.).bold(true).into(),
                    View::button("Open",count.action(|n|n.update(|n|*n+=1))).width(80).height(34).key("open").into(),
                    " control before continuing with this longer explanation.".into(),
                ],key:"paragraph");
                label(format!("Count {}",count.get()),key:"footer");
            }}
        }
    });
    host.layout(Size::new(700., 300.), &renderer).unwrap();
    let wide = by_key(&host, "paragraph");
    let button = by_key(&host, "open");
    let id = button.id;
    assert!((wide.bounds.y + wide.baseline - button.bounds.y - button.baseline).abs() < 1.);
    assert_eq!(
        host.hit_test(Point {
            x: button.bounds.x + 5.,
            y: button.bounds.y + 5.
        })
        .unwrap(),
        Some(id)
    );
    assert!(host.activate(id).unwrap());
    assert_eq!(count.get(), 1);
    host.layout(Size::new(210., 300.), &renderer).unwrap();
    assert_eq!(by_key(&host, "open").id, id);
    let narrow = by_key(&host, "paragraph");
    assert!(narrow.bounds.height > wide.bounds.height);
    assert!(by_key(&host, "footer").bounds.y >= narrow.bounds.y + narrow.bounds.height + 9.);
    assert!(host.focus(id).unwrap());
}
#[test]
fn elided_inline_controls_are_inert_and_reappear_when_space_returns() {
    let renderer = renderer();
    let clicked = State::new(0);
    let host = Host::new({
        let clicked = clicked.clone();
        move || {
            view! {column{
                rich_text(vec!["A deliberately long explanation before the inline control ".into(),View::button("Tail",clicked.action(|n|n.update(|n|*n+=1))).key("tail").into()],max_lines:1,text_overflow:TextOverflow::Ellipsis,key:"paragraph");
                button("Visible",on_click:||{});
            }}
        }
    });
    host.layout(Size::new(100., 100.), &renderer).unwrap();
    let id = by_key(&host, "tail").id;
    assert!(by_key(&host, "tail").hidden);
    assert!(!host.activate(id).unwrap());
    host.layout(Size::new(1200., 100.), &renderer).unwrap();
    assert!(!by_key(&host, "tail").hidden);
    assert!(host.activate(id).unwrap());
    assert_eq!(clicked.get(), 1);
}
#[test]
fn invisible_runs_do_not_shift_other_inline_view_slots() {
    let renderer = renderer();
    let host = Host::new(
        || view! {rich_text(vec![View::button("Removed",||{}).visible(false).into(),"Text ".into(),View::button("Present",||{}).key("present").into()]);},
    );
    host.layout(Size::new(400., 100.), &renderer).unwrap();
    assert!(by_key(&host, "present").bounds.width > 0.);
    assert!(host.snapshot().unwrap().iter().all(|n| n.text != "Removed"));
}
