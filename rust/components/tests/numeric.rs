use scars_ui::*;
use std::{
    rc::Rc,
    time::{Duration, Instant},
};
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
fn node(host: &Host, label: &str) -> Node {
    host.snapshot()
        .unwrap()
        .into_iter()
        .find(|n| n.label == label)
        .unwrap()
}
fn field(host: &Host) -> Node {
    host.snapshot()
        .unwrap()
        .into_iter()
        .find(|n| n.kind == Kind::TextField)
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
fn validation_distinguishes_drafts_and_steps_exact_integers_without_corrupting_invalid_edits() {
    let value = State::new(9_007_199_254_740_993i64);
    let events = State::new(Vec::<Validation>::new());
    let number = Numeric::new(value.binding(), i64::MIN, i64::MAX, 1)
        .significant_digits(3)
        .on_validation(events.handler(|state, event| state.update(|v| v.push(event))));
    let edit = number.draft.binding();
    assert_eq!(edit.get(), "9007199254740993");
    for (text, status) in [
        ("-", ValidationStatus::Incomplete),
        ("broken", ValidationStatus::Invalid),
        ("9223372036854775808", ValidationStatus::OutOfRange),
    ] {
        edit.set(text.into());
        assert_eq!(number.validation().status, status);
        assert!(number.validation().dirty);
        assert!(!number.commit());
        assert!(!number.step(true));
        assert_eq!(edit.get(), text);
        assert_eq!(value.get(), 9_007_199_254_740_993);
    }
    edit.set("9007199254740995".into());
    assert!(number.step(true));
    assert_eq!(value.get(), 9_007_199_254_740_996);
    assert_eq!(number.validation().status, ValidationStatus::Valid);
    assert!(!number.validation().dirty);
    assert_eq!(events.get().last().unwrap().status, ValidationStatus::Valid);
    let accepted = State::new(3i64);
    let read = accepted.clone();
    let write = accepted.clone();
    let guarded = Numeric::new(
        Binding::new(
            move || read.get(),
            move |value| {
                if value <= 5 {
                    write.set(value)
                }
            },
        ),
        0,
        10,
        1,
    );
    guarded.draft.binding().set("8".into());
    assert!(!guarded.commit());
    assert!(!guarded.step(true));
    assert_eq!(guarded.draft.binding().get(), "8");
    assert_eq!(accepted.get(), 3);
    let unsigned = Numeric::new(State::new(0u64).binding(), 0, u64::MAX, 1);
    unsigned.draft.binding().set("-1".into());
    assert_eq!(unsigned.validation().status, ValidationStatus::OutOfRange);
}
#[test]
fn precision_does_not_round_pristine_values_or_hide_exact_external_changes() {
    let value = State::new(1.2345);
    let number = Numeric::new(value.binding(), 1.234, 2., 0.01).significant_digits(3);
    assert_eq!(number.draft.binding().get(), "1.23");
    assert!(number.commit());
    assert_eq!(value.get(), 1.2345);
    number.draft.binding().set("bad".into());
    value.set(1.2346);
    assert_eq!(number.draft.binding().get(), "1.23");
    assert!(!number.validation().dirty);
    assert!(number.draft.error().is_none());
    assert!(number.commit());
    assert_eq!(value.get(), 1.2346);
    let outside =
        Numeric::new(State::new(1.23451).binding(), 0., 1.2345, 0.1).significant_digits(3);
    assert_eq!(outside.draft.binding().get(), "1.23");
    assert_eq!(outside.validation().status, ValidationStatus::OutOfRange);
    assert!(outside.validation().message.is_some());
    assert!(!outside.can_step(true));
    let scientific =
        Numeric::new(State::new(0.000012345).binding(), 0., 1., 0.1).significant_digits(3);
    assert_eq!(scientific.draft.binding().get(), "1.23e-5");
    let extreme =
        Numeric::new(State::new(f64::MAX).binding(), 0., f64::MAX, 1.).significant_digits(3);
    assert!(
        extreme
            .draft
            .binding()
            .get()
            .parse::<f64>()
            .unwrap()
            .is_finite()
    );
    assert!(extreme.commit());
    for text in ["", "-.", "1e", "1e-"] {
        scientific.draft.binding().set(text.into());
        assert_eq!(scientific.validation().status, ValidationStatus::Incomplete);
    }
}
#[test]
fn blur_submit_cancel_and_readonly_policies_share_the_draft() {
    for commit_on_blur in [true, false] {
        let value = State::new(3i64);
        let submitted = State::new(0);
        let cancelled = State::new(0);
        let number = Numeric::new(value.binding(), 0, 10, 1)
            .commit_on_blur(commit_on_blur)
            .on_submit(submitted.action(|v| v.update(|v| *v += 1)))
            .on_cancel(cancelled.action(|v| v.update(|v| *v += 1)));
        let host = Host::new({
            let number = number.clone();
            move || {
                View::column(vec![
                    numeric_field("Count", &number),
                    View::button("Other", || {}),
                ])
            }
        });
        host.layout(Size::new(400., 220.), &Metrics).unwrap();
        let id = field(&host).id;
        let other = node(&host, "Other").id;
        host.focus(id).unwrap();
        host.set_text(id, "7").unwrap();
        host.focus(other).unwrap();
        assert_eq!(value.get(), if commit_on_blur { 7 } else { 3 });
        host.focus(id).unwrap();
        host.text_command(TextCommand::Enter, false).unwrap();
        assert_eq!(value.get(), 7);
        assert_eq!(submitted.get(), 1);
        host.set_text(id, "-").unwrap();
        host.focus(other).unwrap();
        assert_eq!(value.get(), 7);
        assert_eq!(number.draft.binding().get(), "-");
        host.focus(id).unwrap();
        host.text_command(TextCommand::Escape, false).unwrap();
        assert_eq!(number.draft.binding().get(), "7");
        assert_eq!(cancelled.get(), 1);
        host.set_text(id, "8").unwrap();
        host.set_active(false);
        host.blur();
        assert_eq!(value.get(), if commit_on_blur { 8 } else { 7 });
    }
    let value = State::new(3i64);
    let number = Numeric::new(value.binding(), 0, 10, 1).read_only(true);
    number.draft.binding().set("8".into());
    assert_eq!(number.draft.binding().get(), "3");
    assert!(!number.step(true) && !number.commit());
}
#[test]
fn held_stepper_uses_deadlines_clamps_overflow_and_settles_at_the_bound() {
    let value = State::new(i64::MAX - 11);
    let number = Numeric::new(value.binding(), i64::MIN, i64::MAX, 10);
    let host = Host::new(move || stepper("Count", &number));
    host.layout(Size::new(200., 50.), &Metrics).unwrap();
    let up = node(&host, "Increase Count");
    let p = Point {
        x: up.bounds.x + 5.,
        y: up.bounds.y + 5.,
    };
    host.pointer(pointer(PointerKind::Down, p)).unwrap();
    assert_eq!(value.get(), i64::MAX - 1);
    host.layout(Size::new(200., 50.), &Metrics).unwrap();
    assert!(host.next_deadline().is_some());
    assert!(!host.needs_animation());
    host.run_due_at(Instant::now() + Duration::from_secs(1));
    assert_eq!(value.get(), i64::MAX);
    assert!(host.next_deadline().is_none());
    host.pointer(pointer(PointerKind::Up, p)).unwrap();
    assert_eq!(value.get(), i64::MAX);
}
#[test]
fn repeat_cancels_on_release_cancel_blur_deactivation_hide_remove_disable_and_pointer_exit() {
    for mode in 0..9 {
        let count = State::new(0);
        let visible = State::new(true);
        let enabled = State::new(true);
        let hidden = State::new(false);
        let host = Host::new({
            let (count, visible, enabled, hidden) = (
                count.clone(),
                visible.clone(),
                enabled.clone(),
                hidden.clone(),
            );
            move || view! {column{if visible.get(){button("Repeat",width:100,height:34,repeat:Repeat::default(),enabled:enabled.get(),hidden:hidden.get(),on_click:count.action(|v|v.update(|v|*v+=1)));}button("Other",on_click:||{});}}
        });
        host.layout(Size::new(200., 100.), &Metrics).unwrap();
        let p = Point { x: 10., y: 10. };
        host.pointer(pointer(PointerKind::Down, p)).unwrap();
        host.layout(Size::new(200., 100.), &Metrics).unwrap();
        assert_eq!(count.get(), 1);
        assert!(host.next_deadline().is_some());
        match mode {
            0 => {
                host.pointer(pointer(PointerKind::Up, p)).unwrap();
            }
            1 => {
                host.pointer(pointer(PointerKind::Cancel, p)).unwrap();
            }
            2 => host.blur(),
            3 => host.set_active(false),
            4 => hidden.set(true),
            5 => visible.set(false),
            6 => enabled.set(false),
            7 => {
                host.pointer(pointer(PointerKind::Move, Point { x: 190., y: 90. }))
                    .unwrap();
            }
            _ => {
                host.focus(node(&host, "Other").id).unwrap();
            }
        }
        assert!(host.next_deadline().is_none());
        host.run_due_at(Instant::now() + Duration::from_secs(2));
        assert_eq!(count.get(), 1);
    }
}
#[test]
fn stepper_is_one_keyboard_stop_with_exact_value_queries_and_actions() {
    let value = State::new(9_007_199_254_740_993i64);
    let number = Numeric::new(value.binding(), i64::MIN, i64::MAX, 1);
    let host = Host::new(move || {
        View::column(vec![stepper("Count", &number), View::button("Next", || {})])
    });
    host.layout(Size::new(240., 100.), &Metrics).unwrap();
    let id = host
        .snapshot()
        .unwrap()
        .into_iter()
        .find(|n| n.kind == Kind::Stepper)
        .unwrap()
        .id;
    host.focus_next(false).unwrap();
    assert_eq!(host.focused(), Some(id));
    assert!(host.text_input().unwrap().is_none());
    assert!(
        host.key(KeyEvent {
            key: Key::Up,
            down: true,
            repeat: false,
            modifiers: Default::default()
        })
        .unwrap()
    );
    assert_eq!(value.get(), 9_007_199_254_740_994);
    assert!(host.step_control(id, false).unwrap());
    assert_eq!(value.get(), 9_007_199_254_740_993);
    assert_eq!(
        host.snapshot()
            .unwrap()
            .into_iter()
            .find(|n| n.id == id)
            .unwrap()
            .stepper
            .unwrap()
            .value,
        "9007199254740993"
    );
    host.focus_next(false).unwrap();
    assert_eq!(host.focused(), Some(node(&host, "Next").id));
}

#[test]
fn generic_value_fields_cancel_with_escape_and_expose_readonly_to_the_host() {
    let value = State::new(3i64);
    let draft = Draft::new(
        value.binding(),
        |s| s.parse::<i64>().map_err(|e| e.to_string()),
        |v| v.to_string(),
    );
    let host = Host::new({
        let d = draft.clone();
        move || value_field("Custom value", &d)
    });
    host.layout(Size::new(400., 220.), &Metrics).unwrap();
    let id = field(&host).id;
    host.focus(id).unwrap();
    host.set_text(id, "unfinished").unwrap();
    assert!(host.text_command(TextCommand::Escape, false).unwrap());
    assert_eq!(draft.binding().get(), "3");
    assert_eq!(value.get(), 3);
    let readonly = draft.read_only(true);
    let host = Host::new(move || value_field("Custom value", &readonly));
    host.layout(Size::new(400., 220.), &Metrics).unwrap();
    assert!(field(&host).read_only);
    assert!(!node(&host, "Apply").enabled);
    assert!(!node(&host, "Revert").enabled);
}

#[test]
fn stepper_announces_the_valid_staged_value_that_its_next_action_will_adjust() {
    let value = State::new(3i64);
    let number = Numeric::new(value.binding(), 0, 10, 1);
    let host = Host::new({
        let n = number.clone();
        move || stepper("Count", &n)
    });
    host.layout(Size::new(200., 50.), &Metrics).unwrap();
    number.binding().set("7".into());
    let control = node(&host, "Count");
    assert_eq!(control.stepper.unwrap().value, "7");
    assert!(host.step_control(control.id, true).unwrap());
    assert_eq!(value.get(), 8);
}
