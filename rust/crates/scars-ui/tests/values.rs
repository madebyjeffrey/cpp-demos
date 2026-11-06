use scars_ui::*;
#[test]
fn colors_round_trip_alpha_and_reject_ambiguous_formats() {
    let c = Color::rgba(12, 34, 56, 78);
    assert_eq!(c.to_string().parse::<Color>().unwrap(), c);
    assert_eq!(
        "#65B0FF".parse::<Color>().unwrap().channels(),
        [101, 176, 255, 255]
    );
    for bad in ["red", "#12", "#12345Z", "#123456789"] {
        assert!(bad.parse::<Color>().is_err());
    }
}
#[test]
fn civil_validation_and_zone_resolution_handle_leaps_gaps_and_overlaps() {
    assert!("2024-02-29".parse::<Date>().is_ok());
    assert!("2025-02-29".parse::<Date>().is_err());
    assert!("25:00:00".parse::<Time>().is_err());
    let gap = "2024-03-10T02:30:00".parse::<DateTime>().unwrap();
    assert!(resolve_zoned(gap, "America/Edmonton", Resolution::Reject).is_err());
    let overlap = "2024-11-03T01:30:00".parse::<DateTime>().unwrap();
    assert!(resolve_zoned(overlap, "America/Edmonton", Resolution::Reject).is_err());
    let early = resolve_zoned(overlap, "America/Edmonton", Resolution::Earlier).unwrap();
    let late = resolve_zoned(overlap, "America/Edmonton", Resolution::Later).unwrap();
    assert_eq!(
        late.timestamp().as_second() - early.timestamp().as_second(),
        3600
    );
    assert!(resolve_zoned(overlap, "Invalid/Zone", Resolution::Reject).is_err());
}
#[test]
fn invalid_zoned_draft_preserves_committed_instant() {
    let initial = resolve_zoned(
        "2026-01-01T12:00:00".parse().unwrap(),
        "America/Edmonton",
        Resolution::Reject,
    )
    .unwrap();
    let value = State::new(initial.timestamp());
    let picker = ZonedPicker::new(value.binding(), "America/Edmonton")
        .unwrap()
        .show_seconds(true);
    picker.binding().set("2024-03-10T02:30:00".into());
    assert!(!picker.commit());
    assert_eq!(value.get(), initial.timestamp());
    picker.set_repeated_time(RepeatedTimePolicy::Later);
    assert!(
        !picker.commit(),
        "gaps stay invalid under a repeated-time policy"
    );
    picker.binding().set("2024-11-03T01:30:00".into());
    assert!(picker.commit());
}
