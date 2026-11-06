use scars_ui::*;
use std::{cell::Cell, rc::Rc};
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
fn date(s: &str) -> Date {
    s.parse().unwrap()
}
fn key(key: Key) -> KeyEvent {
    KeyEvent {
        key,
        down: true,
        repeat: false,
        modifiers: Modifiers::default(),
    }
}
fn layout(h: &Host) {
    h.layout(Size::new(480., 640.), &Metrics).unwrap();
}
fn day(h: &Host, d: Date) -> Node {
    h.snapshot()
        .unwrap()
        .into_iter()
        .find(|n| n.calendar_day.is_some_and(|i| i.date == d))
        .unwrap()
}
fn field(h: &Host) -> Node {
    h.snapshot()
        .unwrap()
        .into_iter()
        .find(|n| n.kind == Kind::TextField)
        .unwrap()
}
#[test]
fn rules_apply_to_drafts_pointer_choices_and_invalid_external_models() {
    let value = State::new(date("2024-02-29"));
    let picker = DatePicker::new(value.binding())
        .bounds(date("2024-02-01"), date("2024-03-31"))
        .disabled_dates(|d| d.day() == 10)
        .validate(|d| {
            if d.day() == 20 {
                Err(Error("Reserved".into()))
            } else {
                Ok(())
            }
        });
    for (text, status) in [
        ("", ValidationStatus::Incomplete),
        ("2024-01-31", ValidationStatus::OutOfRange),
        ("2024-02-10", ValidationStatus::Invalid),
        ("2024-02-20", ValidationStatus::Invalid),
        ("2025-02-29", ValidationStatus::Invalid),
    ] {
        picker.draft.binding().set(text.into());
        assert_eq!(picker.validation().status, status);
        assert!(!picker.commit());
        assert_eq!(value.get(), date("2024-02-29"));
    }
    assert!(!picker.select(date("2024-02-10")));
    assert!(!picker.select(date("2024-02-20")));
    assert!(!picker.select(date("2024-04-01")));
    assert!(picker.select(date("2024-03-15")));
    assert_eq!(value.get(), date("2024-03-15"));
    value.set(date("2024-04-01"));
    assert_eq!(picker.validation().status, ValidationStatus::OutOfRange);
    assert!(!picker.commit());
    assert_eq!(value.get(), date("2024-04-01"));
    let h = Host::new(move || picker.view());
    layout(&h);
    assert!(!day(&h, date("2024-03-10")).enabled);
}
#[test]
fn cursor_moves_without_committing_crosses_leaps_and_navigates_disabled_days() {
    let value = State::new(date("2024-01-31"));
    let picker = DatePicker::new(value.binding()).disabled_dates(|d| d.day() == 29);
    let h = Host::new({
        let p = picker.clone();
        move || p.view()
    });
    layout(&h);
    h.focus(day(&h, date("2024-01-31")).id).unwrap();
    assert!(h.key(key(Key::PageDown)).unwrap());
    layout(&h);
    assert_eq!(picker.calendar().cursor, date("2024-02-29"));
    assert_eq!(value.get(), date("2024-01-31"));
    let disabled = day(&h, date("2024-02-29"));
    assert!(!disabled.enabled);
    assert!(disabled.focused);
    assert!(!h.activate(disabled.id).unwrap());
    assert!(h.key(key(Key::Right)).unwrap());
    layout(&h);
    assert_eq!(picker.calendar().cursor, date("2024-03-01"));
    assert!(h.key(key(Key::Enter)).unwrap());
    assert_eq!(value.get(), date("2024-03-01"));
    h.key(key(Key::Down)).unwrap();
    assert_eq!(picker.calendar().cursor, date("2024-03-08"));
    assert_eq!(value.get(), date("2024-03-01"));
    h.key(key(Key::Home)).unwrap();
    assert_eq!(picker.calendar().cursor, date("2024-03-04"));
    h.key(key(Key::End)).unwrap();
    assert_eq!(picker.calendar().cursor, date("2024-03-10"));
    h.key(KeyEvent {
        modifiers: Modifiers {
            control: true,
            ..Default::default()
        },
        ..key(Key::End)
    })
    .unwrap();
    assert_eq!(picker.calendar().cursor, date("2024-03-31"));
    h.key(key(Key::Escape)).unwrap();
    assert_eq!(picker.calendar().cursor, value.get());
}
#[test]
fn blur_submit_cancel_readonly_and_tab_stop_policies_share_the_draft() {
    for auto in [false, true] {
        let value = State::new(date("2024-02-01"));
        let submits = Rc::new(Cell::new(0));
        let cancels = Rc::new(Cell::new(0));
        let picker = DatePicker::new(value.binding())
            .commit_on_blur(auto)
            .on_submit({
                let s = submits.clone();
                move || s.set(s.get() + 1)
            })
            .on_cancel({
                let s = cancels.clone();
                move || s.set(s.get() + 1)
            });
        let h = Host::new({
            let p = picker.clone();
            move || View::column(vec![p.view(), View::button("After", || {})])
        });
        layout(&h);
        let f = field(&h);
        h.focus(f.id).unwrap();
        picker.draft.binding().set("2024-02-02".into());
        h.blur();
        assert_eq!(
            value.get(),
            date(if auto { "2024-02-02" } else { "2024-02-01" })
        );
        assert_eq!(submits.get(), 0);
        h.focus(f.id).unwrap();
        h.key(key(Key::Enter)).unwrap();
        assert_eq!(value.get(), date("2024-02-02"));
        assert_eq!(submits.get(), 1);
        picker.draft.binding().set("bad".into());
        h.key(key(Key::Enter)).unwrap();
        assert_eq!(submits.get(), 1);
        h.key(key(Key::Escape)).unwrap();
        assert_eq!(cancels.get(), 1);
        assert!(!picker.validation().dirty);
        h.blur();
        let mut stops = vec![];
        for _ in 0..7 {
            h.focus_next(false).unwrap();
            stops.push(h.focused().unwrap());
        }
        let nodes = h.snapshot().unwrap();
        assert_eq!(
            stops
                .iter()
                .filter(|id| nodes
                    .iter()
                    .find(|n| n.id == **id)
                    .unwrap()
                    .calendar_day
                    .is_some())
                .count(),
            1
        );
        let readonly = picker.read_only(true);
        let text = readonly.draft.binding().get();
        readonly.draft.binding().set("2024-03-04".into());
        assert_eq!(readonly.draft.binding().get(), text);
        assert!(!readonly.commit());
        assert!(!readonly.select(date("2024-03-04")));
    }
}
#[test]
fn external_changes_restore_calendar_month_and_disabled_ancestor_blocks_navigation() {
    let value = State::new(date("2024-02-29"));
    let enabled = State::new(true);
    let picker = DatePicker::new(value.binding()).locale(EditLocale::new("ar-EG").unwrap());
    let h = Host::new({
        let p = picker.clone();
        let enabled = enabled.clone();
        move || p.view().enabled(enabled.get())
    });
    layout(&h);
    h.focus(day(&h, date("2024-02-29")).id).unwrap();
    h.key(key(Key::Left)).unwrap();
    assert_eq!(picker.calendar().cursor, date("2024-03-01"));
    let field_id = field(&h).id;
    value.set(date("2026-12-15"));
    layout(&h);
    assert_eq!(
        field(&h).id,
        field_id,
        "calendar page changes keep the text editor identity"
    );
    assert_eq!(picker.calendar().month, date("2026-12-01"));
    let d = day(&h, date("2026-12-15"));
    assert!(
        d.focused,
        "external selection restores focused calendar cursor"
    );
    picker.draft.binding().set("bad draft".into());
    value.set(date("2026-03-04"));
    value.set(date("2026-12-15"));
    assert!(
        !picker.validation().dirty,
        "state revisions prevent stale draft resurrection after ABA"
    );
    assert_eq!(picker.calendar().cursor, date("2026-12-15"));
    layout(&h);
    assert_eq!(field(&h).id, field_id);
    assert!(h.focus(d.id).unwrap());
    enabled.set(false);
    layout(&h);
    assert!(!h.focus(d.id).unwrap());
    assert!(!h.activate(d.id).unwrap());
    assert!(!h.key(key(Key::Right)).unwrap());
    assert_eq!(value.get(), date("2026-12-15"));
}
#[test]
fn validation_is_pure_and_callbacks_can_publish_authoritative_values_without_recursing() {
    let value = State::new(date("2024-02-01"));
    let calls = Rc::new(Cell::new(0));
    let picker = DatePicker::new(value.binding()).on_validation({
        let value = value.clone();
        let calls = calls.clone();
        move |_| {
            calls.set(calls.get() + 1);
            value.set(date("2024-03-04"));
        }
    });
    assert!(!picker.select(date("2024-02-02")));
    assert_eq!(value.get(), date("2024-03-04"));
    assert_eq!(calls.get(), 1);
    let flag = State::new(false);
    let bad = DatePicker::new(value.binding()).validate(move |_| {
        flag.set(true);
        Ok(())
    });
    assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| bad.validation())).is_err());
}

#[test]
fn date_extremes_readonly_navigation_and_rejected_binding_commits_are_safe() {
    for initial in [Date::MIN, Date::MAX] {
        let value = State::new(initial);
        let picker = DatePicker::new(value.binding()).read_only(true);
        let host = Host::new({
            let p = picker.clone();
            move || p.view()
        });
        layout(&host);
        assert!(host.focus(day(&host, initial).id).unwrap());
        for k in [
            Key::Left,
            Key::Right,
            Key::PageUp,
            Key::PageDown,
            Key::Home,
            Key::End,
        ] {
            host.key(key(k)).unwrap();
            layout(&host);
            assert_eq!(value.get(), initial);
        }
    }
    let source = State::new(date("2024-02-01"));
    let read = source.clone();
    let reject = Binding::new(move || read.get(), |_| {});
    let picker = DatePicker::new(reject);
    assert!(!picker.select(date("2024-02-02")));
    assert_eq!(source.get(), date("2024-02-01"));
}

#[test]
fn calendar_focus_reveals_after_reflow_and_respects_later_manual_scrolling() {
    let value = State::new(date("2024-01-31"));
    let picker = DatePicker::new(value.binding());
    let host = Host::new(move || {
        View::scroll(vec![View::spacer().height(300).grow(0.), picker.view()]).key("scroll")
    });
    for _ in 0..2 {
        host.layout(Size::new(320., 180.), &Metrics).unwrap();
    }
    host.focus(day(&host, date("2024-01-31")).id).unwrap();
    for _ in 0..2 {
        host.layout(Size::new(320., 180.), &Metrics).unwrap();
    }
    host.key(key(Key::Right)).unwrap();
    for _ in 0..3 {
        host.layout(Size::new(320., 180.), &Metrics).unwrap();
    }
    let current = day(&host, date("2024-02-01"));
    assert!(current.focused);
    assert!(current.clip.height > 0.);
    let scroll = host
        .snapshot()
        .unwrap()
        .into_iter()
        .find(|n| n.key.as_deref() == Some("scroll"))
        .unwrap()
        .id;
    host.scroll_to(scroll, Point::default()).unwrap();
    host.layout(Size::new(320., 180.), &Metrics).unwrap();
    assert_eq!(
        host.scroll_info(scroll).unwrap().unwrap().offset.y,
        0.,
        "idle layout must not snap back to the focused day"
    );
}

#[test]
fn validation_callbacks_cannot_commit_a_retired_or_replaced_calendar_action() {
    for replace in [false, true] {
        let value = State::new(date("2024-02-01"));
        let changed = State::new(false);
        let original = DatePicker::new(value.binding())
            .on_validation(changed.handler(|s, _: Validation| s.set(true)));
        let replacement = DatePicker::new(value.binding());
        let host = Host::new(move || {
            if !changed.get() {
                original.view()
            } else if replace {
                replacement.view()
            } else {
                View::label("Removed")
            }
        });
        layout(&host);
        let target = day(&host, date("2024-02-02")).id;
        assert!(!host.activate(target).unwrap());
        assert_eq!(value.get(), date("2024-02-01"));
    }
    let source = State::new(date("2024-02-01"));
    let revision = Rc::new(Cell::new(0u64));
    let read = source.clone();
    let write = source.clone();
    let rev = revision.clone();
    let binding =
        Binding::new(move || read.get(), move |v| write.set(v)).with_revision(move || rev.get());
    let picker = DatePicker::new(binding);
    picker.draft.binding().set("bad".into());
    source.set(date("2024-03-04"));
    revision.set(1);
    source.set(date("2024-02-01"));
    revision.set(2);
    assert_eq!(picker.draft.binding().get(), "2024-02-01");
    assert!(!picker.validation().dirty);
}
