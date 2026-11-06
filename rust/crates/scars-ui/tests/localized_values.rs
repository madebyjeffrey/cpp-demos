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
fn locale(tag: &str) -> EditLocale {
    EditLocale::new(tag).unwrap()
}

#[test]
fn profiles_are_explicit_and_formatting_round_trips_exact_integers() {
    assert_eq!(EditLocale::default(), EditLocale::ASCII);
    assert_eq!(locale("EN-us").as_str(), "en-US");
    for bad in [
        "en",
        "en_US",
        "zz-ZZ",
        "en-CA",
        "de-DE-u-nu-latn",
        "ar-EG-u-nu-latn",
        "en-US-u-ca-buddhist",
    ] {
        assert!(EditLocale::new(bad).is_err(), "silently accepted {bad}");
    }
    for tag in ["en-US", "en-GB", "de-DE", "fr-FR", "ar-EG"] {
        let value = State::new(0i64);
        let n = Numeric::new(value.binding(), i64::MIN, i64::MAX, 1).locale(locale(tag));
        for exact in [
            i64::MIN,
            -9_007_199_254_740_993,
            0,
            9_007_199_254_740_993,
            i64::MAX,
        ] {
            value.set(exact);
            let display = n.draft.binding().get();
            let other = State::new(42i64);
            let edit = Numeric::new(other.binding(), i64::MIN, i64::MAX, 1).locale(locale(tag));
            edit.draft.binding().set(display.clone());
            assert!(edit.commit(), "{tag}: {display:?}: {:?}", edit.validation());
            assert_eq!(other.get(), exact);
            if tag == "ar-EG" {
                assert!(display.contains('٠') || display.chars().any(|c| ('١'..='٩').contains(&c)));
            }
        }
        let high = State::new(u64::MAX);
        let wide = Numeric::new(high.binding(), 0, u64::MAX, 1).locale(locale(tag));
        let text = wide.draft.binding().get();
        high.set(0);
        wide.draft.binding().set(text);
        assert!(wide.commit());
        assert_eq!(high.get(), u64::MAX);
    }
}

#[test]
fn separators_grouping_signs_and_digit_sets_have_one_meaning() {
    for (tag, good, expected, bad) in [
        (
            "en-US",
            "1,234.5",
            1234.5,
            vec!["12,34.5", "1.234,5", "1 234.5", "1,,234", "1,234.5,6"],
        ),
        (
            "de-DE",
            "1.234,5",
            1234.5,
            vec!["12.34,5", "1,234.5", "1 234,5", "1.234,5.6", "1,2e1.000"],
        ),
        (
            "fr-FR",
            "1\u{202f}234,5",
            1234.5,
            vec!["1 234,5", "1\u{a0}234,5", "12\u{202f}34,5", "1.234,5"],
        ),
        (
            "ar-EG",
            "١٬٢٣٤٫٥",
            1234.5,
            vec![
                "١2٣٤٫٥",
                "١٢٣٤٫5",
                "١٫٢e3",
                "١٬٢٣٫٥",
                "١٢٣٤.٥",
                "۱۲۳۴٫۵",
                "\u{200f}١٢",
            ],
        ),
    ] {
        let value = State::new(0.);
        let n = Numeric::new(value.binding(), -1e10, 1e10, 0.5).locale(locale(tag));
        n.draft.binding().set(good.into());
        assert!(n.commit(), "{tag}");
        assert_eq!(value.get(), expected);
        for text in bad {
            n.draft.binding().set(text.into());
            assert!(!n.commit(), "accepted {tag} {text}");
            assert_eq!(value.get(), expected);
        }
    }
    let value = State::new(0.);
    let de = Numeric::new(value.binding(), -1e10, 1e10, 0.5).locale(locale("de-DE"));
    de.draft.binding().set("1,234".into());
    assert!(de.commit());
    assert_eq!(value.get(), 1.234);
    let ar = Numeric::new(value.binding(), -1e10, 1e10, 0.5).locale(locale("ar-EG"));
    ar.draft.binding().set("-1234٫5".into());
    assert!(ar.commit());
    assert_eq!(value.get(), -1234.5);
    for good in ["+١٫٥e٢", "١٫٥e+٢", "١٫٥e-٢"] {
        ar.draft.binding().set(good.into());
        assert!(ar.commit(), "{good}");
    }
    for partial in ["", "-", "+", "٫", "-٫", "١e", "١e+"] {
        ar.draft.binding().set(partial.into());
        assert_eq!(
            ar.validation().status,
            ValidationStatus::Incomplete,
            "{partial}"
        );
        assert!(!ar.commit());
    }
}

#[test]
fn localized_drafts_step_without_losing_precision_or_overwriting_invalid_edits() {
    let value = State::new(9_007_199_254_740_993i64);
    let n = Numeric::new(value.binding(), i64::MIN, i64::MAX, 1).locale(locale("de-DE"));
    n.draft.binding().set("9.007.199.254.740.994".into());
    assert!(n.step(true));
    assert_eq!(value.get(), 9_007_199_254_740_995);
    assert_eq!(n.draft.binding().get(), "9.007.199.254.740.995");
    n.draft.binding().set("9.22".into());
    assert!(!n.step(true));
    assert_eq!(value.get(), 9_007_199_254_740_995);
    assert_eq!(n.draft.binding().get(), "9.22");
    n.draft.binding().set("9.223.372.036.854.775.808".into());
    assert_eq!(n.validation().status, ValidationStatus::OutOfRange);
    assert!(!n.commit());
    let small = State::new(0.);
    let bounded = Numeric::new(small.binding(), -1000., 1000., 0.5).locale(locale("de-DE"));
    bounded.draft.binding().set("2.000,5".into());
    assert_eq!(bounded.validation().status, ValidationStatus::OutOfRange);
    assert!(bounded.validation().message.unwrap().contains("1.000"));
    small.set(1000.0001);
    let rounded = bounded.clone().significant_digits(3);
    assert_eq!(rounded.validation().status, ValidationStatus::OutOfRange);
    assert!(!rounded.validation().dirty);
    assert!(
        rounded
            .validation()
            .message
            .unwrap()
            .starts_with("Wert von")
    );
    small.set(0.);
    let readonly = bounded
        .read_only(true)
        .locale(locale("fr-FR"))
        .significant_digits(3);
    let initial = readonly.draft.binding().get();
    readonly.draft.binding().set("1,5".into());
    assert_eq!(readonly.draft.binding().get(), initial);
    assert!(!readonly.commit());
    assert!(!readonly.step(true));
}

#[test]
fn rounding_exponents_and_external_changes_preserve_the_source() {
    let value = State::new(1.23456789e20);
    for tag in ["en-US", "de-DE", "fr-FR", "ar-EG"] {
        let a = Numeric::new(value.binding(), -f64::MAX, f64::MAX, 1.)
            .locale(locale(tag))
            .significant_digits(4);
        let b = Numeric::new(value.binding(), -f64::MAX, f64::MAX, 1.)
            .significant_digits(4)
            .locale(locale(tag));
        assert_eq!(a.draft.binding().get(), b.draft.binding().get());
        assert!(a.commit());
        assert_eq!(value.get(), 1.23456789e20);
        let other = State::new(0.);
        let edit = Numeric::new(other.binding(), -f64::MAX, f64::MAX, 1.).locale(locale(tag));
        edit.draft.binding().set(a.draft.binding().get());
        assert!(edit.commit());
        assert_eq!(other.get(), 1.235e20);
        a.draft.binding().set("bad".into());
        value.set(42.5);
        assert_ne!(a.draft.binding().get(), "bad");
        assert!(a.commit());
        assert_eq!(value.get(), 42.5);
        value.set(1.23456789e20);
        for exact in [
            f64::MAX,
            -f64::MAX,
            f64::MIN_POSITIVE,
            f64::from_bits(1),
            -0.,
        ] {
            let source = State::new(exact);
            let source =
                Numeric::new(source.binding(), -f64::MAX, f64::MAX, 1.).locale(locale(tag));
            let target = State::new(42.);
            let target_edit =
                Numeric::new(target.binding(), -f64::MAX, f64::MAX, 1.).locale(locale(tag));
            target_edit
                .draft
                .binding()
                .set(source.draft.binding().get());
            assert!(target_edit.commit(), "{tag}: {exact}");
            assert_eq!(target.get().to_bits(), exact.to_bits());
        }
    }
}

#[test]
fn localized_dates_follow_only_the_selected_field_order_and_reject_unsupported_years() {
    for (tag, text, expected) in [
        ("en-US", "03/04/2026", Date::new(2026, 3, 4).unwrap()),
        ("en-GB", "03/04/2026", Date::new(2026, 4, 3).unwrap()),
        ("de-DE", "04.03.2026", Date::new(2026, 3, 4).unwrap()),
        ("fr-FR", "04/03/2026", Date::new(2026, 3, 4).unwrap()),
        ("ar-EG", "٠٤/٠٣/٢٠٢٦", Date::new(2026, 3, 4).unwrap()),
    ] {
        let value = State::new(Date::new(2024, 2, 29).unwrap());
        let picker = DatePicker::new(value.binding()).locale(locale(tag));
        picker.draft.binding().set(text.into());
        assert!(picker.draft.commit(), "{tag}");
        assert_eq!(value.get(), expected);
        assert_eq!(picker.draft.binding().get(), text);
        for bad in [
            "2026-03-04",
            "03/04/26",
            "31/02/2026",
            "00/00/0000",
            "03/04/2026T12:00:00",
            "04/03/20٢٦",
        ] {
            picker.draft.binding().set(bad.into());
            assert!(!picker.draft.commit(), "accepted {tag} {bad}");
            assert_eq!(value.get(), expected);
        }
        for exact in [
            Date::new(1, 1, 1).unwrap(),
            Date::new(9999, 12, 31).unwrap(),
            Date::new(2024, 2, 29).unwrap(),
        ] {
            value.set(exact);
            let other = State::new(Date::new(2000, 1, 1).unwrap());
            let edit = DatePicker::new(other.binding()).locale(locale(tag));
            edit.draft.binding().set(picker.draft.binding().get());
            assert!(edit.draft.commit());
            assert_eq!(other.get(), exact);
        }
        value.set(Date::MIN);
        assert_eq!(picker.draft.validation().status, ValidationStatus::Invalid);
        assert!(!picker.draft.commit());
        let iso = DatePicker::new(value.binding()).locale(EditLocale::ASCII);
        assert!(iso.draft.commit());
        assert_eq!(iso.draft.binding().get(), Date::MIN.to_string());
    }
}

#[test]
fn calendars_have_local_labels_week_order_and_bounded_cells_in_both_directions() {
    for (tag, heading, first_label, apply, next, day1, day2) in [
        (
            "en-US",
            "March 2026",
            "Su",
            "Apply",
            "Next month",
            "03/01/2026",
            "03/02/2026",
        ),
        (
            "de-DE",
            "März 2026",
            "Mo",
            "Übernehmen",
            "Nächster Monat",
            "01.03.2026",
            "02.03.2026",
        ),
        (
            "fr-FR",
            "mars 2026",
            "Lu",
            "Appliquer",
            "Mois suivant",
            "01/03/2026",
            "02/03/2026",
        ),
        (
            "ar-EG",
            "مارس ٢٠٢٦",
            "سبت",
            "تطبيق",
            "الشهر التالي",
            "٠١/٠٣/٢٠٢٦",
            "٠٢/٠٣/٢٠٢٦",
        ),
    ] {
        let date = State::new(Date::new(2026, 3, 1).unwrap());
        let picker = DatePicker::new(date.binding()).locale(locale(tag));
        let host = Host::new(move || picker.view());
        host.layout(Size::new(310., 600.), &Metrics).unwrap();
        let nodes = host.snapshot().unwrap();
        assert!(nodes.iter().any(|n| n.text == heading));
        assert!(nodes.iter().any(|n| n.text == apply));
        let days: Vec<_> = nodes
            .iter()
            .filter(|n| {
                n.kind == Kind::Label
                    && [
                        "Mo", "Tu", "We", "Th", "Fr", "Sa", "Su", "Di", "Mi", "Do", "So", "Lu",
                        "Ma", "Me", "Je", "Ve", "اثن", "ثلا", "أرب", "خمي", "جمع", "سبت", "أحد",
                    ]
                    .contains(&n.text.as_str())
            })
            .collect();
        let first = if tag == "ar-EG" {
            days.iter()
                .max_by(|a, b| a.bounds.x.total_cmp(&b.bounds.x))
                .unwrap()
        } else {
            days.iter()
                .min_by(|a, b| a.bounds.x.total_cmp(&b.bounds.x))
                .unwrap()
        };
        assert_eq!(first.text, first_label);
        for n in nodes.iter().filter(|n| {
            n.kind == Kind::Button && (n.label.contains("2026") || n.label.contains("٢٠٢٦"))
        }) {
            assert!(
                n.bounds.x >= 0. && (n.bounds.x + n.bounds.width) <= 310.001,
                "{tag}: {:?}",
                n.bounds
            );
        }
        let day = nodes
            .iter()
            .find(|n| n.kind == Kind::Button && n.label == day2)
            .unwrap();
        assert!(host.activate(day.id).unwrap());
        assert_eq!(date.get(), Date::new(2026, 3, 2).unwrap());
        let one = nodes
            .iter()
            .find(|n| n.kind == Kind::Button && n.label == day1)
            .unwrap();
        if tag == "ar-EG" {
            assert!(day.bounds.x < one.bounds.x);
        } // RTL progression in a week.
        let advance = nodes.iter().find(|n| n.label == next).unwrap();
        assert!(host.activate(advance.id).unwrap());
        assert!(
            host.snapshot()
                .unwrap()
                .iter()
                .any(|n| n.text.contains(if tag == "de-DE" {
                    "April"
                } else if tag == "fr-FR" {
                    "avril"
                } else if tag == "ar-EG" {
                    "أبريل"
                } else {
                    "April"
                }))
        );
    }
}
