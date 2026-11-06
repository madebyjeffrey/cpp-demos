use scars_window::ui::*;
fn renderer() -> Renderer {
    Renderer::with_font(include_bytes!("../../../../demo/assets/Roboto-Regular.ttf")).unwrap()
}
fn mounted(text: &str, width: f64) -> (State<String>, Host, NodeId, Renderer) {
    let value = State::new(text.to_owned());
    let host = Host::new({
        let value = value.clone();
        move || view! {text_editor("",bind:value,height:100,key:"editor");}
    });
    let renderer = renderer();
    host.layout(Size::new(width, 100.), &renderer).unwrap();
    let id = host.snapshot().unwrap().last().unwrap().id;
    host.focus(id).unwrap();
    host.layout(Size::new(width, 100.), &renderer).unwrap();
    (value, host, id, renderer)
}
#[test]
fn wrapped_line_movement_and_boundaries_use_shaped_lines_and_caret_affinity() {
    let text = "abcdefghijabcdefghijabcdefghijabcdefghijabcdefghijabcdefghij";
    let (_, host, id, renderer) = mounted(text, 120.);
    let layout = renderer.text(
        text,
        TextStyle {
            wrap: true,
            ..Default::default()
        },
        100.,
    );
    assert!(layout.lines.len() > 2);
    host.select_text(
        id,
        Selection {
            anchor: 2,
            caret: 2,
        },
    )
    .unwrap();
    host.text_command(TextCommand::LineEnd, false).unwrap();
    let caret = host.text_input().unwrap().unwrap().selection.caret;
    assert_eq!(caret, layout.lines[0].range.end);
    let r = host.text_rectangles(id, caret..caret)[0];
    let node = host.snapshot().unwrap().last().unwrap().clone();
    assert!((r.y - node.text_bounds.y - layout.lines[0].bounds.y).abs() < 1.);
    let click = Pointer {
        kind: PointerKind::Down,
        position: Point {
            x: node.bounds.x + node.bounds.width - 12.,
            y: r.y + r.height / 2.,
        },
        button: 0,
        modifiers: Default::default(),
    };
    host.pointer(click).unwrap();
    assert_eq!(host.text_input().unwrap().unwrap().selection.caret, caret);
    assert!((host.text_rectangles(id, caret..caret)[0].y - r.y).abs() < 1.);
    host.pointer(Pointer {
        kind: PointerKind::Up,
        ..click
    })
    .unwrap();
    host.text_command(TextCommand::Down, true).unwrap();
    let selection = host.text_input().unwrap().unwrap().selection;
    assert_eq!(selection.anchor, caret);
    assert!(
        selection.caret >= layout.lines[1].range.start
            && selection.caret <= layout.lines[1].range.end
    );
    host.text_command(TextCommand::LineStart, false).unwrap();
    assert_eq!(
        host.text_input().unwrap().unwrap().selection.caret,
        layout.lines[1].range.start
    );
    host.text_command(TextCommand::Start, false).unwrap();
    assert_eq!(host.text_input().unwrap().unwrap().selection.caret, 0);
}
#[test]
fn vertical_navigation_restores_preferred_x_after_a_short_line_and_pages() {
    let text = "MMMMMMMMMM\nM\nMMMMMMMMMM\nM\nMMMMMMMMMM\nM\nMMMMMMMMMM\nM\nMMMMMMMMMM";
    let (_, host, id, _) = mounted(text, 400.);
    host.select_text(
        id,
        Selection {
            anchor: 5,
            caret: 5,
        },
    )
    .unwrap();
    host.text_command(TextCommand::Down, false).unwrap();
    assert_eq!(host.text_input().unwrap().unwrap().selection.caret, 12);
    host.text_command(TextCommand::Down, false).unwrap();
    assert_eq!(host.text_input().unwrap().unwrap().selection.caret, 18);
    host.text_command(TextCommand::Up, false).unwrap();
    host.text_command(TextCommand::Up, false).unwrap();
    assert_eq!(host.text_input().unwrap().unwrap().selection.caret, 5);
    host.text_command(TextCommand::PageDown, true).unwrap();
    let s = host.text_input().unwrap().unwrap().selection;
    assert_eq!(s.anchor, 5);
    assert!(s.caret > 18);
    host.text_command(TextCommand::PageUp, false).unwrap();
    assert!(host.text_input().unwrap().unwrap().selection.caret < s.caret);
}
#[test]
fn visual_line_deletion_keeps_following_wrapped_text_and_undo_is_discrete() {
    let text = "abcdefghijabcdefghijabcdefghijabcdefghij";
    let (value, host, _, renderer) = mounted(text, 120.);
    let layout = renderer.text(
        text,
        TextStyle {
            wrap: true,
            ..Default::default()
        },
        100.,
    );
    host.text_command(TextCommand::LineEnd, false).unwrap();
    host.text_command(TextCommand::DeleteLineBackward, false)
        .unwrap();
    assert_eq!(value.get(), text[layout.lines[0].range.end..]);
    host.text_command(TextCommand::Undo, false).unwrap();
    assert_eq!(value.get(), text);
}
#[test]
fn utf16_line_indices_and_newline_carets_remain_utf8_and_grapheme_safe() {
    let text = "😀e\u{301}\n👨‍👩‍👧‍👦\n";
    let renderer = renderer();
    let layout = renderer.text(
        text,
        TextStyle {
            wrap: true,
            ..Default::default()
        },
        300.,
    );
    assert_eq!(layout.lines.len(), 3);
    assert_eq!(layout.lines[0].range, 0..7);
    assert_eq!(layout.lines[1].range, 8..33);
    assert_eq!(layout.lines[2].range, 34..34);
    for line in &layout.lines {
        assert!(text.is_char_boundary(line.range.start));
        assert!(text.is_char_boundary(line.range.end));
    }
    let last = layout.caret_rect(text.len(), CaretAffinity::Downstream);
    assert!(last.y > layout.lines[1].bounds.y);
}
#[test]
fn double_and_triple_click_select_words_and_visual_lines_then_drag_whole_units() {
    let text = "alpha beta gamma delta epsilon zeta eta theta";
    let (_, host, id, renderer) = mounted(text, 180.);
    let layout = renderer.text(
        text,
        TextStyle {
            wrap: true,
            ..Default::default()
        },
        160.,
    );
    let node = host.snapshot().unwrap().last().unwrap().clone();
    let p = layout.caret_rect(7, CaretAffinity::Downstream);
    let point = Point {
        x: node.text_bounds.x + p.x + 1.,
        y: node.text_bounds.y + p.y + p.height / 2.,
    };
    let event = |kind, position| Pointer {
        kind,
        position,
        button: 0,
        modifiers: Default::default(),
    };
    host.pointer_click(event(PointerKind::Down, point), 2)
        .unwrap();
    assert_eq!(host.text_input().unwrap().unwrap().selection.range(), 6..10);
    let p = layout.caret_rect(2, CaretAffinity::Downstream);
    host.pointer(event(
        PointerKind::Move,
        Point {
            x: node.text_bounds.x + p.x,
            y: point.y,
        },
    ))
    .unwrap();
    assert_eq!(
        host.text_input().unwrap().unwrap().selection,
        Selection {
            anchor: 10,
            caret: 0
        }
    );
    host.pointer(event(PointerKind::Up, point)).unwrap();
    host.pointer_click(event(PointerKind::Down, point), 3)
        .unwrap();
    assert_eq!(
        host.text_input().unwrap().unwrap().selection.range(),
        layout.lines[0].range.start..layout.lines[0].end_including_newline
    );
    host.pointer(event(PointerKind::Up, point)).unwrap();
    assert!(host.next_deadline().is_none());
    assert_eq!(host.focused(), Some(id));
}
