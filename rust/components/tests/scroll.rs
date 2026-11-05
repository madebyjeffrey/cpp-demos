use scars_ui::*;
use std::rc::Rc;
struct Metrics;
impl Measure for Metrics {
    fn text(&self, text: &str, style: TextStyle, _: f64) -> TextLayout {
        TextLayout {
            size: Size::new(text.len() as f64 * 8., 20.),
            baseline: style.size,
            placeholders: vec![],
            hit_test: Rc::new(|_| 0),
            range_rects: Rc::new(|_| vec![]),
        }
    }
    fn image(&self, _: &Bitmap) -> Result<Size, Error> {
        Ok(Size::new(1., 1.))
    }
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
        modifiers: Modifiers::default(),
    }
}
#[test]
fn axes_clamp_offsets_queries_and_content_replacement() {
    for axes in [
        ScrollAxes::Vertical,
        ScrollAxes::Horizontal,
        ScrollAxes::Both,
    ] {
        let size = State::new(Size::new(600., 400.));
        let host = Host::new({
            let size = size.clone();
            move || {
                view! {scroll(key:"scroll",height:100,scroll_axes:axes,gap:0){
                    button("Content",width:size.get().width,height:size.get().height,on_click:||{});
                }}
            }
        });
        host.layout(Size::new(200., 150.), &Metrics).unwrap();
        let id = keyed(&host, "scroll").id;
        let info = host.scroll_info(id).unwrap().unwrap();
        let max = Point {
            x: if axes != ScrollAxes::Vertical {
                400.
            } else {
                0.
            },
            y: if axes != ScrollAxes::Horizontal {
                300.
            } else {
                0.
            },
        };
        assert_eq!(info.max, max);
        assert!(
            host.scroll_to(
                id,
                Point {
                    x: 10000.,
                    y: 10000.
                }
            )
            .unwrap()
        );
        assert_eq!(host.scroll_info(id).unwrap().unwrap().offset, max);
        host.layout(Size::new(200., 150.), &Metrics).unwrap();
        let info = host.scroll_info(id).unwrap().unwrap();
        assert_eq!(
            info.bars().count(),
            if axes == ScrollAxes::Both { 2 } else { 1 }
        );
        for bar in info.bars() {
            assert!(bar.thumb.width > 0. && bar.thumb.height > 0.);
        }
        assert!(!host.scroll_by(id, Point { x: f64::NAN, y: 0. }).unwrap());
        size.set(Size::new(80., 40.));
        host.layout(Size::new(200., 150.), &Metrics).unwrap();
        let info = host.scroll_info(id).unwrap().unwrap();
        assert_eq!(info.offset, Point::default());
        assert_eq!(info.bars().count(), 0);
    }
}
#[test]
fn diagonal_wheel_residuals_propagate_to_ancestors_only() {
    let host = Host::new(|| {
        view! {scroll(key:"outer",height:120,scroll_axes:ScrollAxes::Both,gap:0){
            scroll(key:"inner",width:400,height:80,scroll_axes:ScrollAxes::Vertical,gap:0){label("Long",height:100);}
            label("Tail",height:300);
        }}
    });
    host.layout(Size::new(200., 150.), &Metrics).unwrap();
    let point = Point { x: 10., y: 10. };
    assert!(host.scroll_at(point, Point { x: 50., y: 70. }).unwrap());
    let inner = host.scroll_info(keyed(&host, "inner").id).unwrap().unwrap();
    let outer = host.scroll_info(keyed(&host, "outer").id).unwrap().unwrap();
    assert_eq!(inner.offset, Point { x: 0., y: 20. });
    assert_eq!(outer.offset, Point { x: 50., y: 50. });
    // A higher sibling overlay blocks scrolling of content behind it.
    let overlay = Host::new(
        || view! {layer{scroll(height:100){label("Content",height:400);}button("Front",height:100,on_click:||{});}},
    );
    overlay.layout(Size::new(200., 150.), &Metrics).unwrap();
    assert!(!overlay.scroll(point, 30.).unwrap());
}
#[test]
fn scrollbar_drag_and_track_click_do_not_activate_content() {
    let count = State::new(0);
    let host = Host::new({
        let count = count.clone();
        move || {
            view! {scroll(key:"scroll",height:100,scroll_axes:ScrollAxes::Both,gap:0){
                button("Content",width:600,height:400,on_click:count.action(|n|n.update(|n|*n+=1)));
            }}
        }
    });
    host.layout(Size::new(200., 150.), &Metrics).unwrap();
    let id = keyed(&host, "scroll").id;
    for axis in [Orientation::Horizontal, Orientation::Vertical] {
        let bar = host
            .scroll_info(id)
            .unwrap()
            .unwrap()
            .bars()
            .find(|b| b.orientation == axis)
            .unwrap();
        let start = Point {
            x: bar.thumb.x + 2.,
            y: bar.thumb.y + 2.,
        };
        assert!(host.pointer(pointer(PointerKind::Down, start)).unwrap());
        let end = if axis == Orientation::Horizontal {
            Point { x: 1000., ..start }
        } else {
            Point { y: 1000., ..start }
        };
        assert!(host.pointer(pointer(PointerKind::Move, end)).unwrap());
        assert!(host.pointer(pointer(PointerKind::Up, end)).unwrap());
        let info = host.scroll_info(id).unwrap().unwrap();
        assert_eq!(
            if axis == Orientation::Horizontal {
                info.offset.x
            } else {
                info.offset.y
            },
            if axis == Orientation::Horizontal {
                info.max.x
            } else {
                info.max.y
            }
        );
        host.layout(Size::new(200., 150.), &Metrics).unwrap();
    }
    assert_eq!(count.get(), 0);
    let bar = host
        .scroll_info(id)
        .unwrap()
        .unwrap()
        .bars()
        .next()
        .unwrap();
    host.pointer(pointer(
        PointerKind::Down,
        Point {
            x: bar.track.x + 1.,
            y: bar.track.y + 1.,
        },
    ))
    .unwrap();
    assert!(host.scroll_info(id).unwrap().unwrap().offset.x < 400.);
}
#[test]
fn keyboard_scroll_and_nested_focus_reveal_use_both_axes() {
    let host = Host::new(|| {
        view! {scroll(key:"outer",height:120,scroll_axes:ScrollAxes::Both,gap:0){
            column(width:600,gap:0){label("Spacer",height:200);button("Focus",key:"target",width:80,height:30,align_self:Align::End,on_click:||{});label("Tail",height:200);}
        }}
    });
    host.layout(Size::new(200., 150.), &Metrics).unwrap();
    let id = keyed(&host, "outer").id;
    host.focus(keyed(&host, "target").id).unwrap();
    host.layout(Size::new(200., 150.), &Metrics).unwrap();
    let target = keyed(&host, "target");
    let info = host.scroll_info(id).unwrap().unwrap();
    assert_eq!(info.offset, Point { x: 400., y: 110. });
    assert_eq!(target.clip, target.bounds);
    assert!(
        host.key(KeyEvent {
            key: Key::Home,
            down: true,
            repeat: false,
            modifiers: Modifiers {
                shift: true,
                ..Default::default()
            }
        })
        .unwrap()
    );
    assert_eq!(host.scroll_info(id).unwrap().unwrap().offset.x, 0.);
    assert!(
        host.key(KeyEvent {
            key: Key::PageDown,
            down: true,
            repeat: false,
            modifiers: Modifiers::default()
        })
        .unwrap()
    );
    assert_eq!(host.scroll_info(id).unwrap().unwrap().offset.y, 218.);
}
#[test]
fn capture_cancels_on_blur_deactivation_hiding_removal_and_cancel() {
    for mode in 0..5 {
        let visible = State::new(true);
        let hidden = State::new(false);
        let host = Host::new({
            let visible = visible.clone();
            let hidden = hidden.clone();
            move || {
                view! {column{
                    if visible.get(){scroll(key:"scroll",hidden:hidden.get(),height:100){label("Content",height:400);}}
                }}
            }
        });
        host.layout(Size::new(200., 150.), &Metrics).unwrap();
        let id = keyed(&host, "scroll").id;
        let bar = host
            .scroll_info(id)
            .unwrap()
            .unwrap()
            .bars()
            .next()
            .unwrap();
        host.pointer(pointer(
            PointerKind::Down,
            Point {
                x: bar.thumb.x + 2.,
                y: bar.thumb.y + 2.,
            },
        ))
        .unwrap();
        match mode {
            0 => host.blur(),
            1 => host.set_active(false),
            2 => hidden.set(true),
            3 => visible.set(false),
            _ => {
                host.pointer(pointer(PointerKind::Cancel, Point::default()))
                    .unwrap();
            }
        }
        host.layout(Size::new(200., 150.), &Metrics).unwrap();
        host.pointer(pointer(
            PointerKind::Move,
            Point {
                x: bar.thumb.x + 2.,
                y: 1000.,
            },
        ))
        .unwrap();
        assert_eq!(
            host.scroll_info(id)
                .unwrap()
                .map(|i| i.offset.y)
                .unwrap_or(0.),
            0.
        );
        assert!(host.next_deadline().is_none());
    }
}
#[test]
fn nested_reveal_projects_inner_motion_before_moving_outer_viewport() {
    let host = Host::new(|| {
        view! {scroll(key:"outer",height:120,scroll_axes:ScrollAxes::Both,gap:0){
            column(width:600,gap:0,align:Align::Start){
                label("Top",height:200);
                scroll(key:"inner",width:120,height:80,margin:[0.,0.,0.,400.],gap:0){label("Inside",height:150);button("Target",key:"target",height:30,on_click:||{});}
                label("Bottom",height:200);
            }
        }}
    });
    host.layout(Size::new(200., 150.), &Metrics).unwrap();
    host.focus(keyed(&host, "target").id).unwrap();
    host.layout(Size::new(200., 150.), &Metrics).unwrap();
    let target = keyed(&host, "target");
    assert_eq!(target.clip, target.bounds);
    assert_eq!(
        host.scroll_info(keyed(&host, "inner").id)
            .unwrap()
            .unwrap()
            .offset
            .y,
        100.
    );
    assert_eq!(
        host.scroll_info(keyed(&host, "outer").id)
            .unwrap()
            .unwrap()
            .offset,
        Point { x: 320., y: 160. }
    );
}
