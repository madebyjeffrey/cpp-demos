use scars_ui::*;
use std::{cell::Cell, rc::Rc};
fn dt(s: &str) -> DateTime {
    s.parse().unwrap()
}
fn date(s: &str) -> Date {
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
fn layout(h: &Host) {
    h.layout(Size::new(500., 700.), &Metrics).unwrap();
}
fn key(key: Key) -> KeyEvent {
    KeyEvent {
        key,
        down: true,
        repeat: false,
        modifiers: Default::default(),
    }
}
fn field(h: &Host) -> NodeId {
    h.snapshot()
        .unwrap()
        .into_iter()
        .find(|n| n.kind == Kind::TextField)
        .unwrap()
        .id
}
fn day(h: &Host, date: Date) -> Node {
    h.snapshot()
        .unwrap()
        .into_iter()
        .find(|n| n.calendar_day.is_some_and(|d| d.date == date))
        .unwrap()
}

#[test]
fn joint_edits_cross_tight_overnight_bounds_with_one_valid_write() {
    let source = State::new(dt("2026-11-02T23:00"));
    let writes = Rc::new(Cell::new(0));
    let (read, write, count) = (source.clone(), source.clone(), writes.clone());
    let picker = DateTimePicker::new(Binding::new(
        move || read.get(),
        move |v| {
            count.set(count.get() + 1);
            write.set(v)
        },
    ))
    .bounds(dt("2026-11-02T22:00"), dt("2026-11-03T02:00"));
    picker.binding().set("2026-11-03T23:00".into());
    assert_eq!(picker.validation().status, ValidationStatus::OutOfRange);
    assert!(!picker.commit());
    assert_eq!(writes.get(), 0);
    picker.binding().set("2026-11-03T01:00".into());
    assert_eq!(picker.validation().status, ValidationStatus::Valid);
    assert_eq!(source.get(), dt("2026-11-02T23:00"));
    assert!(picker.commit());
    assert_eq!(writes.get(), 1);
    assert_eq!(source.get(), dt("2026-11-03T01:00"));
}

#[test]
fn exact_clock_steps_carry_across_days_leaps_years_and_respect_joint_rules() {
    let source = State::new(dt("2024-02-28T23:59:59.123456789"));
    let picker = DateTimePicker::new(source.binding())
        .show_seconds(true)
        .second_step(2)
        .bounds(dt("2024-02-28T23:00"), dt("2024-03-01T01:00"))
        .validate(|v| {
            if v.time().minute() == 13 {
                Err(Error("Reserved".into()))
            } else {
                Ok(())
            }
        });
    assert!(picker.step(TimePart::Second, true));
    assert_eq!(source.get(), dt("2024-02-29T00:00:01.123456789"));
    assert!(picker.step(TimePart::Hour, false));
    assert_eq!(source.get(), dt("2024-02-28T23:00:01.123456789"));
    assert!(!picker.step(TimePart::Minute, false));
    picker.binding().set("2024-02-29T00:12:01.123456789".into());
    assert!(!picker.step(TimePart::Minute, true));
    assert_eq!(picker.binding().get(), "2024-02-29T00:12:01.123456789");
    assert!(picker.step(TimePart::Minute, false));
    assert_eq!(source.get(), dt("2024-02-29T00:11:01.123456789"));
    let picker = DateTimePicker::new(source.binding());
    source.set(dt("2024-12-31T23:30:45.123"));
    assert!(picker.step(TimePart::Hour, true));
    assert_eq!(source.get(), dt("2025-01-01T00:30:45.123"));
    source.set(DateTime::MIN);
    assert!(!picker.step(TimePart::Hour, false));
    source.set(DateTime::MAX);
    assert!(!picker.step(TimePart::Minute, true));
    assert!(picker.commit());
    assert_eq!(source.get(), DateTime::MAX);
}

#[test]
fn calendar_and_clock_share_staged_candidate_and_preserve_the_field_identity() {
    let source = State::new(dt("2024-02-28T23:55:45.123456789"));
    let picker = DateTimePicker::new(source.binding())
        .commit_on_blur(false)
        .disabled_dates(|d| d.day() == 10);
    let host = Host::new({
        let p = picker.clone();
        move || p.view()
    });
    layout(&host);
    let id = field(&host);
    picker.binding().set("2024-03-01T00:05".into());
    layout(&host);
    assert_eq!(picker.calendar().selected, date("2024-03-01"));
    assert_eq!(field(&host), id);
    assert!(host.activate(day(&host, date("2024-03-02")).id).unwrap());
    assert_eq!(source.get(), dt("2024-03-02T00:05:45.123456789"));
    assert_eq!(picker.binding().get(), "2024-03-02T00:05");
    assert!(!day(&host, date("2024-03-10")).enabled);
    assert!(!picker.select(date("2024-03-10")));
    picker.binding().set("bad".into());
    layout(&host);
    assert!(!host.activate(day(&host, date("2024-03-03")).id).unwrap());
    assert!(!picker.step(TimePart::Minute, true));
    assert_eq!(picker.binding().get(), "bad");
    picker.cancel();
    host.focus(day(&host, date("2024-03-02")).id).unwrap();
    host.key(key(Key::PageDown)).unwrap();
    assert_eq!(picker.calendar().cursor, date("2024-04-02"));
    host.key(key(Key::Escape)).unwrap();
    layout(&host);
    assert_eq!(picker.calendar().cursor, date("2024-03-02"));
    assert_eq!(field(&host), id);
}

#[test]
fn locale_and_clock_grammar_are_checked_across_the_whole_value() {
    let source = State::new(dt("2024-02-29T13:02:03.123456789"));
    for (tag, text) in [
        ("de-DE", "29.02.2024T01:02:03,123456789 PM"),
        ("ar-EG", "٢٩/٠٢/٢٠٢٤T٠١:٠٢:٠٣٫١٢٣٤٥٦٧٨٩ م"),
    ] {
        let picker = DateTimePicker::new(source.binding())
            .hour_cycle(HourCycle::H12)
            .show_seconds(true)
            .locale(EditLocale::new(tag).unwrap());
        assert_eq!(picker.binding().get(), text);
        picker.binding().set(text.into());
        assert!(picker.commit());
        assert_eq!(source.get(), dt("2024-02-29T13:02:03.123456789"));
        if tag == "ar-EG" {
            picker
                .binding()
                .set("٢٩/٠٢/٢٠٢٤T01:02:03٫123456789 م".into());
            assert!(
                !picker.commit(),
                "mixed digits across date/time are rejected"
            );
        }
    }
    let picker = DateTimePicker::new(source.binding());
    for invalid in [
        "2024-02-29T13:02Z",
        "2024-02-29T13:02[UTC]",
        "2024-02-29 13:02",
        "2024-02-29T13:02:03",
    ] {
        picker.binding().set(invalid.into());
        assert!(!picker.commit(), "{invalid}");
    }
    picker.binding().set("2024-03-01T14:00".into());
    assert!(picker.commit());
    assert_eq!(source.get(), dt("2024-03-01T14:00:03.123456789"));
}

#[test]
fn blur_submit_cancel_readonly_and_external_revisions_use_the_group() {
    for blur in [true, false] {
        let source = State::new(dt("2024-02-29T12:00"));
        let submits = Rc::new(Cell::new(0));
        let cancels = Rc::new(Cell::new(0));
        let picker = DateTimePicker::new(source.binding())
            .commit_on_blur(blur)
            .on_submit({
                let c = submits.clone();
                move || c.set(c.get() + 1)
            })
            .on_cancel({
                let c = cancels.clone();
                move || c.set(c.get() + 1)
            });
        let host = Host::new({
            let p = picker.clone();
            move || p.view()
        });
        layout(&host);
        let id = field(&host);
        host.focus(id).unwrap();
        picker.binding().set("2024-03-01T13:00".into());
        host.blur();
        assert_eq!(
            source.get(),
            dt(if blur {
                "2024-03-01T13:00"
            } else {
                "2024-02-29T12:00"
            })
        );
        assert_eq!(submits.get(), 0);
        host.focus(id).unwrap();
        host.key(key(Key::Enter)).unwrap();
        assert_eq!(source.get(), dt("2024-03-01T13:00"));
        assert_eq!(submits.get(), 1);
        picker.binding().set("bad".into());
        host.key(key(Key::Enter)).unwrap();
        assert_eq!(submits.get(), 1);
        host.key(key(Key::Escape)).unwrap();
        assert_eq!(cancels.get(), 1);
        picker.binding().set("bad".into());
        source.set(dt("2024-03-02T13:00"));
        source.set(dt("2024-03-01T13:00"));
        assert_eq!(picker.binding().get(), "2024-03-01T13:00");
        let readonly = picker.read_only(true);
        readonly.binding().set("2024-04-01T14:00".into());
        assert!(!readonly.commit());
        assert!(!readonly.step(TimePart::Hour, true));
        assert!(!readonly.select(date("2024-03-02")));
        assert_eq!(readonly.binding().get(), "2024-03-01T13:00");
    }
}

#[test]
fn rejected_setters_and_reentrant_callbacks_cannot_publish_stale_candidates() {
    let source = State::new(dt("2024-02-29T12:00"));
    let read = source.clone();
    let reject = DateTimePicker::new(Binding::new(move || read.get(), |_| {}));
    reject.binding().set("2024-03-01T13:00".into());
    assert!(!reject.commit());
    assert!(!reject.select(date("2024-03-02")));
    assert!(!reject.step(TimePart::Minute, true));
    assert_eq!(reject.binding().get(), "2024-03-01T13:00");
    let callback = DateTimePicker::new(source.binding())
        .on_validation(source.handler(|s, _: Validation| s.set(dt("2024-04-15T16:30"))));
    assert!(!callback.step(TimePart::Hour, true));
    assert_eq!(source.get(), dt("2024-04-15T16:30"));
    // The calendar's projected writer carries the same validation callback.
    let host = Host::new({
        let p = callback.clone();
        move || p.view()
    });
    layout(&host);
    assert!(!host.activate(day(&host, date("2024-04-16")).id).unwrap());
    assert_eq!(source.get(), dt("2024-04-15T16:30"));
    let writes = State::new(0);
    let impure = DateTimePicker::new(source.binding()).validate({
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
