use scars_ui::*;
use std::{cell::Cell, rc::Rc};

fn time(s: &str) -> Time {
    s.parse().unwrap()
}
struct Metrics;
impl Measure for Metrics {
    fn text(&self, s: &str, _: TextStyle, _: f64) -> TextLayout {
        TextLayout {
            size: Size::new(s.len() as f64 * 7., 18.),
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
fn key(key: Key) -> KeyEvent {
    KeyEvent {
        key,
        down: true,
        repeat: false,
        modifiers: Default::default(),
    }
}
fn layout(h: &Host) {
    h.layout(Size::new(480., 400.), &Metrics).unwrap();
}
fn field(h: &Host) -> NodeId {
    h.snapshot()
        .unwrap()
        .into_iter()
        .find(|n| n.kind == Kind::TextField)
        .unwrap()
        .id
}

#[test]
fn edits_preserve_hidden_precision_and_explicit_seconds_replace_it_exactly() {
    let value = State::new(time("12:34:56.987654321"));
    let picker = TimePicker::new(value.binding());
    assert_eq!(picker.binding().get(), "12:34");
    assert!(picker.commit());
    assert_eq!(value.get(), time("12:34:56.987654321"));
    picker.binding().set("13:20".into());
    assert!(picker.commit());
    assert_eq!(value.get(), time("13:20:56.987654321"));
    assert!(!picker.can_step(TimePart::Second, true));
    picker.binding().set("13:20:00".into());
    assert!(!picker.commit());
    let picker = picker.show_seconds(true);
    assert_eq!(picker.binding().get(), "13:20:56.987654321");
    picker.binding().set("14:00".into());
    assert!(picker.commit());
    assert_eq!(value.get(), time("14:00:56.987654321"));
    picker.binding().set("14:00:00.000000001".into());
    assert!(picker.commit());
    assert_eq!(value.get(), time("14:00:00.000000001"));
}

#[test]
fn twelve_hour_cycles_handle_midnight_noon_and_reject_ambiguous_input() {
    let value = State::new(Time::MIN);
    let picker = TimePicker::new(value.binding()).hour_cycle(HourCycle::H12);
    assert_eq!(picker.binding().get(), "12:00 AM");
    for (text, expected) in [
        ("12:00 PM", "12:00"),
        ("1:15 pm", "13:15"),
        ("12:00 AM", "00:00"),
    ] {
        picker.binding().set(text.into());
        assert!(picker.commit(), "{text}");
        assert_eq!(value.get(), time(expected));
    }
    assert!(picker.step(TimePart::Period, false));
    assert_eq!(picker.binding().get(), "12:00 PM");
    assert!(picker.step(TimePart::Period, true));
    assert_eq!(value.get(), Time::MIN);
    for invalid in [
        "00:00 AM",
        "13:00 PM",
        "12:00",
        "12:00 PM ",
        "12:00 AMZ",
        "2024-01-01T12:00 AM",
    ] {
        picker.binding().set(invalid.into());
        assert!(!picker.commit(), "{invalid}");
        assert!(!picker.step(TimePart::Hour, true));
        assert_eq!(picker.binding().get(), invalid);
        assert_eq!(value.get(), Time::MIN);
    }
}

#[test]
fn locale_digits_periods_and_fraction_separator_match_the_editable_grammar() {
    for (tag, text) in [
        ("en-US", "01:02:03.123456789 PM"),
        ("de-DE", "01:02:03,123456789 PM"),
        ("fr-FR", "01:02:03,123456789 PM"),
        ("ar-EG", "٠١:٠٢:٠٣٫١٢٣٤٥٦٧٨٩ م"),
    ] {
        let value = State::new(time("13:02:03.123456789"));
        let picker = TimePicker::new(value.binding())
            .hour_cycle(HourCycle::H12)
            .show_seconds(true)
            .locale(EditLocale::new(tag).unwrap());
        assert_eq!(picker.binding().get(), text);
        picker.binding().set(text.into());
        assert!(picker.commit());
        assert_eq!(value.get(), time("13:02:03.123456789"));
        if tag == "ar-EG" {
            picker.binding().set("٠١:02:٠٣٫١ م".into());
            assert!(!picker.commit(), "mixed digit sets");
            picker.binding().set("01:02:03.1 PM".into());
            assert!(!picker.commit(), "wrong fractional separator/period");
        } else if tag != "en-US" {
            picker.binding().set("01:02:03.1 PM".into());
            assert!(!picker.commit(), "wrong fractional separator");
        }
    }
}

#[test]
fn exact_steps_carry_components_and_reject_day_bounds_custom_rules_and_bad_drafts() {
    let value = State::new(time("12:58:59.123456789"));
    let picker = TimePicker::new(value.binding())
        .show_seconds(true)
        .minute_step(5)
        .second_step(2)
        .bounds(time("12:00"), time("14:00"))
        .validate(|t| {
            if t.minute() == 13 {
                Err(Error("Reserved minute".into()))
            } else {
                Ok(())
            }
        });
    assert!(picker.step(TimePart::Second, true));
    assert_eq!(value.get(), time("12:59:01.123456789"));
    assert!(picker.step(TimePart::Minute, true));
    assert_eq!(value.get(), time("13:04:01.123456789"));
    assert!(!picker.step(TimePart::Hour, true));
    picker.binding().set("13:08:01.123456789".into());
    assert!(!picker.step(TimePart::Minute, true));
    assert_eq!(picker.binding().get(), "13:08:01.123456789");
    assert_eq!(value.get(), time("13:04:01.123456789"));
    assert!(picker.step(TimePart::Minute, false));
    assert_eq!(value.get(), time("13:03:01.123456789"));
    for (text, status) in [
        ("", ValidationStatus::Incomplete),
        ("15:00", ValidationStatus::OutOfRange),
        ("12:13", ValidationStatus::Invalid),
        ("12:00:60", ValidationStatus::Invalid),
        ("12:00Z", ValidationStatus::Invalid),
    ] {
        picker.binding().set(text.into());
        assert_eq!(picker.validation().status, status);
        assert!(!picker.commit());
        assert!(!picker.step(TimePart::Hour, false));
    }
    let value = State::new(Time::MIN);
    let picker = TimePicker::new(value.binding()).show_seconds(true);
    assert!(!picker.step(TimePart::Second, false));
    value.set(Time::MAX);
    assert!(!picker.step(TimePart::Second, true));
    assert!(!picker.step(TimePart::Minute, true));
    assert_eq!(value.get(), Time::MAX);
}

#[test]
fn blur_submit_cancel_readonly_and_stepper_keyboard_share_one_draft() {
    for blur in [false, true] {
        let value = State::new(time("12:30:45.123"));
        let submits = Rc::new(Cell::new(0));
        let cancels = Rc::new(Cell::new(0));
        let picker = TimePicker::new(value.binding())
            .commit_on_blur(blur)
            .minute_step(5)
            .on_submit({
                let s = submits.clone();
                move || s.set(s.get() + 1)
            })
            .on_cancel({
                let s = cancels.clone();
                move || s.set(s.get() + 1)
            });
        let host = Host::new({
            let p = picker.clone();
            move || p.view()
        });
        layout(&host);
        let id = field(&host);
        host.focus(id).unwrap();
        picker.binding().set("13:00".into());
        host.blur();
        assert_eq!(
            value.get(),
            time(if blur { "13:00:45.123" } else { "12:30:45.123" })
        );
        host.focus(id).unwrap();
        host.key(key(Key::Enter)).unwrap();
        assert_eq!(value.get(), time("13:00:45.123"));
        assert_eq!(submits.get(), 1);
        picker.binding().set("bad".into());
        host.key(key(Key::Enter)).unwrap();
        assert_eq!(submits.get(), 1);
        host.key(key(Key::Escape)).unwrap();
        assert_eq!(cancels.get(), 1);
        let step = host
            .snapshot()
            .unwrap()
            .into_iter()
            .find(|n| n.kind == Kind::Stepper && n.label == "Minute")
            .unwrap();
        host.focus(step.id).unwrap();
        assert!(host.key(key(Key::Up)).unwrap());
        assert_eq!(value.get(), time("13:05:45.123"));
        layout(&host);
        assert_eq!(field(&host), id);
        let readonly = picker.read_only(true);
        readonly.binding().set("14:00".into());
        assert_eq!(readonly.binding().get(), "13:05");
        assert!(!readonly.commit());
        assert!(!readonly.step(TimePart::Minute, true));
    }
}

#[test]
fn external_revisions_rejected_setters_and_callbacks_remain_authoritative() {
    let value = State::new(time("12:00"));
    let picker = TimePicker::new(value.binding());
    picker.binding().set("bad".into());
    value.set(time("13:00"));
    value.set(time("12:00"));
    assert_eq!(
        picker.binding().get(),
        "12:00",
        "ABA invalidates stale text"
    );
    value.set(time("13:20:22.123"));
    assert_eq!(picker.binding().get(), "13:20");
    assert!(picker.commit());
    assert_eq!(value.get(), time("13:20:22.123"));
    let read = value.clone();
    let reject = TimePicker::new(Binding::new(move || read.get(), |_| {}));
    reject.binding().set("14:00".into());
    assert!(!reject.commit());
    assert!(!reject.step(TimePart::Minute, true));
    assert_eq!(reject.binding().get(), "14:00");
    assert_eq!(value.get(), time("13:20:22.123"));
    let callback = TimePicker::new(value.binding())
        .on_validation(value.handler(|v, _: Validation| v.set(time("16:00"))));
    assert!(!callback.step(TimePart::Hour, true));
    assert_eq!(value.get(), time("16:00"));
    assert_eq!(callback.binding().get(), "16:00");
    let writes = State::new(0);
    let impure = TimePicker::new(value.binding()).validate({
        let w = writes.clone();
        move |_| {
            w.set(1);
            Ok(())
        }
    });
    assert!(
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| impure.validation())).is_err()
    );
    assert_eq!(writes.get(), 0);
}
