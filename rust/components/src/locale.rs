//! Checked editing profiles. ICU4X supplies decimal output; our grammar consumes it exactly.
use crate::{Date, Error};
use icu_decimal::{DecimalFormatter, input::Decimal};
use icu_locale_core::Locale;
use std::rc::Rc;

/// An explicit, validated editing locale shared by numeric and date controls.
/// ASCII/ISO is the default; unsupported profiles/extensions return an error.
#[derive(Clone, Debug, Default)]
pub struct EditLocale(Option<Rc<Localized>>);
#[derive(Debug)]
struct Localized {
    profile: Profile,
    formatter: DecimalFormatter,
    minus: (String, String),
    plus: (String, String),
}
#[derive(Clone, Copy)]
pub(crate) enum ActionLabel {
    Apply,
    Revert,
    PreviousMonth,
    NextMonth,
    Decrease,
    Increase,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Profile {
    Us,
    Gb,
    De,
    Fr,
    Ar,
}
impl PartialEq for EditLocale {
    fn eq(&self, other: &Self) -> bool {
        self.as_str() == other.as_str()
    }
}
impl Eq for EditLocale {}
impl EditLocale {
    pub const ASCII: Self = Self(None);
    pub fn new(tag: &str) -> Result<Self, Error> {
        let locale: Locale = tag
            .parse()
            .map_err(|e| Error(format!("Invalid editing locale: {e}")))?;
        let profile = match locale.to_string().as_str() {
            "en-US" => Profile::Us, "en-GB" => Profile::Gb,
            "de-DE" => Profile::De, "fr-FR" => Profile::Fr, "ar-EG" => Profile::Ar,
            _ => return Err(Error("Supported editing locales: en-US, en-GB, de-DE, fr-FR, ar-EG; use EditLocale::ASCII for ISO".into())),
        };
        // This profile explicitly promises Arabic-Indic digits, independent of ICU defaults.
        let locale = if profile == Profile::Ar {
            "ar-EG-u-nu-arab".parse().unwrap()
        } else {
            locale
        };
        let formatter = DecimalFormatter::try_new((&locale).into(), Default::default())
            .map_err(|e| Error(format!("Cannot create editing formatter: {e}")))?;
        let one = profile.digits("1");
        let affix = |raw: &str| -> Result<(String, String), Error> {
            let formatted = formatter.format_to_string(&raw.parse::<Decimal>().unwrap());
            let (before, after) = formatted.split_once(&one).ok_or_else(|| {
                Error("Unsupported ICU sign/digit data for editing profile".into())
            })?;
            Ok((before.into(), after.into()))
        };
        let minus = affix("-1")?;
        let plus = affix("+1")?;
        let out = Self(Some(Rc::new(Localized {
            profile,
            formatter,
            minus,
            plus,
        })));
        // Keep the public grammar stable if bundled ICU data changes. No edit fallback.
        let expected = profile.digits(&format!(
            "1{}023{}456{}789{}0",
            profile.group(),
            profile.group(),
            profile.group(),
            profile.decimal()
        ));
        if out.number("1023456789.0") != expected
            || out.number_input(&out.number("-1023456789.0")).as_deref() != Ok("-1023456789.0")
        {
            return Err(Error(
                "ICU output does not match the supported editing grammar".into(),
            ));
        }
        Ok(out)
    }
    pub fn as_str(&self) -> &'static str {
        match self.0.as_ref().map(|l| l.profile) {
            None => "ASCII/ISO",
            Some(Profile::Us) => "en-US",
            Some(Profile::Gb) => "en-GB",
            Some(Profile::De) => "de-DE",
            Some(Profile::Fr) => "fr-FR",
            Some(Profile::Ar) => "ar-EG",
        }
    }
    pub(crate) fn number(&self, ascii: &str) -> String {
        let Some(l) = &self.0 else {
            return ascii.into();
        };
        let (mantissa, exponent) = ascii
            .split_once(['e', 'E'])
            .map_or((ascii, None), |(m, e)| (m, Some(e)));
        let Ok(decimal) = mantissa.parse::<Decimal>() else {
            // Invalid external models (e.g. NaN) stay visible and invalid, never panic.
            return ascii.into();
        };
        let mut output = l.formatter.format_to_string(&decimal);
        if let Some(e) = exponent {
            output.push('e');
            output.push_str(&l.profile.digits(e));
        }
        output
    }
    pub(crate) fn number_input(&self, text: &str) -> Result<String, String> {
        let Some(l) = &self.0 else {
            return Ok(text.into());
        };
        let affix = |(prefix, suffix): &(String, String)| {
            (!(prefix.is_empty() && suffix.is_empty()))
                .then(|| text.strip_prefix(prefix)?.strip_suffix(suffix))
                .flatten()
        };
        let (sign, body) = if let Some(s) = affix(&l.minus) {
            ("-", s)
        } else if let Some(s) = affix(&l.plus) {
            ("+", s)
        } else if let Some(s) = text.strip_prefix('-') {
            ("-", s)
        } else if let Some(s) = text.strip_prefix('+') {
            ("+", s)
        } else {
            ("", text)
        };
        let body = l
            .profile
            .ascii_digits(body)
            .map_err(|_| self.number_error().to_owned())?;
        let mut parts = body.split(['e', 'E']);
        let mantissa = parts.next().unwrap();
        let exponent = parts.next();
        if parts.next().is_some() {
            return Err(self.number_error().into());
        }
        let mut decimal = mantissa.split(l.profile.decimal());
        let integer = decimal.next().unwrap();
        let fraction = decimal.next();
        if decimal.next().is_some() {
            return Err(self.number_error().into());
        }
        let groups: Vec<_> = integer.split(l.profile.group()).collect();
        if !groups.iter().all(|s| s.bytes().all(|b| b.is_ascii_digit()))
            || groups.len() > 1
                && (!(1..=3).contains(&groups[0].len()) || groups[1..].iter().any(|s| s.len() != 3))
            || fraction.is_some_and(|s| !s.bytes().all(|b| b.is_ascii_digit()))
            || exponent.is_some_and(|s| {
                !s.strip_prefix(['+', '-'])
                    .unwrap_or(s)
                    .bytes()
                    .all(|b| b.is_ascii_digit())
            })
        {
            return Err(self.number_error().into());
        }
        let mut ascii = format!("{sign}{}", groups.concat());
        if let Some(f) = fraction {
            ascii.push('.');
            ascii.push_str(f);
        }
        if let Some(e) = exponent {
            ascii.push('e');
            ascii.push_str(e);
        }
        Ok(ascii)
    }
    pub(crate) fn number_error(&self) -> &'static str {
        match self.profile() {
            Some(Profile::De) => {
                "Eine endliche Zahl mit Dezimalkomma und gültiger Gruppierung eingeben"
            }
            Some(Profile::Fr) => {
                "Saisissez un nombre fini avec une virgule décimale et un groupement valide"
            }
            Some(Profile::Ar) => "أدخل عددًا منتهيًا بفاصل عشري ٫ وتجميع صحيح",
            Some(_) => "Use a finite number with '.' decimals and valid grouping",
            None => "Use a finite ASCII number without grouping",
        }
    }
    pub(crate) fn incomplete_error(&self) -> &'static str {
        match self.profile() {
            Some(Profile::De) => "Eine vollständige Zahl eingeben",
            Some(Profile::Fr) => "Saisissez un nombre complet",
            Some(Profile::Ar) => "أدخل عددًا كاملًا",
            _ => "Enter a complete number",
        }
    }
    pub(crate) fn range_error(&self, min: &str, max: &str) -> String {
        let (min, max) = (self.number(min), self.number(max));
        match self.profile() {
            Some(Profile::De) => format!("Wert von {min} bis {max} eingeben"),
            Some(Profile::Fr) => format!("Saisissez une valeur de {min} à {max}"),
            Some(Profile::Ar) => format!("أدخل قيمة من {min} إلى {max}"),
            _ => format!("Enter a value from {min} to {max}"),
        }
    }
    pub(crate) fn number_hint(&self) -> &'static str {
        match self.profile() {
            Some(Profile::De) => "Zahl · 1.234,5",
            Some(Profile::Fr) => "Nombre · 1\u{202f}234,5",
            Some(Profile::Ar) => "عدد · ١٬٢٣٤٫٥",
            Some(_) => "Number · 1,234.5",
            None => "ASCII number",
        }
    }
    fn profile(&self) -> Option<Profile> {
        self.0.as_ref().map(|l| l.profile)
    }
    pub(crate) fn date(&self, date: Date) -> String {
        let Some(p) = self.profile() else {
            return date.to_string();
        };
        let (a, b) = if p == Profile::Us {
            (date.month(), date.day())
        } else {
            (date.day(), date.month())
        };
        p.digits(&format!(
            "{a:02}{}{b:02}{}{:04}",
            p.date_separator(),
            p.date_separator(),
            date.year()
        ))
    }
    pub(crate) fn date_supported(&self, date: Date) -> bool {
        self.0.is_none() || (1..=9999).contains(&date.year())
    }
    pub(crate) fn date_input(&self, text: &str) -> Result<Date, String> {
        let Some(p) = self.profile() else {
            return crate::values::parse_date(text);
        };
        let error = || format!("{} · {}", self.date_hint(), self.date_invalid());
        let ascii = p.ascii_digits(text).map_err(|_| error())?;
        let fields: Vec<_> = ascii.split(p.date_separator()).collect();
        if fields.len() != 3
            || fields[0].len() != 2
            || fields[1].len() != 2
            || fields[2].len() != 4
            || fields
                .iter()
                .any(|s| !s.bytes().all(|b| b.is_ascii_digit()))
        {
            return Err(error());
        }
        let (a, b, year) = (
            fields[0].parse::<i8>().map_err(|_| error())?,
            fields[1].parse::<i8>().map_err(|_| error())?,
            fields[2].parse::<i16>().map_err(|_| error())?,
        );
        if year < 1 {
            return Err(error());
        }
        let (month, day) = if p == Profile::Us { (a, b) } else { (b, a) };
        Date::new(year, month, day).map_err(|_| error())
    }
    fn date_invalid(&self) -> &'static str {
        match self.profile() {
            Some(Profile::De) => "Ungültiges Datum",
            Some(Profile::Fr) => "Date invalide",
            Some(Profile::Ar) => "تاريخ غير صالح",
            _ => "Invalid date",
        }
    }
    pub(crate) fn date_hint(&self) -> &'static str {
        match self.profile() {
            None => "Date · YYYY-MM-DD",
            Some(Profile::Us) => "Date · MM/DD/YYYY",
            Some(Profile::Gb) => "Date · DD/MM/YYYY",
            Some(Profile::De) => "Datum · TT.MM.JJJJ",
            Some(Profile::Fr) => "Date · JJ/MM/AAAA",
            Some(Profile::Ar) => "التاريخ · DD/MM/YYYY",
        }
    }
    pub(crate) fn week_start(&self) -> i8 {
        match self.profile() {
            Some(Profile::Us) => 6,
            Some(Profile::Ar) => 5,
            _ => 0,
        }
    }
    pub(crate) fn rtl(&self) -> bool {
        self.profile() == Some(Profile::Ar)
    }
    pub(crate) fn weekdays(&self) -> [&'static str; 7] {
        match self.profile() {
            Some(Profile::De) => ["Mo", "Di", "Mi", "Do", "Fr", "Sa", "So"],
            Some(Profile::Fr) => ["Lu", "Ma", "Me", "Je", "Ve", "Sa", "Di"],
            Some(Profile::Ar) => ["اثن", "ثلا", "أرب", "خمي", "جمع", "سبت", "أحد"],
            _ => ["Mo", "Tu", "We", "Th", "Fr", "Sa", "Su"],
        }
    }
    pub(crate) fn month(&self, date: Date) -> String {
        let Some(p) = self.profile() else {
            return format!("{}-{:02}", date.year(), date.month());
        };
        let names = match p {
            Profile::De => [
                "Januar",
                "Februar",
                "März",
                "April",
                "Mai",
                "Juni",
                "Juli",
                "August",
                "September",
                "Oktober",
                "November",
                "Dezember",
            ],
            Profile::Fr => [
                "janvier",
                "février",
                "mars",
                "avril",
                "mai",
                "juin",
                "juillet",
                "août",
                "septembre",
                "octobre",
                "novembre",
                "décembre",
            ],
            Profile::Ar => [
                "يناير",
                "فبراير",
                "مارس",
                "أبريل",
                "مايو",
                "يونيو",
                "يوليو",
                "أغسطس",
                "سبتمبر",
                "أكتوبر",
                "نوفمبر",
                "ديسمبر",
            ],
            _ => [
                "January",
                "February",
                "March",
                "April",
                "May",
                "June",
                "July",
                "August",
                "September",
                "October",
                "November",
                "December",
            ],
        };
        format!(
            "{} {}",
            names[date.month() as usize - 1],
            p.digits(&date.year().to_string())
        )
    }
    pub(crate) fn day(&self, day: i8) -> String {
        self.profile()
            .map_or_else(|| day.to_string(), |p| p.digits(&day.to_string()))
    }
    pub(crate) fn action(&self, action: ActionLabel) -> &'static str {
        match self.profile() {
            Some(Profile::De) => [
                "Übernehmen",
                "Zurücksetzen",
                "Vorheriger Monat",
                "Nächster Monat",
                "Verringern",
                "Erhöhen",
            ][action as usize],
            Some(Profile::Fr) => [
                "Appliquer",
                "Rétablir",
                "Mois précédent",
                "Mois suivant",
                "Diminuer",
                "Augmenter",
            ][action as usize],
            Some(Profile::Ar) => [
                "تطبيق",
                "تراجع",
                "الشهر السابق",
                "الشهر التالي",
                "إنقاص",
                "زيادة",
            ][action as usize],
            _ => [
                "Apply",
                "Revert",
                "Previous month",
                "Next month",
                "Decrease",
                "Increase",
            ][action as usize],
        }
    }
}
impl Profile {
    fn decimal(self) -> char {
        match self {
            Self::De | Self::Fr => ',',
            Self::Ar => '\u{66b}',
            _ => '.',
        }
    }
    fn group(self) -> char {
        match self {
            Self::De => '.',
            Self::Fr => '\u{202f}',
            Self::Ar => '\u{66c}',
            _ => ',',
        }
    }
    fn date_separator(self) -> char {
        if self == Self::De { '.' } else { '/' }
    }
    fn digits(self, ascii: &str) -> String {
        ascii
            .chars()
            .map(|c| {
                if self == Self::Ar && c.is_ascii_digit() {
                    char::from_u32('٠' as u32 + c as u32 - '0' as u32).unwrap()
                } else {
                    c
                }
            })
            .collect()
    }
    fn ascii_digits(self, text: &str) -> Result<String, String> {
        let mut scripts = 0u8;
        let out = text
            .chars()
            .map(|c| {
                if c.is_ascii_digit() {
                    scripts |= 1;
                    c
                } else if self == Self::Ar && ('٠'..='٩').contains(&c) {
                    scripts |= 2;
                    char::from_u32('0' as u32 + c as u32 - '٠' as u32).unwrap()
                } else {
                    c
                }
            })
            .collect();
        if scripts == 3 {
            return Err("Use one digit set throughout the value".into());
        }
        Ok(out)
    }
}
