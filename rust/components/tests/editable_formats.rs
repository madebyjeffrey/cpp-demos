use scars_ui::*;

#[test]
fn ascii_numbers_preserve_exact_integers_and_reject_ambiguous_input() {
    let value = State::new(0i64);
    let number = Numeric::new(value.binding(), i64::MIN, i64::MAX, 1);
    for exact in [
        i64::MIN,
        -9_007_199_254_740_993,
        9_007_199_254_740_993,
        i64::MAX,
    ] {
        number.draft.binding().set(exact.to_string());
        assert!(number.commit());
        assert_eq!(value.get(), exact);
        assert_eq!(number.draft.binding().get(), exact.to_string());
    }
    for bad in [
        "1,234", "1.234", "١٢٣٤", "1 234", "1_234", " 12", "12 ", "1e3",
    ] {
        let before = value.get();
        number.draft.binding().set(bad.into());
        assert!(!number.commit(), "accepted {bad}");
        assert_eq!(value.get(), before);
    }
    number.draft.binding().set("9223372036854775808".into());
    assert_eq!(number.validation().status, ValidationStatus::OutOfRange);
    let wide = State::new(u64::MAX);
    let unsigned = Numeric::new(wide.binding(), 0, u64::MAX, 1);
    assert_eq!(unsigned.draft.binding().get(), u64::MAX.to_string());
    unsigned.draft.binding().set((u64::MAX - 1).to_string());
    assert!(unsigned.commit());
    unsigned.draft.binding().set(u64::MAX.to_string());
    assert!(unsigned.commit());
    assert_eq!(wide.get(), u64::MAX);
    let float = State::new(0.);
    let decimal = Numeric::new(float.binding(), -1e10, 1e10, 0.5);
    for (text, expected) in [("-12.5e2", -1250.), (".5", 0.5), ("+12.", 12.)] {
        decimal.draft.binding().set(text.into());
        assert!(decimal.commit());
        assert_eq!(float.get(), expected);
    }
    for bad in ["1,5", "١٫٥", "NaN", "inf", "1.234,56", "1,234.56"] {
        decimal.draft.binding().set(bad.into());
        assert!(!decimal.commit(), "accepted {bad}");
    }
}

#[test]
fn date_editor_consumes_only_its_date_and_round_trips_extreme_years() {
    let value = State::new(Date::new(2024, 2, 29).unwrap());
    let picker = DatePicker::new(value.binding());
    for bad in [
        "03/04/2026",
        "2026-03-04T12:00:00",
        "2026-03-04T12:00:00Z",
        "2026-03-04[UTC]",
        "2026-03-04 ",
        "2025-02-29",
    ] {
        picker.draft.binding().set(bad.into());
        assert!(!picker.draft.commit(), "accepted {bad}");
        assert_eq!(value.get(), Date::new(2024, 2, 29).unwrap());
    }
    for date in [Date::MIN, Date::new(0, 1, 1).unwrap(), Date::MAX] {
        picker.draft.binding().set(date.to_string());
        assert!(picker.draft.commit(), "{}", date);
        assert_eq!(value.get(), date);
    }
}

#[test]
fn civil_time_and_datetime_never_discard_offsets_or_date_parts() {
    let time = State::new(Time::MIN);
    let draft = time_draft(time.binding());
    for valid in ["14:30", "23:59:59.123456789", "00:00:00.000"] {
        draft.binding().set(valid.into());
        assert!(draft.commit(), "accepted grammar: {valid}");
        assert_eq!(time.get(), valid.parse::<Time>().unwrap());
        let again = time_draft(time.binding());
        again.binding().set(again.binding().get());
        assert!(again.commit());
    }
    let before = time.get();
    for bad in [
        "2026-03-04T14:30:00",
        "14:30Z",
        "14:30:00+01:00",
        "14:30[UTC]",
        "14:30.5",
        "14:30:00.1234567890",
        "2:30",
        "24:00",
        "23:59:60",
    ] {
        draft.binding().set(bad.into());
        assert!(!draft.commit(), "accepted {bad}");
        assert_eq!(time.get(), before);
    }
    let initial = "2026-03-04T14:30:00".parse::<DateTime>().unwrap();
    let local = State::new(initial);
    let draft = datetime_draft(local.binding());
    for bad in [
        "2026-03-04T14:30:00+01:00",
        "2026-03-04T14:30:00Z",
        "2026-03-04T14:30:00[UTC]",
        "03/04/2026 14:30",
    ] {
        draft.binding().set(bad.into());
        assert!(!draft.commit(), "accepted {bad}");
        assert_eq!(local.get(), initial);
    }
    for exact in [DateTime::MIN, initial, DateTime::MAX] {
        draft.binding().set(exact.to_string());
        assert!(draft.commit());
        assert_eq!(local.get(), exact);
    }
}

#[test]
fn zoned_editor_requires_an_explicit_zone_when_an_offset_is_supplied() {
    let initial = resolve_zoned(
        "2026-03-04T14:30:00".parse().unwrap(),
        "America/Edmonton",
        Resolution::Reject,
    )
    .unwrap();
    let value = State::new(initial.clone());
    let picker = ZonedPicker::new(value.binding());
    for bad in [
        "2026-03-04T14:30:00Z",
        "2026-03-04T14:30:00-07:00",
        "2026-03-04T14:30:60-07:00[America/Edmonton]",
        "2026-03-04T14:30:00-07:00[America/Edmonton][u-ca=gregory]",
        "2026-03-04T14:30:00+00:00[America/Edmonton]",
    ] {
        picker.draft.binding().set(bad.into());
        assert!(!picker.draft.commit());
        assert_eq!(value.get(), initial);
    }
    picker.draft.binding().set("2026-03-04T15:30".into());
    assert!(picker.draft.commit());
    assert_eq!(value.get().time_zone(), initial.time_zone());
    picker.draft.binding().set(initial.to_string());
    assert!(picker.draft.commit());
    assert_eq!(value.get(), initial);
    let fixed = "2026-03-04T14:30:00+05:30[+05:30]"
        .parse::<Zoned>()
        .unwrap();
    let value = State::new(fixed.clone());
    let picker = ZonedPicker::new(value.binding());
    picker.draft.binding().set(fixed.to_string());
    assert!(picker.draft.commit());
    assert_eq!(value.get(), fixed);
}
