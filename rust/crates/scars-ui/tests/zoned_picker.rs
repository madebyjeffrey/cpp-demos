use scars_ui::*;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
fn ts(s: &str) -> Timestamp {
    s.parse().unwrap()
}
fn dt(s: &str) -> DateTime {
    s.parse().unwrap()
}

#[test]
fn gaps_always_reject_and_fold_policy_revalidates_without_discarding_text() {
    let source = State::new(ts("2024-01-01T12:00:00.123456789Z"));
    let p = ZonedPicker::new(source.binding(), "America/Edmonton")
        .unwrap()
        .show_seconds(true);
    for policy in [
        RepeatedTimePolicy::Reject,
        RepeatedTimePolicy::Earlier,
        RepeatedTimePolicy::Later,
    ] {
        p.set_repeated_time(policy);
        p.binding().set("2024-03-10T02:30:00.123456789".into());
        assert!(!p.commit());
        assert_eq!(p.validation().status, ValidationStatus::Invalid);
        assert_eq!(p.binding().get(), "2024-03-10T02:30:00.123456789");
        assert_eq!(source.get(), ts("2024-01-01T12:00:00.123456789Z"));
    }
    p.set_repeated_time(RepeatedTimePolicy::Reject);
    p.binding().set("2024-11-03T01:30:00.123456789".into());
    assert!(!p.commit());
    p.set_repeated_time(RepeatedTimePolicy::Earlier);
    assert_eq!(p.binding().get(), "2024-11-03T01:30:00.123456789");
    assert!(p.commit());
    assert_eq!(source.get(), ts("2024-11-03T07:30:00.123456789Z"));
    p.choose_occurrence(RepeatedTimePolicy::Later).unwrap();
    assert_eq!(source.get(), ts("2024-11-03T08:30:00.123456789Z"));
    p.set_repeated_time(RepeatedTimePolicy::Reject);
    assert!(p.commit(), "existing fold retains its known instant");
    assert_eq!(source.get(), ts("2024-11-03T08:30:00.123456789Z"));
}

#[test]
fn zone_changes_explicitly_preserve_instant_or_wall_time_and_hidden_precision() {
    let source = State::new(ts("2024-06-01T12:34:56.123456789Z"));
    let p = ZonedPicker::new(source.binding(), "UTC").unwrap();
    p.change_zone("Asia/Tokyo", ZoneChange::KeepInstant)
        .unwrap();
    assert_eq!(source.get(), ts("2024-06-01T12:34:56.123456789Z"));
    assert_eq!(p.binding().get(), "2024-06-01T21:34");
    p.change_zone("Europe/Paris", ZoneChange::KeepLocal)
        .unwrap();
    assert_eq!(source.get(), ts("2024-06-01T19:34:56.123456789Z"));
    assert_eq!(p.binding().get(), "2024-06-01T21:34");
    // A valid uncommitted candidate participates in the explicit zone operation.
    p.binding().set("2024-06-02T01:15".into());
    p.change_zone("+05:30", ZoneChange::KeepInstant).unwrap();
    assert_eq!(source.get(), ts("2024-06-01T23:15:56.123456789Z"));
    assert_eq!(p.binding().get(), "2024-06-02T04:45");
    // Equal wall time after a zone-only change still invalidates a stale draft.
    source.set(ts("2024-01-01T12:00:00Z"));
    p.change_zone("UTC", ZoneChange::KeepInstant).unwrap();
    p.change_zone("Europe/London", ZoneChange::KeepInstant)
        .unwrap();
    assert_eq!(p.time_zone().iana_name(), Some("Europe/London"));
    assert_eq!(p.binding().get(), "2024-01-01T12:00");
}

#[test]
fn instant_civil_and_custom_limits_guard_text_calendar_steps_and_zone_changes() {
    let source = State::new(ts("2024-06-01T12:00:00.000000001Z"));
    let p = ZonedPicker::new(source.binding(), "UTC")
        .unwrap()
        .show_seconds(true)
        .bounds(
            ts("2024-06-01T12:00:00.000000001Z"),
            ts("2024-06-01T14:00:00.000000001Z"),
        )
        .civil_bounds(dt("2024-06-01T11:00"), dt("2024-06-01T15:00"))
        .validate(|z| {
            if z.time().minute() == 13 {
                Err(Error("Reserved".into()))
            } else {
                Ok(())
            }
        });
    p.binding().set("2024-06-01T12:00:00".into());
    assert_eq!(p.validation().status, ValidationStatus::OutOfRange);
    assert!(!p.commit());
    p.cancel();
    assert!(!p.step(TimePart::Second, false));
    assert!(p.step(TimePart::Hour, true));
    assert_eq!(source.get(), ts("2024-06-01T13:00:00.000000001Z"));
    p.binding().set("2024-06-01T13:13:00.000000001".into());
    assert_eq!(p.validation().status, ValidationStatus::Invalid);
    assert!(!p.commit());
    p.cancel();
    assert!(!p.select("2024-06-02".parse().unwrap()));
    assert!(
        p.change_zone("Asia/Tokyo", ZoneChange::KeepInstant)
            .is_err(),
        "civil range checked after conversion"
    );
    assert!(
        p.change_zone("+02:00", ZoneChange::KeepLocal).is_err(),
        "instant range checked after resolution"
    );
    assert_eq!(p.time_zone().iana_name(), Some("UTC"));
    assert_eq!(source.get(), ts("2024-06-01T13:00:00.000000001Z"));
}

#[test]
fn failed_zone_changes_preserve_draft_instant_and_zone_including_dst_failures() {
    let source = State::new(ts("2024-03-10T02:30:12.123Z"));
    let p = ZonedPicker::new(source.binding(), "UTC")
        .unwrap()
        .show_seconds(true);
    let before = p.binding().get();
    for target in ["Invalid/Zone", "America/Edmonton"] {
        assert!(p.change_zone(target, ZoneChange::KeepLocal).is_err());
        assert_eq!(source.get(), ts("2024-03-10T02:30:12.123Z"));
        assert_eq!(p.binding().get(), before);
        assert_eq!(p.time_zone().iana_name(), Some("UTC"));
    }
    p.binding().set("unfinished".into());
    assert!(
        p.change_zone("Asia/Tokyo", ZoneChange::KeepInstant)
            .is_err()
    );
    assert!(p.choose_occurrence(RepeatedTimePolicy::Earlier).is_err());
    assert_eq!(p.binding().get(), "unfinished");
    assert_eq!(source.get(), ts("2024-03-10T02:30:12.123Z"));
}

#[test]
fn wall_clock_steps_resolve_dst_instead_of_inventing_elapsed_time() {
    let source = State::new(ts("2024-03-10T08:30:45.123Z")); // Edmonton 01:30
    let p = ZonedPicker::new(source.binding(), "America/Edmonton").unwrap();
    assert!(!p.step(TimePart::Hour, true), "02:30 is a gap");
    source.set(ts("2024-11-03T06:30:45.123Z")); // 00:30
    assert!(!p.step(TimePart::Hour, true), "01:30 is repeated");
    p.set_repeated_time(RepeatedTimePolicy::Later);
    assert!(p.step(TimePart::Hour, true));
    assert_eq!(source.get(), ts("2024-11-03T08:30:45.123Z"));
    assert_eq!(p.binding().get(), "2024-11-03T01:30");
    // Lord Howe has a 30-minute overlap, not an assumed one-hour DST shift.
    source.set(ts("2024-04-06T14:45:00Z"));
    p.change_zone("Australia/Lord_Howe", ZoneChange::KeepInstant)
        .unwrap();
    p.choose_occurrence(RepeatedTimePolicy::Later).unwrap();
    assert_eq!(source.get(), ts("2024-04-06T15:15:00Z"));
}

#[test]
fn readonly_external_revisions_localized_grammar_and_extremes_remain_exact() {
    let source = State::new(ts("2024-02-29T13:02:03.123456789Z"));
    let p = ZonedPicker::new(source.binding(), "UTC")
        .unwrap()
        .hour_cycle(HourCycle::H12)
        .show_seconds(true)
        .locale(EditLocale::new("de-DE").unwrap());
    assert_eq!(p.binding().get(), "29.02.2024T01:02:03,123456789 PM");
    p.binding().set("bad".into());
    source.set(ts("2024-03-01T13:02:03.123456789Z"));
    source.set(ts("2024-02-29T13:02:03.123456789Z"));
    assert_eq!(p.binding().get(), "29.02.2024T01:02:03,123456789 PM");
    let p = p.read_only(true);
    p.binding().set("01.03.2024T01:02 PM".into());
    assert!(!p.commit());
    assert!(!p.step(TimePart::Hour, true));
    assert!(!p.set_repeated_time(RepeatedTimePolicy::Later));
    assert!(
        p.change_zone("Asia/Tokyo", ZoneChange::KeepInstant)
            .is_err()
    );
    assert!(p.choose_occurrence(RepeatedTimePolicy::Later).is_err());
    let p = ZonedPicker::new(source.binding(), "UTC")
        .unwrap()
        .show_seconds(true);
    for extreme in [Timestamp::MIN, Timestamp::MAX] {
        source.set(extreme);
        assert!(p.commit());
        assert_eq!(source.get(), extreme);
    }
}

#[test]
fn rejected_setters_and_reentrant_zone_callbacks_cannot_be_overwritten() {
    let source = State::new(ts("2024-06-01T12:00:00Z"));
    let read = source.clone();
    let reject = ZonedPicker::new(Binding::new(move || read.get(), |_| {}), "UTC").unwrap();
    reject.binding().set("2024-06-01T13:00".into());
    assert!(!reject.commit());
    assert!(
        reject
            .change_zone("Asia/Tokyo", ZoneChange::KeepLocal)
            .is_err()
    );
    assert_eq!(reject.binding().get(), "2024-06-01T13:00");
    assert_eq!(reject.time_zone().iana_name(), Some("UTC"));
    let slot: Rc<RefCell<Option<ZonedPicker>>> = Rc::new(RefCell::new(None));
    let (read, write, slot_write) = (source.clone(), source.clone(), slot.clone());
    let p = ZonedPicker::new(
        Binding::new(
            move || read.get(),
            move |v| {
                write.set(v);
                let p = slot_write.borrow().clone().unwrap();
                p.change_zone("Europe/Paris", ZoneChange::KeepInstant)
                    .unwrap();
            },
        ),
        "UTC",
    )
    .unwrap();
    *slot.borrow_mut() = Some(p.clone());
    assert!(p.change_zone("Asia/Tokyo", ZoneChange::KeepLocal).is_err());
    assert_eq!(p.time_zone().iana_name(), Some("Europe/Paris"));
    assert_eq!(source.get(), ts("2024-06-01T03:00:00Z"));
    slot.borrow_mut().take(); // release the intentional test callback cycle
    let notifications = Rc::new(Cell::new(0));
    let p = ZonedPicker::new(source.binding(), "UTC")
        .unwrap()
        .on_validation({
            let n = notifications.clone();
            let s = source.clone();
            move |_| {
                n.set(n.get() + 1);
                s.set(ts("2024-06-02T12:00:00Z"));
            }
        });
    assert!(
        p.change_zone("Asia/Tokyo", ZoneChange::KeepInstant)
            .is_err()
    );
    assert_eq!(source.get(), ts("2024-06-02T12:00:00Z"));
    assert_eq!(notifications.get(), 1);
}

#[test]
fn validation_is_pure_and_checks_the_target_zone() {
    let source = State::new(ts("2024-06-01T12:00:00Z"));
    let p = ZonedPicker::new(source.binding(), "UTC")
        .unwrap()
        .validate(|z| {
            if z.time_zone().iana_name() == Some("Asia/Tokyo") {
                Err(Error("Unavailable zone".into()))
            } else {
                Ok(())
            }
        });
    assert!(
        p.change_zone("Asia/Tokyo", ZoneChange::KeepInstant)
            .is_err()
    );
    assert_eq!(p.time_zone().iana_name(), Some("UTC"));
    let writes = State::new(0);
    let p = ZonedPicker::new(source.binding(), "UTC")
        .unwrap()
        .validate({
            let w = writes.clone();
            move |_| {
                w.set(1);
                Ok(())
            }
        });
    assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| p.validation())).is_err());
    assert_eq!(writes.get(), 0);
}

#[test]
fn callback_switching_to_an_out_of_bounds_fold_is_not_reported_as_a_valid_step() {
    let source = State::new(ts("2024-11-03T07:00:00Z"));
    let p = ZonedPicker::new(source.binding(), "America/Edmonton")
        .unwrap()
        .minute_step(30)
        .repeated_time(RepeatedTimePolicy::Earlier)
        .bounds(ts("2024-11-03T07:00:00Z"), ts("2024-11-03T07:59:59Z"))
        .on_validation(source.handler(|s, _: Validation| s.set(ts("2024-11-03T08:30:00Z"))));
    assert!(!p.step(TimePart::Minute, true));
    assert_eq!(source.get(), ts("2024-11-03T08:30:00Z"));
    assert_eq!(p.validation().status, ValidationStatus::OutOfRange);
}
