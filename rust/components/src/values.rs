//! Typed sRGB and ISO civil/zoned controls. Parsing never guesses a locale or DST policy.
use crate::locale::ActionLabel;
use crate::{Binding, Draft, EditLocale, View};
pub use jiff::{
    Timestamp, Zoned,
    civil::{Date, DateTime, Time},
    tz::TimeZone,
};
use std::{fmt, str::FromStr};
// Civil editors must consume exactly their advertised grammar. Jiff's general
// Temporal parsers also accept timestamps and can discard their extra fields.
pub(crate) fn parse_date(text: &str) -> Result<Date, String> {
    let date = text.parse::<Date>().map_err(|e| e.to_string())?;
    if text != date.to_string() {
        return Err("Use an ISO date: YYYY-MM-DD, without time or offset".into());
    }
    Ok(date)
}
pub(crate) fn parse_time(text: &str) -> Result<Time, String> {
    let (clock, fraction) = text
        .split_once('.')
        .map_or((text, None), |(a, b)| (a, Some(b)));
    let valid = matches!(clock.len(), 5 | 8)
        && clock.bytes().enumerate().all(|(i, b)| {
            if matches!(i, 2 | 5) {
                b == b':'
            } else {
                b.is_ascii_digit()
            }
        })
        && clock.as_bytes().get(6..8).is_none_or(|s| s < b"60")
        && fraction.is_none_or(|s| {
            clock.len() == 8 && (1..=9).contains(&s.len()) && s.bytes().all(|b| b.is_ascii_digit())
        });
    if !valid {
        return Err("Use HH:MM[:SS[.fraction]], without a date, offset or zone".into());
    }
    text.parse::<Time>().map_err(|e| e.to_string())
}
pub(crate) fn parse_datetime(text: &str) -> Result<DateTime, String> {
    let (date, time) = text
        .split_once('T')
        .ok_or("Use an ISO local date/time: YYYY-MM-DDTHH:MM[:SS]")?;
    Ok(parse_date(date)?.to_datetime(parse_time(time)?))
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Color(pub u32);
impl Color {
    pub fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self(u32::from_be_bytes([a, r, g, b]))
    }
    pub fn channels(self) -> [u8; 4] {
        let [a, r, g, b] = self.0.to_be_bytes();
        [r, g, b, a]
    }
}
impl fmt::Display for Color {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let [r, g, b, a] = self.channels();
        write!(f, "#{r:02X}{g:02X}{b:02X}{a:02X}")
    }
}
impl FromStr for Color {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, String> {
        let value = s.strip_prefix('#').ok_or("Use #RRGGBB or #RRGGBBAA")?;
        if !matches!(value.len(), 6 | 8) || !value.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err("Use #RRGGBB or #RRGGBBAA".into());
        }
        let v = u32::from_str_radix(value, 16).map_err(|e| e.to_string())?;
        Ok(if value.len() == 6 {
            Self(0xff000000 | v)
        } else {
            let [r, g, b, a] = v.to_be_bytes();
            Self::rgba(r, g, b, a)
        })
    }
}
pub fn value_field<T: Clone + PartialEq + 'static>(
    label: impl Into<String>,
    draft: &Draft<T>,
) -> View {
    localized_value_field(label, draft, &EditLocale::ASCII)
}
pub(crate) fn localized_value_field<T: Clone + PartialEq + 'static>(
    label: impl Into<String>,
    draft: &Draft<T>,
    locale: &EditLocale,
) -> View {
    let label = label.into();
    let commit = draft.clone();
    let apply = draft.clone();
    let cancel = draft.clone();
    let escape = draft.clone();
    let read_only = draft.is_read_only();
    let mut children = vec![
        View::label(&label),
        View::text_field("", draft.binding())
            .accessible_label(label)
            .read_only(read_only)
            .validation(draft.validation())
            .on_cancel(move || escape.cancel())
            .on_submit(move || {
                commit.commit();
            }),
        View::row(vec![
            View::button(locale.action(ActionLabel::Apply), move || {
                apply.commit();
            })
            .enabled(!read_only),
            View::button(locale.action(ActionLabel::Revert), move || cancel.cancel())
                .enabled(!read_only),
        ]),
    ];
    if let Some(error) = draft.error() {
        children.push(View::label(error).wrap(true).foreground(0xffff8787));
    }
    View::column(children)
}
pub fn time_draft(value: Binding<Time>) -> Draft<Time> {
    Draft::new(value, parse_time, |v| v.to_string())
}
pub fn datetime_draft(value: Binding<DateTime>) -> Draft<DateTime> {
    Draft::new(value, parse_datetime, |v| v.to_string())
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Resolution {
    Reject,
    Earlier,
    Later,
}
pub fn resolve_zoned(local: DateTime, zone: &str, resolution: Resolution) -> Result<Zoned, String> {
    let zone = TimeZone::get(zone).map_err(|e| e.to_string())?;
    let ambiguous = zone.to_ambiguous_zoned(local);
    match resolution {
        Resolution::Reject => ambiguous.unambiguous(),
        Resolution::Earlier => ambiguous.earlier(),
        Resolution::Later => ambiguous.later(),
    }
    .map_err(|e| e.to_string())
}
