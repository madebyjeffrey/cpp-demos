//! Bounded decimal fields for append-only date/clock prefix recognition.
use std::ops::RangeInclusive;
fn range(
    text: &str,
    width: usize,
    min: u16,
    max: u16,
    complete: bool,
) -> Option<RangeInclusive<u16>> {
    if text.len() > width
        || complete && text.len() != width
        || !text.bytes().all(|b| b.is_ascii_digit())
    {
        return None;
    }
    let value = if text.is_empty() {
        0
    } else {
        text.parse::<u32>().ok()?
    };
    let scale = 10u32.pow((width - text.len()) as u32);
    let start = (value * scale).max(u32::from(min));
    let end = ((value + 1) * scale - 1).min(u32::from(max));
    (start <= end).then_some(start as u16..=end as u16)
}
pub(crate) fn fields(
    text: &str,
    separator: char,
    bounds: &[(usize, u16, u16)],
) -> Option<Vec<RangeInclusive<u16>>> {
    let parts: Vec<_> = text.split(separator).collect();
    if parts.len() > bounds.len() {
        return None;
    }
    bounds
        .iter()
        .enumerate()
        .map(|(i, &(width, min, max))| {
            range(
                parts.get(i).copied().unwrap_or(""),
                width,
                min,
                max,
                i + 1 < parts.len(),
            )
        })
        .collect()
}
