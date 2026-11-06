use scars_ui::*;
use std::rc::Rc;
struct Metrics;
impl Measure for Metrics {
    fn text(&self, text: &str, style: TextStyle, _: f64) -> TextLayout {
        TextLayout {
            size: Size::new(text.len() as f64 * 8., 20.),
            baseline: style.size,
            lines: vec![],
            placeholders: vec![],
            hit_test: Rc::new(|_| 0),
            range_rects: Rc::new(|_| vec![]),
        }
    }
    fn image(&self, _: &Bitmap) -> Result<Size, Error> {
        Ok(Size::new(1., 1.))
    }
}
fn node(host: &Host, kind: Kind) -> Node {
    host.snapshot()
        .unwrap()
        .into_iter()
        .find(|n| n.kind == kind)
        .unwrap()
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
fn key(host: &Host, key: Key, shift: bool) -> bool {
    host.key(KeyEvent {
        key,
        down: true,
        repeat: false,
        modifiers: Modifiers {
            shift,
            ..Default::default()
        },
    })
    .unwrap()
}
#[test]
fn both_axes_fit_minima_padding_and_infeasible_sizes_without_changing_requested_ratio() {
    for orientation in [Orientation::Horizontal, Orientation::Vertical] {
        let split = Split::new(0.25)
            .orientation(orientation)
            .limits(80., 120.)
            .divider_width(10.);
        let host = Host::new({
            let split = split.clone();
            move || {
                split
                    .view(View::label("First"), View::label("Second"))
                    .padding(10)
            }
        });
        host.layout(Size::new(500., 300.), &Metrics).unwrap();
        let info = node(&host, Kind::Split).split.unwrap();
        if orientation == Orientation::Horizontal {
            assert_eq!(info.first, Rect::new(10., 10., 117.5, 280.));
            assert_eq!(info.second.width, 352.5);
        } else {
            assert_eq!(info.first, Rect::new(10., 10., 480., 80.));
            assert_eq!(info.second.height, 190.);
        }
        host.layout(Size::new(160., 120.), &Metrics).unwrap();
        let info = node(&host, Kind::Split).split.unwrap();
        assert_eq!(info.limits.0, info.limits.1);
        assert_eq!(split.ratio(), 0.25);
        assert!(
            info.first.width >= 0.
                && info.first.height >= 0.
                && info.second.width >= 0.
                && info.second.height >= 0.
        );
    }
    let huge = Split::new(0.5).limits(f64::MAX, f64::MAX);
    let host = Host::new(move || huge.view(View::label("A"), View::label("B")));
    host.layout(Size::new(100., 80.), &Metrics).unwrap();
    assert_eq!(node(&host, Kind::Split).split.unwrap().first.width, 44.);
    let invalid = Split::new(0.5).limits(-1., 2.);
    assert!(
        Host::new(move || invalid.view(View::label("A"), View::label("B")))
            .refresh()
            .is_err()
    );
}
#[test]
fn divider_drag_keeps_grab_offset_clamps_and_holds_the_resize_cursor_outside() {
    for orientation in [Orientation::Horizontal, Orientation::Vertical] {
        let split = Split::new(0.5).orientation(orientation).limits(50., 50.);
        let host = Host::new({
            let split = split.clone();
            move || split.view(View::label("A"), View::label("B"))
        });
        let size = Size::new(400., 200.);
        host.layout(size, &Metrics).unwrap();
        let divider = node(&host, Kind::SplitDivider);
        let info = divider.split.unwrap();
        let p = Point {
            x: divider.bounds.x + 3.,
            y: divider.bounds.y + 3.,
        };
        let cursor = if orientation == Orientation::Horizontal {
            PointerCursor::ResizeHorizontal
        } else {
            PointerCursor::ResizeVertical
        };
        assert_eq!(host.pointer_cursor(p).unwrap(), cursor);
        assert!(host.pointer(pointer(PointerKind::Down, p)).unwrap());
        host.pointer(pointer(PointerKind::Move, p)).unwrap();
        assert_eq!(split.ratio(), 0.5);
        let moved = if orientation == Orientation::Horizontal {
            Point { x: p.x + 20., ..p }
        } else {
            Point { y: p.y + 20., ..p }
        };
        host.pointer(pointer(PointerKind::Move, moved)).unwrap();
        host.layout(size, &Metrics).unwrap();
        assert!((split.ratio() - (info.available / 2. + 20.) / info.available).abs() < 0.0001);
        assert_eq!(
            host.pointer_cursor(Point { x: -100., y: -100. }).unwrap(),
            cursor
        );
        host.pointer(pointer(PointerKind::Move, Point { x: -100., y: -100. }))
            .unwrap();
        host.layout(size, &Metrics).unwrap();
        assert!((split.ratio() - 50. / info.available).abs() < 0.0001);
        host.pointer(pointer(PointerKind::Up, Point { x: -100., y: -100. }))
            .unwrap();
        assert_eq!(
            host.pointer_cursor(Point { x: -100., y: -100. }).unwrap(),
            PointerCursor::Arrow
        );
        assert_eq!(host.focused(), Some(divider.id));
    }
}
#[test]
fn collapsed_panes_are_inert_but_retain_editor_history_selection_and_scroll() {
    let split = Split::new(0.4).limits(20., 20.);
    let value = State::new("start".to_owned());
    let host = Host::new({
        let (split, value) = (split.clone(), value.clone());
        move || {
            split.view(view!{column{ text_field("Name",bind:value,key:"field");scroll(height:80,key:"scroll"){label("Long",height:400);}}},View::button("Other",||{}))
        }
    });
    let size = Size::new(500., 240.);
    host.layout(size, &Metrics).unwrap();
    let field = keyed(&host, "field").id;
    let scroll = keyed(&host, "scroll").id;
    let divider = node(&host, Kind::SplitDivider).id;
    host.focus(field).unwrap();
    host.set_text(field, "edit").unwrap();
    host.select_text(
        field,
        Selection {
            anchor: 1,
            caret: 3,
        },
    )
    .unwrap();
    host.layout(size, &Metrics).unwrap();
    host.scroll_to(scroll, Point { x: 0., y: 30. }).unwrap();
    host.layout(size, &Metrics).unwrap();
    host.mark_text("日", 3..3, None).unwrap();
    split.collapse(SplitPane::First);
    host.layout(size, &Metrics).unwrap();
    assert!(keyed(&host, "field").hidden);
    assert!(!host.activate(field).unwrap());
    assert_eq!(host.focused(), Some(divider));
    assert!(host.text_input().unwrap().is_none());
    split.restore();
    host.layout(size, &Metrics).unwrap();
    assert_eq!(keyed(&host, "field").id, field);
    assert!(!keyed(&host, "field").hidden);
    assert_eq!(host.scroll_info(scroll).unwrap().unwrap().offset.y, 30.);
    host.focus(field).unwrap();
    let input = host.text_input().unwrap().unwrap();
    assert_eq!(input.text, "edit");
    assert_eq!(
        input.selection,
        Selection {
            anchor: 1,
            caret: 3
        }
    );
    assert!(input.marked.is_none() && input.can_undo);
    host.text_command(TextCommand::Undo, false).unwrap();
    assert_eq!(value.get(), "start");
}
#[test]
fn keyboard_limits_collapse_restore_and_escape_drag_rollback_are_explicit() {
    let split = Split::new(0.5).limits(50., 50.).keyboard_step(10.);
    let host = Host::new({
        let split = split.clone();
        move || split.view(View::label("A"), View::label("B"))
    });
    let size = Size::new(400., 200.);
    host.layout(size, &Metrics).unwrap();
    let divider = node(&host, Kind::SplitDivider);
    host.focus(divider.id).unwrap();
    assert!(key(&host, Key::Right, false));
    host.layout(size, &Metrics).unwrap();
    assert_eq!(
        host.split_info(divider.id).unwrap().unwrap().first.width,
        204.
    );
    assert!(key(&host, Key::Left, true));
    host.layout(size, &Metrics).unwrap();
    assert_eq!(
        host.split_info(divider.id).unwrap().unwrap().first.width,
        104.
    );
    key(&host, Key::Home, false);
    host.layout(size, &Metrics).unwrap();
    let saved = split.ratio();
    assert_eq!(
        host.split_info(divider.id).unwrap().unwrap().first.width,
        50.
    );
    key(&host, Key::Enter, false);
    host.layout(size, &Metrics).unwrap();
    assert_eq!(split.collapsed_pane(), SplitPane::First);
    assert_eq!(
        host.split_info(divider.id).unwrap().unwrap().first.width,
        0.
    );
    key(&host, Key::Escape, false);
    host.layout(size, &Metrics).unwrap();
    assert_eq!(split.collapsed_pane(), SplitPane::None);
    assert_eq!(split.ratio(), saved);
    let divider = node(&host, Kind::SplitDivider);
    let p = Point {
        x: divider.bounds.x + 4.,
        y: 20.,
    };
    host.pointer(pointer(PointerKind::Down, p)).unwrap();
    host.pointer(pointer(PointerKind::Move, Point { x: p.x + 40., ..p }))
        .unwrap();
    host.layout(size, &Metrics).unwrap();
    assert_ne!(split.ratio(), saved);
    assert!(key(&host, Key::Escape, false));
    host.layout(size, &Metrics).unwrap();
    assert_eq!(split.ratio(), saved);
    split.collapse(SplitPane::First);
    host.layout(size, &Metrics).unwrap();
    let divider = node(&host, Kind::SplitDivider);
    let p = Point {
        x: divider.bounds.x + 3.,
        y: 20.,
    };
    host.pointer(pointer(PointerKind::Down, p)).unwrap();
    host.layout(size, &Metrics).unwrap();
    host.pointer(pointer(PointerKind::Move, Point { x: 183., ..p }))
        .unwrap();
    host.layout(size, &Metrics).unwrap();
    assert_eq!(split.collapsed_pane(), SplitPane::None);
    assert_eq!(
        host.split_info(divider.id).unwrap().unwrap().first.width,
        180.
    );
    key(&host, Key::Escape, false);
    host.layout(size, &Metrics).unwrap();
    assert_eq!(split.collapsed_pane(), SplitPane::First);
    assert_eq!(split.ratio(), saved);
}
#[test]
fn captures_cancel_on_blur_deactivate_hide_remove_disable_and_pointer_cancel() {
    for mode in 0..10 {
        let ratio = State::new(0.5);
        let visible = State::new(true);
        let hidden = State::new(false);
        let enabled = State::new(true);
        let orientation = State::new(Orientation::Horizontal);
        let model = Split::bound(ratio.binding());
        let host = Host::new({
            let orientation = orientation.clone();
            let model = model.clone();
            let (visible, hidden, enabled) = (visible.clone(), hidden.clone(), enabled.clone());
            move || {
                model
                    .clone()
                    .orientation(orientation.get())
                    .view(View::button("A", || {}).key("focus"), View::label("B"))
                    .visible(visible.get())
                    .hidden(hidden.get())
                    .enabled(enabled.get())
            }
        });
        let size = Size::new(400., 200.);
        host.layout(size, &Metrics).unwrap();
        let divider = node(&host, Kind::SplitDivider);
        let p = Point {
            x: divider.bounds.x + 4.,
            y: 20.,
        };
        host.pointer(pointer(PointerKind::Down, p)).unwrap();
        match mode {
            0 => host.blur(),
            1 => host.set_active(false),
            2 => hidden.set(true),
            3 => visible.set(false),
            4 => enabled.set(false),
            5 => {
                host.pointer(pointer(PointerKind::Cancel, p)).unwrap();
            }
            6 => orientation.set(Orientation::Vertical),
            7 => model.collapse(SplitPane::First),
            8 => {
                host.focus(keyed(&host, "focus").id).unwrap();
            }
            _ => ratio.set(0.6),
        }
        host.layout(size, &Metrics).unwrap();
        host.pointer(pointer(PointerKind::Move, Point { x: 300., y: 20. }))
            .unwrap();
        assert_eq!(ratio.get(), if mode == 9 { 0.6 } else { 0.5 });
        assert_ne!(
            host.pointer_cursor(Point { x: -1., y: -1. }).unwrap(),
            PointerCursor::ResizeHorizontal
        );
    }
}
#[test]
fn legacy_ratio_helper_and_external_collapse_binding_keep_mounted_lifetimes() {
    let ratio = State::new(0.3);
    let host = Host::new({
        let ratio = ratio.clone();
        move || split_view(&ratio, View::button("A", || {}), View::button("B", || {}))
    });
    let size = Size::new(500., 200.);
    host.layout(size, &Metrics).unwrap();
    let split = node(&host, Kind::Split).id;
    host.collapse_split(split, SplitPane::Second).unwrap();
    host.layout(size, &Metrics).unwrap();
    assert_eq!(
        host.split_info(split).unwrap().unwrap().collapsed,
        SplitPane::Second
    );
    assert_eq!(ratio.get(), 0.3);
    ratio.set(0.4);
    host.layout(size, &Metrics).unwrap();
    assert_eq!(
        host.split_info(split).unwrap().unwrap().collapsed,
        SplitPane::Second
    );
    host.collapse_split(split, SplitPane::None).unwrap();
    host.layout(size, &Metrics).unwrap();
    assert_eq!(host.split_info(split).unwrap().unwrap().ratio, 0.4);
    let split = Split::new(0.5);
    let ergonomic =
        Host::new(move || view! {split(&split,View::label("First"),View::label("Second"));});
    ergonomic.layout(size, &Metrics).unwrap();
    assert!(node(&ergonomic, Kind::Split).split.is_some());
    let collapsed = State::new(SplitPane::First);
    let model = Split::new(0.4).collapsed(collapsed.binding());
    let host = Host::new(move || model.view(View::label("A"), View::label("B")));
    host.layout(size, &Metrics).unwrap();
    assert_eq!(
        node(&host, Kind::Split).split.unwrap().collapsed,
        SplitPane::First
    );
    collapsed.set(SplitPane::Second);
    host.layout(size, &Metrics).unwrap();
    assert_eq!(
        node(&host, Kind::Split).split.unwrap().collapsed,
        SplitPane::Second
    );
}
