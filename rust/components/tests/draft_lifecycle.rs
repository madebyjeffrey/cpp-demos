use scars_ui::*;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

#[test]
fn rejected_setters_report_feedback_retain_text_and_allow_explicit_retry() {
    let source = State::new(1i64);
    let allow = Rc::new(Cell::new(false));
    let (read, write, allowed) = (source.clone(), source.clone(), allow.clone());
    let draft = Draft::new(
        Binding::new(
            move || read.get(),
            move |v| {
                if allowed.get() {
                    write.set(v)
                }
            },
        ),
        |s| s.parse::<i64>().map_err(|e| e.to_string()),
        |v| v.to_string(),
    );
    draft.binding().set("2".into());
    assert!(!draft.commit());
    let validation = draft.validation();
    assert_eq!(validation.status, ValidationStatus::Invalid);
    assert!(validation.dirty);
    assert!(validation.message.is_some());
    assert_eq!(draft.binding().get(), "2");
    assert_eq!(source.get(), 1);
    allow.set(true);
    assert!(draft.commit());
    assert_eq!(source.get(), 2);
    assert!(!draft.validation().dirty);
    assert_eq!(draft.validation().status, ValidationStatus::Valid);
}

#[test]
fn reentrant_setter_keeps_a_newer_draft_instead_of_resetting_it() {
    let source = State::new(1i64);
    let slot: Rc<RefCell<Option<Binding<String>>>> = Rc::new(RefCell::new(None));
    let (read, write, other) = (source.clone(), source.clone(), slot.clone());
    let draft = Draft::new(
        Binding::new(
            move || read.get(),
            move |v| {
                write.set(v);
                other
                    .borrow()
                    .as_ref()
                    .unwrap()
                    .set("newer unfinished input".into());
            },
        ),
        |s| s.parse::<i64>().map_err(|e| e.to_string()),
        |v| v.to_string(),
    );
    *slot.borrow_mut() = Some(draft.binding());
    draft.binding().set("2".into());
    assert!(!draft.commit());
    assert_eq!(source.get(), 2);
    assert_eq!(draft.binding().get(), "newer unfinished input");
    slot.borrow_mut().take();
}

#[test]
fn callbacks_cannot_turn_invalid_or_superseded_results_into_success() {
    let source = State::new(1i64);
    let armed = Rc::new(Cell::new(false));
    let number = Numeric::new(source.binding(), 0, 10, 1).on_validation({
        let s = source.clone();
        let a = armed.clone();
        move |_| {
            if a.replace(false) {
                s.set(99)
            }
        }
    });
    armed.set(true);
    assert!(!number.commit());
    assert_eq!(number.validation().status, ValidationStatus::OutOfRange);
    source.set(1);
    armed.set(true);
    assert!(!number.step(true));
    assert_eq!(source.get(), 99);
    let source = State::new(Time::new(12, 0, 0, 0).unwrap());
    let armed = Rc::new(Cell::new(false));
    let time = TimePicker::new(source.binding())
        .bounds(Time::MIN, Time::new(14, 0, 0, 0).unwrap())
        .on_validation({
            let s = source.clone();
            let a = armed.clone();
            move |_| {
                if a.replace(false) {
                    s.set(Time::new(23, 0, 0, 0).unwrap())
                }
            }
        });
    armed.set(true);
    assert!(!time.step(TimePart::Hour, true));
    assert_eq!(time.validation().status, ValidationStatus::OutOfRange);
}

#[test]
fn non_reflexive_invalid_models_can_be_corrected_without_losing_typed_text() {
    let source = State::new(f64::NAN);
    let number = Numeric::new(source.binding(), 0., 10., 1.);
    assert_eq!(number.validation().status, ValidationStatus::Invalid);
    number.draft.binding().set("2.5".into());
    assert_eq!(number.draft.binding().get(), "2.5");
    assert!(number.commit());
    assert_eq!(source.get(), 2.5);
}

#[test]
fn codec_queries_are_pure_and_formatter_context_changes_reset_the_old_text() {
    let writes = State::new(0);
    let value = State::new(1i64);
    let draft = Draft::new(
        value.binding(),
        {
            let w = writes.clone();
            move |_| {
                w.set(1);
                Ok(2)
            }
        },
        |v| v.to_string(),
    );
    assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| draft.validation())).is_err());
    assert_eq!(writes.get(), 0);
    let writes = State::new(0);
    assert!(
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| Draft::new(
            value.binding(),
            |s| s.parse::<i64>().map_err(|e| e.to_string()),
            {
                let w = writes.clone();
                move |v| {
                    w.set(1);
                    v.to_string()
                }
            }
        )))
        .is_err()
    );
    assert_eq!(writes.get(), 0);
    let prefix = State::new("A".to_owned());
    let draft = Draft::new(value.binding(), |_| Ok(2), {
        let p = prefix.clone();
        move |v| format!("{}{v}", p.get())
    });
    draft.binding().set("unfinished A".into());
    prefix.set("B".into());
    assert_eq!(draft.binding().get(), "B1");
    assert!(!draft.validation().dirty);
}

#[test]
fn validation_callback_staging_a_newer_color_draft_keeps_it_and_supersedes_apply() {
    let source = State::new(Color::rgba(1, 2, 3, 4));
    let armed = Rc::new(Cell::new(false));
    let slot: Rc<RefCell<Option<Binding<String>>>> = Rc::new(RefCell::new(None));
    let picker = ColorPicker::new(source.binding()).on_validation({
        let a = armed.clone();
        let s = slot.clone();
        move |_| {
            if a.replace(false) {
                s.borrow().as_ref().unwrap().set("#FFEEDDCC".into());
            }
        }
    });
    *slot.borrow_mut() = Some(picker.binding());
    picker.binding().set("#12345678".into());
    armed.set(true);
    assert!(!picker.commit());
    assert_eq!(source.get(), Color::rgba(18, 52, 86, 120));
    assert_eq!(picker.binding().get(), "#FFEEDDCC");
    assert!(picker.validation().dirty);
    slot.borrow_mut().take();
}

#[test]
fn date_selection_does_not_commit_newer_text_staged_by_validation() {
    let source = State::new("2024-02-01".parse::<Date>().unwrap());
    let armed = Rc::new(Cell::new(false));
    let slot: Rc<RefCell<Option<Binding<String>>>> = Rc::new(RefCell::new(None));
    let picker = DatePicker::new(source.binding()).on_validation({
        let a = armed.clone();
        let s = slot.clone();
        move |_| {
            if a.replace(false) {
                s.borrow().as_ref().unwrap().set("2024-02-03".into());
            }
        }
    });
    *slot.borrow_mut() = Some(picker.draft.binding());
    armed.set(true);
    assert!(!picker.select("2024-02-02".parse().unwrap()));
    assert_eq!(source.get(), "2024-02-01".parse::<Date>().unwrap());
    assert_eq!(picker.draft.binding().get(), "2024-02-03");
    slot.borrow_mut().take();
}

#[test]
fn all_typed_controls_keep_rejections_retry_and_read_back_normalization() {
    macro_rules! check {
        ($initial:expr, $raw:expr, $normalized:expr, $build:expr) => {{
            let source = State::new($initial);
            let allow = Rc::new(Cell::new(false));
            let calls = Rc::new(Cell::new(0));
            let (read, write, accepted, writes) =
                (source.clone(), source.clone(), allow.clone(), calls.clone());
            let normalized = $normalized;
            let picker = ($build)(Binding::new(
                move || read.get(),
                move |_| {
                    writes.set(writes.get() + 1);
                    if accepted.get() {
                        write.set(normalized);
                    }
                },
            ));
            picker.binding().set($raw.into());
            assert!(!picker.commit());
            assert_eq!(picker.binding().get(), $raw);
            let error = picker.validation();
            assert_eq!(error.status, ValidationStatus::Invalid);
            assert!(error.dirty && error.message.is_some());
            allow.set(true);
            assert!(picker.commit());
            assert_eq!(source.get(), normalized);
            assert_eq!(picker.validation().status, ValidationStatus::Valid);
            assert!(!picker.validation().dirty);
            let count = calls.get();
            assert!(picker.commit());
            assert_eq!(
                calls.get(),
                count,
                "pristine Apply must not repeat the setter"
            );
        }};
    }
    check!(1i64, "2", 3, |b| Numeric::new(b, 0, 10, 1));
    check!(
        "2024-02-01".parse::<Date>().unwrap(),
        "2024-02-02",
        "2024-02-03".parse::<Date>().unwrap(),
        DatePicker::new
    );
    check!(
        Time::new(10, 0, 0, 0).unwrap(),
        "11:00",
        Time::new(12, 0, 0, 0).unwrap(),
        TimePicker::new
    );
    check!(
        "2024-02-01T10:00:00".parse::<DateTime>().unwrap(),
        "2024-02-02T11:00",
        "2024-02-03T12:00:00".parse::<DateTime>().unwrap(),
        DateTimePicker::new
    );
    check!(
        "2024-02-01T10:00:00Z".parse::<Timestamp>().unwrap(),
        "2024-02-02T11:00",
        "2024-02-03T12:00:00Z".parse::<Timestamp>().unwrap(),
        |b| ZonedPicker::new(b, "UTC").unwrap()
    );
    check!(
        Color::rgba(1, 2, 3, 4),
        "#12345678",
        Color::rgba(5, 6, 7, 8),
        ColorPicker::new
    );
}
