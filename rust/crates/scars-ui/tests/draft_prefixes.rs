use scars_ui::*;

#[test]
fn date_prefixes_are_appendable_calendar_values_in_each_supported_order() {
    for (locale, incomplete, invalid) in [
        (
            EditLocale::ASCII,
            vec!["2", "2024-", "2024-02-", "2024-02-2", "-00000"],
            vec![
                "2024-13",
                "2024-02-3",
                "2023-02-29",
                "+002024",
                "2024-2-",
                "2024-02-29T",
            ],
        ),
        (
            EditLocale::new("en-US").unwrap(),
            vec!["0", "02/", "02/29/", "02/29/202"],
            vec!["02/30/", "13/", "02/29/2023", "2/", "02/29/-"],
        ),
        (
            EditLocale::new("de-DE").unwrap(),
            vec!["2", "29.", "29.02.", "29.02.202"],
            vec!["30.02.", "32.", "29.02.2023", "29/02/"],
        ),
        (
            EditLocale::new("ar-EG").unwrap(),
            vec!["٢", "٢٩/", "٢٩/٠٢/", "٢٩/٠٢/٢٠٢"],
            vec!["٣٠/٠٢/", "٢9/", "٢٩/٠٢/٢٠٢٣", "٢٩/٠٢/000"],
        ),
    ] {
        let source = State::new("2024-01-01".parse::<Date>().unwrap());
        let picker = DatePicker::new(source.binding()).locale(locale);
        for (texts, expected) in [
            (incomplete, ValidationStatus::Incomplete),
            (invalid, ValidationStatus::Invalid),
        ] {
            for text in texts {
                picker.binding().set(text.into());
                assert_eq!(picker.validation().status, expected, "{text:?}");
                assert!(picker.validation().dirty);
                assert!(!picker.commit());
                assert_eq!(source.get(), "2024-01-01".parse::<Date>().unwrap());
                assert_eq!(picker.binding().get(), text);
            }
        }
    }
}

#[test]
fn clock_prefixes_obey_ranges_optional_seconds_and_period_placement() {
    for (cycle, seconds, incomplete, invalid) in [
        (
            HourCycle::H24,
            false,
            vec!["2", "23:", "23:5"],
            vec!["24", "23:6", "2:", "23:00:", "23:00Z"],
        ),
        (
            HourCycle::H24,
            true,
            vec!["23:00:", "23:00:5", "23:00:59."],
            vec!["23:00:6", "23:00:60", "23:00:59.1234567890", "23:00."],
        ),
        (
            HourCycle::H12,
            true,
            vec!["0", "1:", "1:00", "1:00 ", "1:00 a", "12:00:59.0 P"],
            vec!["00", "13", "1:0 A", "1:00 X", "12:00:59. PM", "1:00 AM "],
        ),
    ] {
        let source = State::new("12:30".parse::<Time>().unwrap());
        let picker = TimePicker::new(source.binding())
            .hour_cycle(cycle)
            .show_seconds(seconds);
        for (texts, expected) in [
            (incomplete, ValidationStatus::Incomplete),
            (invalid, ValidationStatus::Invalid),
        ] {
            for text in texts {
                picker.binding().set(text.into());
                assert_eq!(
                    picker.validation().status,
                    expected,
                    "{cycle:?}/{seconds}: {text:?}"
                );
                assert!(!picker.commit());
                assert_eq!(picker.binding().get(), text);
                assert_eq!(source.get(), "12:30".parse::<Time>().unwrap());
            }
        }
    }
    let picker = TimePicker::new(State::new(Time::MIN).binding())
        .locale(EditLocale::new("ar-EG").unwrap())
        .hour_cycle(HourCycle::H12)
        .show_seconds(true);
    for text in ["١:", "١:٠٠ ", "١:٠٠:٥٩٫"] {
        picker.binding().set(text.into());
        assert_eq!(
            picker.validation().status,
            ValidationStatus::Incomplete,
            "{text}"
        );
    }
    for text in ["١:0", "١:٠٠:٥٩.", "١:٠٠ A"] {
        picker.binding().set(text.into());
        assert_eq!(
            picker.validation().status,
            ValidationStatus::Invalid,
            "{text}"
        );
    }
}

#[test]
fn civil_and_zoned_editors_share_prefix_rules_without_accepting_offsets_or_bad_dates() {
    let source = State::new("2024-01-01T12:30".parse::<DateTime>().unwrap());
    let civil = DateTimePicker::new(source.binding());
    let zoned = ZonedPicker::new(
        State::new("2024-01-01T12:30Z".parse::<Timestamp>().unwrap()).binding(),
        "UTC",
    )
    .unwrap();
    for text in ["2024-02-", "2024-02-29", "2024-02-29T", "2024-02-29T23:5"] {
        civil.binding().set(text.into());
        zoned.binding().set(text.into());
        assert_eq!(
            civil.validation().status,
            ValidationStatus::Incomplete,
            "{text}"
        );
        assert_eq!(
            zoned.validation().status,
            ValidationStatus::Incomplete,
            "{text}"
        );
        assert!(!civil.commit() && !zoned.commit());
    }
    for text in [
        "2023-02-29T",
        "2024-02-T",
        "2024-02-29T24",
        "2024-02-29T23:00Z",
    ] {
        civil.binding().set(text.into());
        zoned.binding().set(text.into());
        assert_eq!(
            civil.validation().status,
            ValidationStatus::Invalid,
            "{text}"
        );
        assert_eq!(
            zoned.validation().status,
            ValidationStatus::Invalid,
            "{text}"
        );
    }
}

#[test]
fn grouping_and_hex_prefixes_stay_uncommitted_and_impossible_suffixes_are_invalid() {
    for (tag, incomplete, invalid) in [
        ("en-US", "1,23", "1,23.4"),
        ("de-DE", "1.23", "1.23,4"),
        ("fr-FR", "1\u{202f}23", "1\u{202f}23,4"),
        ("ar-EG", "١٬٢٣", "١٬٢٣٫٤"),
    ] {
        let value = State::new(10.);
        let number =
            Numeric::new(value.binding(), 0., 10000., 1.).locale(EditLocale::new(tag).unwrap());
        number.binding().set(incomplete.into());
        assert_eq!(
            number.validation().status,
            ValidationStatus::Incomplete,
            "{tag}"
        );
        assert!(!number.commit());
        assert_eq!(value.get(), 10.);
        number.binding().set(invalid.into());
        assert_eq!(
            number.validation().status,
            ValidationStatus::Invalid,
            "{tag}"
        );
    }
    let picker = ColorPicker::new(State::new(Color::rgba(1, 2, 3, 4)).binding());
    for text in ["#a", "#abc", "#abcde", "#abcdef0"] {
        picker.binding().set(text.into());
        assert_eq!(
            picker.validation().status,
            ValidationStatus::Incomplete,
            "{text}"
        );
        assert!(!picker.commit());
    }
    for text in ["#abcdeg", "#abcdef000", "abc"] {
        picker.binding().set(text.into());
        assert_eq!(
            picker.validation().status,
            ValidationStatus::Invalid,
            "{text}"
        );
    }
    let picker = picker.alpha(false);
    picker.binding().set("#abcdef0".into());
    assert_eq!(picker.validation().status, ValidationStatus::Invalid);
}

#[test]
fn generic_codecs_can_declare_prefixes_and_iso_helpers_use_the_same_rules() {
    let draft = Draft::new(
        State::new(1i64).binding(),
        |s| s.parse::<i64>().map_err(|e| e.to_string()),
        |v| v.to_string(),
    )
    .incomplete_when(|s| s == "pending");
    draft.binding().set("pending".into());
    assert_eq!(draft.validation().status, ValidationStatus::Incomplete);
    assert!(!draft.commit());
    draft.binding().set("2".into());
    assert_eq!(draft.validation().status, ValidationStatus::Valid);
    assert!(draft.commit());
    let time = time_draft(State::new(Time::MIN).binding());
    time.binding().set("12:30:59.".into());
    assert_eq!(time.validation().status, ValidationStatus::Incomplete);
    let datetime =
        datetime_draft(State::new("2024-01-01T12:30".parse::<DateTime>().unwrap()).binding());
    datetime.binding().set("2024-02-29T".into());
    assert_eq!(datetime.validation().status, ValidationStatus::Incomplete);
}

#[test]
fn every_prefix_of_representative_formatted_values_can_be_typed_incrementally() {
    fn check(name: &str, binding: Binding<String>, validation: impl Fn() -> Validation) {
        let text = binding.get();
        for end in text
            .char_indices()
            .map(|(i, _)| i)
            .chain(std::iter::once(text.len()))
        {
            binding.set(text[..end].into());
            assert!(
                matches!(
                    validation().status,
                    ValidationStatus::Valid | ValidationStatus::Incomplete
                ),
                "{name}: {:?} is a prefix of {text:?}",
                &text[..end]
            );
        }
    }
    for tag in ["ASCII/ISO", "en-US", "en-GB", "de-DE", "fr-FR", "ar-EG"] {
        let locale = if tag == "ASCII/ISO" {
            EditLocale::ASCII
        } else {
            EditLocale::new(tag).unwrap()
        };
        let number = Numeric::new(State::new(-1234567.125).binding(), -1e12, 1e12, 1.)
            .locale(locale.clone());
        check(tag, number.binding(), || number.validation());
        let date = DatePicker::new(State::new("2024-02-29".parse::<Date>().unwrap()).binding())
            .locale(locale.clone());
        check(tag, date.binding(), || date.validation());
        for cycle in [HourCycle::H24, HourCycle::H12] {
            let time = TimePicker::new(
                State::new("23:59:59.123456789".parse::<Time>().unwrap()).binding(),
            )
            .locale(locale.clone())
            .hour_cycle(cycle)
            .show_seconds(true);
            check(tag, time.binding(), || time.validation());
            let datetime = DateTimePicker::new(
                State::new("2024-02-29T23:59:59.123456789".parse::<DateTime>().unwrap()).binding(),
            )
            .locale(locale.clone())
            .hour_cycle(cycle)
            .show_seconds(true);
            check(tag, datetime.binding(), || datetime.validation());
        }
    }
    for text in ["0000-02-29", "-000004-02-29", "-009999-01-01"] {
        let date = DatePicker::new(State::new(text.parse::<Date>().unwrap()).binding());
        check(text, date.binding(), || date.validation());
    }
    let zoned = ZonedPicker::new(
        State::new("2024-02-29T23:59:59Z".parse::<Timestamp>().unwrap()).binding(),
        "UTC",
    )
    .unwrap()
    .show_seconds(true);
    check("zoned", zoned.binding(), || zoned.validation());
    let color = ColorPicker::new(State::new(Color::rgba(1, 2, 3, 4)).binding());
    check("color", color.binding(), || color.validation());
}
