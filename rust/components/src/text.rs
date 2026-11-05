//! Unicode boundaries use UTF-8 byte offsets. Native transport uses UTF-16 units;
//! checked conversions reject interior code-unit/byte positions rather than guessing.

use crate::Error;
use icu_decimal::DecimalFormatter;
use icu_locale_core::Locale;
use icu_segmenter::{GraphemeClusterSegmenter, WordSegmenter};

pub fn grapheme_boundaries(text: &str) -> Vec<usize> {
    GraphemeClusterSegmenter::new().segment_str(text).collect()
}
pub fn word_boundaries(text: &str) -> Vec<usize> {
    WordSegmenter::new_auto(Default::default())
        .segment_str(text)
        .collect()
}
pub fn utf16_offset(text: &str, byte: usize) -> Option<usize> {
    Some(text.get(..byte)?.encode_utf16().count())
}
pub fn byte_offset(text: &str, utf16: usize) -> Option<usize> {
    let mut units = 0;
    for (byte, ch) in text.char_indices() {
        if units == utf16 {
            return Some(byte);
        }
        units += ch.len_utf16();
        if units > utf16 {
            return None;
        }
    }
    (units == utf16).then_some(text.len())
}

/// Locale-aware integer display only. Parsing and editable field policy are later stages.
pub struct IntegerFormatter(DecimalFormatter);
impl IntegerFormatter {
    /// Uses BCP 47 locale names and ICU4X's standard data fallback.
    pub fn new(locale: &str) -> Result<Self, Error> {
        let locale: Locale = locale
            .parse()
            .map_err(|e| Error(format!("Invalid locale: {e}")))?;
        DecimalFormatter::try_new((&locale).into(), Default::default())
            .map(Self)
            .map_err(|e| Error(format!("Cannot create number formatter: {e}")))
    }
    pub fn format(&self, value: i64) -> String {
        self.0.format_to_string(&value.into())
    }
}
