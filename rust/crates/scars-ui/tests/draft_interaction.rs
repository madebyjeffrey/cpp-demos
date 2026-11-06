use scars_ui::*;
use std::rc::Rc;
struct Metrics;
impl Measure for Metrics {
    fn text(&self, s: &str, _: TextStyle, _: f64) -> TextLayout {
        TextLayout {
            size: Size::new(s.chars().count() as f64 * 7., 18.),
            baseline: 14.,
            lines: vec![],
            placeholders: vec![],
            hit_test: Rc::new(|_| 0),
            range_rects: Rc::new(|_| vec![]),
        }
    }
    fn image(&self, _: &Bitmap) -> Result<Size, Error> {
        Ok(Size::default())
    }
}
fn layout(h: &Host) {
    h.layout(Size::new(1000., 1800.), &Metrics).unwrap();
}
fn field(h: &Host) -> NodeId {
    let nodes = h.snapshot().unwrap();
    nodes
        .iter()
        .find(|n| n.label == "Hex color")
        .or_else(|| {
            nodes
                .iter()
                .find(|n| n.key.as_deref() == Some("$datetime-field"))
        })
        .or_else(|| {
            nodes
                .iter()
                .find(|n| n.kind == Kind::TextField && n.label != "Time zone")
        })
        .unwrap()
        .id
}
fn composing(h: &Host) {
    assert!(h.text_input().unwrap().unwrap().marked.is_some());
}
fn exercise(h: Host, binding: Binding<String>, source: impl Fn() -> String, wanted: &str) {
    let original = source();
    let canonical = binding.get();
    layout(&h);
    let field = field(&h);
    h.focus(field).unwrap();
    binding.set(wanted.into());
    h.mark_text(
        "unfinished preedit",
        0..0,
        Some(0..h.text_input().unwrap().unwrap().text.len()),
    )
    .unwrap();
    for node in h.snapshot().unwrap() {
        if !node.enabled {
            continue;
        }
        if node.label == "Apply"
            || node.label == "Change zone"
            || node.calendar_day.is_some()
            || node.kind == Kind::Segment
        {
            assert!(
                !h.activate(node.id).unwrap(),
                "action during marked text: {}",
                node.label
            );
        }
        if node.stepper.is_some() {
            assert!(!h.step_control(node.id, true).unwrap());
        }
        if node.kind == Kind::Slider {
            assert!(!h.set_number(node.id, 0.).unwrap());
        }
        if node.kind == Kind::Segment {
            assert!(!h.set_selected(node.id, true).unwrap());
        }
    }
    composing(&h);
    assert_eq!(source(), original);
    assert_eq!(binding.get(), wanted);
    if let Some(cancel) = h
        .snapshot()
        .unwrap()
        .iter()
        .find(|n| n.label == "Revert" || n.label == "Cancel")
        .cloned()
    {
        assert!(h.activate(cancel.id).unwrap());
        assert!(h.text_input().unwrap().unwrap().marked.is_none());
    } else {
        h.text_command(TextCommand::Escape, false).unwrap();
        assert_eq!(binding.get(), wanted);
        h.text_command(TextCommand::Escape, false).unwrap();
    }
    assert_eq!(binding.get(), canonical);
    assert_eq!(source(), original);
    layout(&h);
    h.focus(field).unwrap();
    h.mark_text(
        wanted,
        0..0,
        Some(0..h.text_input().unwrap().unwrap().text.len()),
    )
    .unwrap();
    h.text_command(TextCommand::Enter, false).unwrap();
    assert_eq!(binding.get(), wanted);
    assert_eq!(source(), original, "first Enter confirms composition only");
    h.text_command(TextCommand::Enter, false).unwrap();
    assert_ne!(
        source(),
        original,
        "second Enter submits the confirmed value"
    );
}

#[test]
fn typed_controls_guard_direct_actions_and_cancel_preedit_before_reverting() {
    macro_rules! check {
        ($initial:expr, $raw:expr, $build:expr, $render:expr) => {{
            let value = State::new($initial);
            let editor = ($build)(value.binding());
            let binding = editor.binding();
            let h = Host::new(move || ($render)(&editor));
            exercise(h, binding, || value.get().to_string(), $raw);
        }};
    }
    check!(
        3i64,
        "7",
        |b| Numeric::new(b, 0, 10, 1).commit_on_blur(false),
        |p: &Numeric<i64>| numeric_field("Count", p)
    );
    check!(
        "2024-02-01".parse::<Date>().unwrap(),
        "2024-02-02",
        |b| DatePicker::new(b).commit_on_blur(false),
        DatePicker::view
    );
    check!(
        "12:30".parse::<Time>().unwrap(),
        "13:40",
        |b| TimePicker::new(b)
            .hour_cycle(HourCycle::H24)
            .commit_on_blur(false),
        TimePicker::view
    );
    check!(
        "2024-02-01T12:30".parse::<DateTime>().unwrap(),
        "2024-02-02T13:40",
        |b| DateTimePicker::new(b).commit_on_blur(false),
        DateTimePicker::view
    );
    check!(
        "2024-02-01T12:30Z".parse::<Timestamp>().unwrap(),
        "2024-02-02T13:40",
        |b| ZonedPicker::new(b, "UTC").unwrap().commit_on_blur(false),
        ZonedPicker::view
    );
    check!(
        Color::rgba(1, 2, 3, 4),
        "#12345678",
        ColorPicker::new,
        ColorPicker::view
    );
    check!("12:30".parse::<Time>().unwrap(), "13:40", time_draft, |d| {
        value_field("Custom time", d)
    });
}

#[test]
fn blur_means_leaving_the_whole_editor_and_revert_does_not_commit_first() {
    macro_rules! check {
        ($initial:expr, $raw:expr, $build:expr, $render:expr) => {
            for (auto, mode) in [false, true]
                .into_iter()
                .flat_map(|auto| (0..3).map(move |mode| (auto, mode)))
            {
                let value = State::new($initial);
                let editor = ($build)(value.binding(), auto);
                let binding = editor.binding();
                let original = value.get();
                let canonical = binding.get();
                let h = Host::new(move || {
                    View::column(vec![($render)(&editor), View::button("After", || {})])
                });
                layout(&h);
                let input = field(&h);
                let internal = h
                    .snapshot()
                    .unwrap()
                    .iter()
                    .find(|n| n.label == "Revert" || n.kind == Kind::Stepper)
                    .unwrap()
                    .id;
                let after = h
                    .snapshot()
                    .unwrap()
                    .iter()
                    .find(|n| n.label == "After")
                    .unwrap()
                    .id;
                h.focus(input).unwrap();
                binding.set($raw.into());
                h.focus(internal).unwrap();
                assert_eq!(value.get(), original, "internal focus must not apply early");
                if h.snapshot()
                    .unwrap()
                    .iter()
                    .any(|n| n.id == internal && n.label == "Revert")
                {
                    h.activate(internal).unwrap();
                    assert_eq!(binding.get(), canonical);
                    assert_eq!(value.get(), original);
                    h.focus(input).unwrap();
                    binding.set($raw.into());
                    h.focus(internal).unwrap();
                }
                match mode {
                    0 => {
                        h.focus(after).unwrap();
                    }
                    1 => h.set_active(false),
                    _ => h.blur(),
                }
                assert_eq!(
                    value.get() != original,
                    auto,
                    "leaving from an internal control must honor blur policy"
                );
            }
        };
    }
    check!(
        3i64,
        "7",
        |b, auto| Numeric::new(b, 0, 10, 1).commit_on_blur(auto),
        |p: &Numeric<i64>| numeric_field("Count", p)
    );
    check!(
        "2024-02-01".parse::<Date>().unwrap(),
        "2024-02-02",
        |b, auto| DatePicker::new(b).commit_on_blur(auto),
        DatePicker::view
    );
    check!(
        "12:30".parse::<Time>().unwrap(),
        "13:40",
        |b, auto| TimePicker::new(b).commit_on_blur(auto),
        TimePicker::view
    );
    check!(
        "2024-02-01T12:30".parse::<DateTime>().unwrap(),
        "2024-02-02T13:40",
        |b, auto| DateTimePicker::new(b).commit_on_blur(auto),
        DateTimePicker::view
    );
    check!(
        "2024-02-01T12:30Z".parse::<Timestamp>().unwrap(),
        "2024-02-02T13:40",
        |b, auto| ZonedPicker::new(b, "UTC").unwrap().commit_on_blur(auto),
        ZonedPicker::view
    );
}

#[test]
fn preedit_guards_are_scoped_to_the_editor_that_owns_it() {
    let first = Numeric::new(State::new(3i64).binding(), 0, 10, 1).commit_on_blur(false);
    let other = State::new(3i64);
    let second = Numeric::new(other.binding(), 0, 10, 1).commit_on_blur(false);
    let h = Host::new(move || {
        View::column(vec![
            numeric_field("First", &first),
            numeric_field("Other", &second),
        ])
    });
    layout(&h);
    h.focus(field(&h)).unwrap();
    h.mark_text("unfinished", 0..0, None).unwrap();
    let target = h
        .snapshot()
        .unwrap()
        .iter()
        .find(|n| n.kind == Kind::Stepper && n.label == "Other")
        .unwrap()
        .id;
    assert!(h.step_control(target, true).unwrap());
    assert_eq!(other.get(), 4);
    composing(&h);
}

#[test]
fn datetime_cancel_preserves_calendar_navigation_started_by_validation() {
    use std::cell::{Cell, RefCell};
    let source = State::new("2024-02-01T12:30".parse::<DateTime>().unwrap());
    let armed = Rc::new(Cell::new(false));
    let slot: Rc<RefCell<Option<Host>>> = Rc::new(RefCell::new(None));
    let picker = DateTimePicker::new(source.binding()).on_validation({
        let a = armed.clone();
        let s = slot.clone();
        move |_| {
            if a.replace(false) {
                s.borrow()
                    .as_ref()
                    .unwrap()
                    .key(KeyEvent {
                        key: Key::PageDown,
                        down: true,
                        repeat: false,
                        modifiers: Default::default(),
                    })
                    .unwrap();
            }
        }
    });
    let h = Host::new({
        let p = picker.clone();
        move || p.view()
    });
    *slot.borrow_mut() = Some(h.clone());
    picker.binding().set("2024-02-02T13:40".into());
    layout(&h);
    let day = h
        .snapshot()
        .unwrap()
        .iter()
        .find(|n| n.calendar_day.is_some_and(|d| d.current))
        .unwrap()
        .id;
    h.focus(day).unwrap();
    armed.set(true);
    picker.cancel();
    assert_eq!(
        picker.calendar().cursor,
        "2024-03-01".parse::<Date>().unwrap()
    );
    slot.borrow_mut().take();
}

#[test]
fn hidden_disabled_and_unmounted_editors_cancel_preedit_but_keep_model_owned_drafts() {
    for mode in 0..3 {
        let value = State::new(3i64);
        let number = Numeric::new(value.binding(), 0, 10, 1).commit_on_blur(false);
        let available = State::new(true);
        let h = Host::new({
            let (n, a) = (number.clone(), available.clone());
            move || {
                if mode == 2 && !a.get() {
                    View::fragment(vec![])
                } else {
                    numeric_field("Count", &n)
                        .hidden(mode == 0 && !a.get())
                        .enabled(mode != 1 || a.get())
                }
            }
        });
        layout(&h);
        h.focus(field(&h)).unwrap();
        number.binding().set("7".into());
        h.mark_text("unfinished", 0..0, None).unwrap();
        available.set(false);
        layout(&h);
        assert_eq!(value.get(), 3);
        assert_eq!(number.binding().get(), "7");
        assert!(h.next_deadline().is_none());
        available.set(true);
        layout(&h);
        h.focus(field(&h)).unwrap();
        let input = h.text_input().unwrap().unwrap();
        assert!(input.marked.is_none(), "mode {mode}");
        assert_eq!(input.text, "7");
        drop(h);
        assert_eq!(number.binding().get(), "7");
        assert_eq!(value.get(), 3);
    }
}

#[test]
fn color_channels_share_numeric_prefix_finite_and_range_statuses() {
    let source = State::new(Color::rgba(1, 2, 3, 4));
    let picker = ColorPicker::new(source.binding());
    let h = Host::new({
        let p = picker.clone();
        move || p.view()
    });
    layout(&h);
    let red = h
        .snapshot()
        .unwrap()
        .iter()
        .find(|n| n.label == "Red value")
        .unwrap()
        .id;
    for (text, status) in [
        ("1e-", ValidationStatus::Incomplete),
        ("NaN", ValidationStatus::Invalid),
        ("inf", ValidationStatus::Invalid),
        ("256", ValidationStatus::OutOfRange),
    ] {
        h.set_text(red, text).unwrap();
        assert_eq!(picker.validation().status, status, "{text}");
        assert!(!picker.commit());
        assert_eq!(source.get(), Color::rgba(1, 2, 3, 4));
    }
}

#[test]
fn equal_display_source_replacements_cancel_stale_preedit_across_typed_controls() {
    macro_rules! check {
        ($initial:expr, $other:expr, $build:expr, $render:expr) => {{
            let source = State::new($initial);
            let initial = source.get();
            let editor = ($build)(source.binding());
            let binding = editor.binding();
            let canonical = binding.get();
            let h = Host::new(move || ($render)(&editor));
            layout(&h);
            h.focus(field(&h)).unwrap();
            h.mark_text("unconfirmed replacement", 0..0, None).unwrap();
            source.set($other);
            source.set(initial);
            layout(&h);
            let input = h.text_input().unwrap().unwrap();
            assert!(input.marked.is_none(), "{canonical}");
            assert_eq!(input.text, canonical);
            assert_eq!(binding.get(), canonical);
            assert_eq!(source.get(), initial);
        }};
    }
    check!(3i64, 4, |b| Numeric::new(b, 0, 10, 1), |p: &Numeric<
        i64,
    >| numeric_field(
        "Count", p
    ));
    check!(
        "2024-02-01".parse::<Date>().unwrap(),
        "2024-02-02".parse::<Date>().unwrap(),
        DatePicker::new,
        DatePicker::view
    );
    check!(
        "12:30".parse::<Time>().unwrap(),
        "13:30".parse::<Time>().unwrap(),
        TimePicker::new,
        TimePicker::view
    );
    check!(
        "2024-02-01T12:30".parse::<DateTime>().unwrap(),
        "2024-02-02T12:30".parse::<DateTime>().unwrap(),
        DateTimePicker::new,
        DateTimePicker::view
    );
    check!(
        "2024-02-01T12:30Z".parse::<Timestamp>().unwrap(),
        "2024-02-02T12:30Z".parse::<Timestamp>().unwrap(),
        |b| ZonedPicker::new(b, "UTC").unwrap(),
        ZonedPicker::view
    );
    check!(
        Color::rgba(1, 2, 3, 4),
        Color::rgba(5, 6, 7, 8),
        ColorPicker::new,
        ColorPicker::view
    );
}

#[test]
fn revision_transport_covers_hidden_color_channels_without_resetting_own_edit_history() {
    let value = State::new(Color::rgba(1, 2, 3, 4));
    let picker = ColorPicker::new(value.binding()).alpha(false);
    let h = Host::new(move || picker.view());
    layout(&h);
    for label in ["Red value", "Green value", "Blue value", "Hex color"] {
        let id = h
            .snapshot()
            .unwrap()
            .iter()
            .find(|n| n.label == label)
            .unwrap()
            .id;
        h.focus(id).unwrap();
        h.mark_text("pending", 0..0, None).unwrap();
        value.set(Color::rgba(1, 2, 3, 5));
        layout(&h);
        assert!(h.text_input().unwrap().unwrap().marked.is_none(), "{label}");
        assert_eq!(value.get(), Color::rgba(1, 2, 3, 5));
        value.set(Color::rgba(1, 2, 3, 4));
        layout(&h);
    }
    let number = Numeric::new(State::new(3i64).binding(), 0, 10, 1).commit_on_blur(false);
    let h = Host::new(move || numeric_field("Count", &number));
    layout(&h);
    h.focus(field(&h)).unwrap();
    h.insert_text("7", Some(0..1)).unwrap();
    layout(&h);
    assert!(h.text_input().unwrap().unwrap().can_undo);
    h.text_command(TextCommand::Undo, false).unwrap();
    layout(&h);
    assert_eq!(h.text_input().unwrap().unwrap().text, "3");
    assert!(h.text_input().unwrap().unwrap().can_redo);
}
