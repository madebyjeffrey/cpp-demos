use scars_window::{skia, ui::*};
fn renderer() -> Renderer {
    Renderer::with_font(include_bytes!("../../../../demo/assets/Roboto-Regular.ttf")).unwrap()
}
fn keyed(host: &Host, key: &str) -> Node {
    host.snapshot()
        .unwrap()
        .into_iter()
        .find(|n| n.key.as_deref() == Some(key))
        .unwrap()
}
fn pointer(kind: PointerKind, p: Point) -> Pointer {
    Pointer {
        kind,
        position: p,
        button: 0,
        modifiers: Default::default(),
    }
}
#[test]
fn caret_reveal_tracks_selection_without_overriding_user_scroll() {
    let text = State::new((0..40).map(|i| format!("Line {i}\n")).collect::<String>());
    let host = Host::new({
        let text = text.clone();
        move || {
            view! {scroll(key:"scroll",height:100,scroll_axes:ScrollAxes::Both,gap:0){
                text_editor("",bind:text,key:"editor",width:500,height:700);
            }}
        }
    });
    let renderer = renderer();
    host.layout(Size::new(200., 150.), &renderer).unwrap();
    let scroll = keyed(&host, "scroll").id;
    let editor = keyed(&host, "editor").id;
    host.focus(editor).unwrap();
    host.select_text(
        editor,
        Selection {
            anchor: text.get().len(),
            caret: text.get().len(),
        },
    )
    .unwrap();
    host.layout(Size::new(200., 150.), &renderer).unwrap();
    let info = host.scroll_info(scroll).unwrap().unwrap();
    assert!(info.offset.y > 500.);
    let caret = host.text_rectangles(editor, text.get().len()..text.get().len())[0];
    assert!(
        caret.y >= info.viewport.y
            && caret.y + caret.height <= info.viewport.y + info.viewport.height + 1.
    );
    host.scroll_to(scroll, Point::default()).unwrap();
    host.layout(Size::new(200., 150.), &renderer).unwrap();
    assert_eq!(
        host.scroll_info(scroll).unwrap().unwrap().offset,
        Point::default()
    );
}
#[test]
fn stationary_drag_selection_autoscrolls_then_settles_after_release() {
    let text = State::new(
        (0..80)
            .map(|i| format!("Line {i} words here\n"))
            .collect::<String>(),
    );
    let host = Host::new(move || view! {text_editor("",bind:text,key:"editor",height:80);});
    let renderer = renderer();
    host.layout(Size::new(240., 80.), &renderer).unwrap();
    let id = keyed(&host, "editor").id;
    host.pointer(pointer(PointerKind::Down, Point { x: 20., y: 20. }))
        .unwrap();
    host.pointer(pointer(PointerKind::Move, Point { x: 100., y: 110. }))
        .unwrap();
    host.layout(Size::new(240., 80.), &renderer).unwrap();
    let first = host.text_input().unwrap().unwrap().selection.caret;
    for _ in 0..4 {
        assert!(host.next_deadline().is_some());
        host.run_due_at(std::time::Instant::now() + std::time::Duration::from_secs(1));
        host.layout(Size::new(240., 80.), &renderer).unwrap();
    }
    assert!(host.text_input().unwrap().unwrap().selection.caret > first);
    host.pointer(pointer(PointerKind::Up, Point { x: 100., y: 110. }))
        .unwrap();
    assert!(host.next_deadline().is_none());
    let after = host.text_input().unwrap().unwrap().selection;
    host.run_due_at(std::time::Instant::now() + std::time::Duration::from_secs(2));
    assert_eq!(host.text_input().unwrap().unwrap().selection, after);
    assert!(host.focus(id).unwrap());
}
#[test]
fn scrollbar_paints_above_content_but_below_later_siblings() {
    let renderer = renderer();
    let host = Host::new(|| {
        view! {layer{
            scroll(key:"scroll",height:100){label("Content",height:400,background:0xffff0000);}
            label("",height:35,background:0xff00ff00,z_index:1);
        }}
    });
    host.layout(Size::new(200., 150.), &renderer).unwrap();
    let bar = host
        .scroll_info(keyed(&host, "scroll").id)
        .unwrap()
        .unwrap()
        .bars()
        .next()
        .unwrap();
    let mut surface = skia::surfaces::raster_n32_premul((200, 150)).unwrap();
    renderer.paint(&host, surface.canvas(), 0.).unwrap();
    let mut pixels = vec![0; 200 * 150 * 4];
    assert!(surface.read_pixels(
        &skia::ImageInfo::new(
            (200, 150),
            skia::ColorType::RGBA8888,
            skia::AlphaType::Unpremul,
            None
        ),
        &mut pixels,
        800,
        (0, 0)
    ));
    let x = (bar.thumb.x + 3.) as usize;
    assert_eq!(
        &pixels[(10 * 200 + x) * 4..(10 * 200 + x) * 4 + 3],
        &[0, 255, 0]
    );
    // Below the sibling, the bar track overlays the red content.
    assert_ne!(
        &pixels[(50 * 200 + x) * 4..(50 * 200 + x) * 4 + 3],
        &[255, 0, 0]
    );
}
#[test]
fn nested_portal_paints_above_its_owners_scrollbar() {
    let renderer = renderer();
    let popup = Popup::new().options(OverlayOptions {
        width: Some(200.),
        ..Default::default()
    });
    popup.show();
    let host = Host::new(move || {
        View::scroll(vec![
            View::layer(vec![
                View::button("Anchor", || {}).key("anchor"),
                popup.popover("anchor", View::label("").height(80).background(0xff00ff00)),
            ])
            .height(50)
            .shrink(0.),
            View::spacer().height(400).shrink(0.),
        ])
        .height(100)
        .key("scroll")
    });
    host.layout(Size::new(200., 150.), &renderer).unwrap();
    let bar = host
        .scroll_info(keyed(&host, "scroll").id)
        .unwrap()
        .unwrap()
        .bars()
        .next()
        .unwrap();
    let mut surface = skia::surfaces::raster_n32_premul((200, 150)).unwrap();
    renderer.paint(&host, surface.canvas(), 0.).unwrap();
    let mut pixels = vec![0; 200 * 150 * 4];
    assert!(surface.read_pixels(
        &skia::ImageInfo::new(
            (200, 150),
            skia::ColorType::RGBA8888,
            skia::AlphaType::Unpremul,
            None
        ),
        &mut pixels,
        800,
        (0, 0)
    ));
    let x = (bar.track.x + 3.) as usize;
    let p = (70 * 200 + x) * 4;
    assert_eq!(&pixels[p..p + 3], &[0, 255, 0]);
}
