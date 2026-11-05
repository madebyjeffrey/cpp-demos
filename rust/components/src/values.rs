//! Typed sRGB and ISO civil/zoned controls. Parsing never guesses a locale or DST policy.
use crate::{Binding, Draft, State, View};
pub use jiff::{
    Zoned,
    civil::{Date, DateTime, Time},
    tz::TimeZone,
};
use std::{fmt, str::FromStr};
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
pub fn value_field<T: Clone + 'static>(label: impl Into<String>, draft: &Draft<T>) -> View {
    let commit = draft.clone();
    let apply = draft.clone();
    let cancel = draft.clone();
    let mut children = vec![
        View::label(label),
        View::text_field("", draft.binding()).on_submit(move || {
            commit.commit();
        }),
        View::row(vec![
            View::button("Apply", move || {
                apply.commit();
            }),
            View::button("Revert", move || cancel.cancel()),
        ]),
    ];
    if let Some(error) = draft.error() {
        children.push(View::label(error).wrap(true).foreground(0xffff8787));
    }
    View::column(children)
}
#[derive(Clone)]
pub struct ColorPicker {
    value: Binding<Color>,
    pub draft: Draft<Color>,
}
impl ColorPicker {
    pub fn new(value: Binding<Color>) -> Self {
        Self {
            draft: Draft::new(value.clone(), |s| s.parse(), |v| v.to_string()),
            value,
        }
    }
    pub fn view(&self) -> View {
        let color = self.value.get();
        let mut children = vec![
            View::panel(vec![View::label(color.to_string())])
                .background(color.0)
                .height(48),
        ];
        for (index, label) in ["Red", "Green", "Blue", "Alpha"].into_iter().enumerate() {
            let read = self.value.clone();
            let write = self.value.clone();
            let binding = Binding::new(
                move || read.get().channels()[index] as f64,
                move |v: f64| {
                    let mut channels = write.get().channels();
                    channels[index] = v.round().clamp(0., 255.) as u8;
                    write.set(Color::rgba(
                        channels[0],
                        channels[1],
                        channels[2],
                        channels[3],
                    ));
                },
            );
            children.push(View::row(vec![
                View::label(label).width(48),
                View::slider(binding, 0., 255., 1.).grow(1.),
            ]));
        }
        children.push(value_field("sRGB · #RRGGBBAA", &self.draft));
        View::column(children)
    }
    pub fn well(&self, popup: &crate::Popup, key: &str) -> (View, View) {
        let open = popup.clone();
        (
            View::button(self.value.get().to_string(), move || open.show())
                .key(key)
                .background(self.value.get().0),
            popup.modal("Color", self.view()),
        )
    }
}
#[derive(Clone)]
pub struct DatePicker {
    value: Binding<Date>,
    pub draft: Draft<Date>,
    month: State<Date>,
}
impl DatePicker {
    pub fn new(value: Binding<Date>) -> Self {
        Self {
            month: State::new(value.get().first_of_month()),
            draft: Draft::new(
                value.clone(),
                |s| s.parse::<Date>().map_err(|e| e.to_string()),
                |v| v.to_string(),
            ),
            value,
        }
    }
    pub fn view(&self) -> View {
        let month = self.month.get();
        let prev = self.month.clone();
        let next = self.month.clone();
        let mut children = vec![View::row(vec![
            View::button("‹", move || {
                if let Ok(date) = prev.get().checked_add(jiff::Span::new().months(-1)) {
                    prev.set(date);
                }
            })
            .width(36),
            View::label(format!("{}-{:02}", month.year(), month.month())).grow(1.),
            View::button("›", move || {
                if let Ok(date) = next.get().checked_add(jiff::Span::new().months(1)) {
                    next.set(date);
                }
            })
            .width(36),
        ])];
        children.push(View::row(
            ["Mo", "Tu", "We", "Th", "Fr", "Sa", "Su"]
                .into_iter()
                .map(|s| View::label(s).width(crate::percent(100. / 7.)))
                .collect(),
        ));
        let offset = month.weekday().to_monday_zero_offset();
        for week in 0..6 {
            let mut cells = Vec::new();
            for day in 0..7 {
                let number = week * 7 + day - offset + 1;
                if number < 1 || number > month.days_in_month() {
                    cells.push(View::label("").width(crate::percent(100. / 7.)));
                    continue;
                }
                let date = Date::new(month.year(), month.month(), number).unwrap();
                let value = self.value.clone();
                cells.push(
                    View::button(number.to_string(), move || value.set(date))
                        .width(crate::percent(100. / 7.))
                        .bold(date == self.value.get())
                        .tooltip(date.to_string()),
                );
            }
            children.push(View::row(cells).gap(4.));
            if (week + 1) * 7 - offset >= month.days_in_month() {
                break;
            }
        }
        children.push(value_field("Date · YYYY-MM-DD", &self.draft));
        View::column(children)
    }
}
pub fn time_draft(value: Binding<Time>) -> Draft<Time> {
    Draft::new(
        value,
        |s| s.parse::<Time>().map_err(|e| e.to_string()),
        |v| v.to_string(),
    )
}
pub fn datetime_draft(value: Binding<DateTime>) -> Draft<DateTime> {
    Draft::new(
        value,
        |s| s.parse::<DateTime>().map_err(|e| e.to_string()),
        |v| v.to_string(),
    )
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
#[derive(Clone)]
pub struct ZonedPicker {
    pub resolution: State<String>,
    pub draft: Draft<Zoned>,
}
impl ZonedPicker {
    pub fn new(value: Binding<Zoned>) -> Self {
        let resolution = State::new("reject".to_owned());
        let policy = resolution.clone();
        let source = value.clone();
        let draft = Draft::new(
            value,
            move |s| {
                let strategy = match policy.get().as_str() {
                    "earlier" => jiff::tz::Disambiguation::Earlier,
                    "later" => jiff::tz::Disambiguation::Later,
                    _ => jiff::tz::Disambiguation::Reject,
                };
                if s.contains('[') {
                    jiff::fmt::temporal::DateTimeParser::new()
                        .disambiguation(strategy)
                        .parse_zoned(s)
                        .map_err(|e| e.to_string())
                } else {
                    let local = s.parse::<DateTime>().map_err(|e| e.to_string())?;
                    source
                        .get()
                        .time_zone()
                        .to_ambiguous_zoned(local)
                        .disambiguate(strategy)
                        .map_err(|e| e.to_string())
                }
            },
            |z| z.to_string(),
        );
        Self { resolution, draft }
    }
    pub fn view(&self) -> View {
        View::column(vec![
            View::segmented(
                self.resolution.binding(),
                vec![
                    crate::OptionItem::new("reject", "Reject ambiguity"),
                    crate::OptionItem::new("earlier", "Earlier"),
                    crate::OptionItem::new("later", "Later"),
                ],
            ),
            value_field("ISO date/time · [IANA/Zone]", &self.draft),
        ])
    }
}
