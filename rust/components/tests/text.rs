use scars_ui::text::*;

#[test]
fn unicode_clusters_and_native_offsets_are_checked() {
    let text = "A👩‍👩‍👧‍👦é🇨🇦Z";
    let boundaries = grapheme_boundaries(text);
    assert_eq!(boundaries.len(), 6);
    assert_eq!(boundaries.first(), Some(&0));
    assert_eq!(boundaries.last(), Some(&text.len()));
    for (byte, _) in text
        .char_indices()
        .chain(std::iter::once((text.len(), '\0')))
    {
        assert_eq!(
            byte_offset(text, utf16_offset(text, byte).unwrap()),
            Some(byte)
        );
    }
    assert_eq!(byte_offset("😀", 1), None);
    assert_eq!(utf16_offset("😀", 1), None);
    assert_eq!(utf16_offset("a", usize::MAX), None);
    assert_eq!(byte_offset("a", usize::MAX), None);
    assert_eq!(grapheme_boundaries(""), vec![0]);
    assert_eq!(word_boundaries("Hello world"), vec![0, 5, 6, 11]);
}

#[test]
fn localized_integer_display_preserves_full_integer_precision() {
    let english = IntegerFormatter::new("en-US").unwrap();
    let german = IntegerFormatter::new("de-DE").unwrap();
    assert_eq!(english.format(1_234_567), "1,234,567");
    assert_eq!(german.format(1_234_567), "1.234.567");
    assert_eq!(english.format(i64::MAX), "9,223,372,036,854,775,807");
    assert_eq!(english.format(i64::MIN), "-9,223,372,036,854,775,808");
    assert!(IntegerFormatter::new("not a locale!").is_err());
}
