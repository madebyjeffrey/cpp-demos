# Editable value formats

M6 editable-format contract. ASCII/ISO editing remains the default. `EditLocale` selects a
checked locale profile shared by `Numeric`, `DatePicker`, `TimePicker` and `DateTimePicker`. Read-only display
formatters may support more locales than editable controls. A display formatter does not define an input grammar.

## Default grammar

| Value | Accepted edit | Rejected rather than inferred/discarded |
| --- | --- | --- |
| Integer | ASCII digits, optional sign; exact native integer range | Grouping, decimal points, exponents, whitespace, localized digits, overflow |
| Finite decimal | ASCII digits, `.`, optional sign and `e`/`E` exponent; `.5` and `12.` allowed | Grouping, localized separators/digits, NaN/infinity, whitespace |
| Gregorian date | Canonical ISO date produced by `Date::to_string()`; four-digit ordinary years, signed extended years at the supported extremes | Numeric locale dates, two-digit years, timestamp/offset/zone suffixes |
| Civil time | `HH:MM[:SS[.fraction]]`, 24-hour; 1–9 fractional digits after seconds | Dates, offsets/zones, leap seconds, excess precision, 24:00, AM/PM |
| Civil datetime | ISO date, `T`, civil time above | Offsets/zones; no discarded instant information |
| Zoned datetime | Civil datetime in the current named zone, or ISO datetime with one bracketed zone (IANA name or explicit fixed offset) and optional matching offset | Bare offsets/Z without a bracketed zone, discarded suffixes, extra calendar annotations, leap-second clamping |
| sRGB color | `#RRGGBB` or `#RRGGBBAA`, case insensitive; output includes alpha | Named/CSS colors, shorthand, inferred channel order |

Date/time validity and named-zone resolution use Jiff. Zone gaps/overlaps reject by
default; choosing earlier/later is explicit. Invalid edits preserve both the draft
and committed value. Apply/Enter validates; Revert restores the model. Numeric
incomplete/invalid/out-of-range classification and readonly/blur policy already
exist. Date editors now provide bounds, disabled/custom validation, readonly/blur/
submit/cancel policy and a noncommitting keyboard cursor. Unifying those policies
for every typed value remains M6.6.

Integer parse/format/step never passes through floating point. Decimal precision is
an explicit display option; an untouched rounded display cannot overwrite the exact
model. Canonical date/time output round-trips at the supported extremes and retains
nanoseconds. Generic `Draft::new` still permits application-defined grammars.

## Explicit locale profiles

Configure the profile before mounting. Changing a builder's locale/precision resets
its draft to the committed value; it is not a live locale-switch operation. The
supported profiles are intentionally finite and testable:

| Profile | Decimal / grouping | Digits | Date edit |
| --- | --- | --- | --- |
| `en-US` | `.` / `,` | ASCII | `MM/DD/YYYY` |
| `en-GB` | `.` / `,` | ASCII | `DD/MM/YYYY` |
| `de-DE` | `,` / `.` | ASCII | `DD.MM.YYYY` |
| `fr-FR` | `,` / narrow no-break space (U+202F) | ASCII | `DD/MM/YYYY` |
| `ar-EG` | Arabic decimal U+066B / grouping U+066C | Arabic-Indic | `DD/MM/YYYY` in the selected digits |

These are advertised editing patterns with four-digit years, not a claim to cover
every CLDR date pattern/calendar. Calendar labels and hints must follow the chosen
profile. The time and civil datetime controls add the explicit clock policy below;
legacy draft helpers retain ISO. BCE/extended-year editing remains available in ISO mode.

- Validate profile names at construction. Unknown/malformed profiles and unsupported
  calendar/numbering extensions return an error; never silently substitute an edit
  grammar. ASCII/ISO stays an explicit choice independent of OS settings.
- ICU4X supplies decimal output/data. Match that output to the advertised parser
  grammar and reject unsupported data/configuration. Do not translate a localized
  string through `f64` to obtain an integer. The existing display-only
  `IntegerFormatter` keeps its documented ICU locale fallback.
- Grouping is optional. When present, require complete groups of three in the
  integer part and the profile's exact separator; never strip arbitrary punctuation
  or spaces. Ungrouped input may have any supported digit length.
- A profile determines decimal meaning: `1,234` means 1234 in `en-US` and 1.234 in
  `de-DE`; malformed grouping fails. Never retry using a different locale.
- Accept the selected digit set or ASCII digits with the profile's separators for
  practical keyboards; reject mixed digit sets throughout the value, including the
  exponent. Arabic editing therefore accepts `1234٫5`, but not `1234.5`. ASCII signs
  and `e`/`E` scientific notation remain available, with no grouping in fractions or exponents. Output must parse
  back under the same profile, including signed values and rounded decimal display.
- Localized date input uses only its stated field order, separators and valid
  Gregorian dates. No month/day guessing, two-digit year pivot or timezone inference.

Sources: the [ICU4X decimal interface](https://docs.rs/icu_decimal/2.4.0/icu_decimal/)
provides formatting; this document defines the stricter SCARS editing policy. Jiff's
[Temporal parser source](https://github.com/BurntSushi/jiff/blob/0.2.39/src/fmt/temporal/parser.rs)
explains why the civil editor checks consumption and rejects leap-second clamping.

ICU output uses the exact sign affixes supplied by its compiled data, including
Arabic Letter Mark where applicable. The parser accepts those affixes and ASCII
signs; it does not strip arbitrary directional controls. Scientific output keeps
`e` and its ASCII sign, with the selected digit set in the exponent. Construction
checks representative ICU output against the advertised grammar; unsupported data
is an error. Gregorian calendar labels use the explicitly supported profile's
month/week/action text; US weeks start Sunday, Arabic weeks Saturday, and other
profiles Monday. The Arabic grid progresses from right to left.

```rust
let locale = EditLocale::new("de-DE")?;
let amount = Numeric::new(value.binding(), -1000., 1000., 0.5).locale(locale.clone());
let calendar = DatePicker::new(date.binding()).locale(locale);
```

Application labels and other UI text remain application-owned. This is a finite
editing-profile implementation, not a general message-catalog or CLDR pattern engine.
No additional dependencies or C++/ICU4C component code are needed. The existing
Rust ICU4X decimal formatter supplies numeric output; Jiff validates Gregorian dates.
Localized years are 0001–9999; unsupported model years remain visibly invalid and
calendar choices cannot commit them. ISO mode preserves Jiff's full year range.

Run `cargo run --example m6_locales --locked --offline` from `rust/`. Use
`--locale ar-EG` (or another profile), `--light`, `--render PATH`, or `--smoke`.
`m6_formats` retains the ASCII/ISO lab. Four default-format tests and six localization
tests cover grammar, precision, grouping/digits, stepping, date order, unsupported
profiles, extreme values, calendar layout/actions and external value replacement.
The opt-in native `localized_values` harness tests actual text/AX transport; it does
not replace the pending interactive VoiceOver/IME audit.

## Standalone time control

`TimePicker` defaults to a 24-hour `HH:MM` field. `hour_cycle(HourCycle::H12)`
uses `hh:MM AM/PM`, accepts one- or two-digit hours 1–12 and case-insensitive English
period markers. Arabic profiles use `ص`/`م`; the period follows one ASCII space.
Midnight is 12 AM and noon 12 PM. Locale does not implicitly select the hour cycle.

`show_seconds(true)` adds `:SS` and, when nonzero, 1–9 fractional digits. The
fraction separator matches the profile's decimal separator; time components always
use `:`. Selected digits or ASCII digits are accepted consistently across the whole
value; mixed digit sets fail. No grouping, whitespace, leap seconds, offsets, dates,
zones or day wrapping is inferred. The field's hint advertises its configured grammar.

Omitting seconds preserves the source's exact seconds and nanoseconds. Supplying
seconds replaces both; omitted fractional digits mean zero nanoseconds. Hidden
seconds cannot be typed until enabled. An untouched display and hour/minute-only
edits therefore retain subsecond precision. Pure bounds/custom validation checks
the entire candidate, including hidden precision. Stepping uses exact arithmetic,
carries into larger components and rejects crossing midnight or configured bounds.
Minute/second steps are 1–60; AM/PM explicitly toggles 12 hours within the same day.

Configure clock/locale/bounds/validation builders before mounting; changing them
restores the committed draft. Readonly/blur flags can vary on view clones. Apply or
Enter commits and submits; blur commits by default without submitting, while Revert
or Escape restores the model. Invalid text and rejected setters keep the draft.
External source revisions invalidate stale edits, including A→B→A transitions.

`cargo run --example m6_time --locked --offline` demonstrates two clock policies
over one exact value. Six portable tests and the hidden native `time_picker` harness
cover precision, localized grammar, steps, validation, lifecycle and text composition.
Zoned instant bounds and timezone policy remain open under M6.4.

## Combined civil datetime control

`DateTimePicker` uses the chosen date grammar, a literal `T`, then the same clock
grammar as `TimePicker`. For example, German 12-hour editing with seconds uses
`29.02.2024T01:02:03,123 PM`. Digit-set consistency applies across the whole value.
The default is ISO date plus 24-hour `HH:MM`; hidden seconds/nanoseconds are retained.
Offsets, named zones and timestamp suffixes are rejected by this civil editor.

One full-value field owns the draft. Editing the date and time across tight joint
bounds does not write intermediate application values. Apply/Enter or default blur
publishes only a complete, valid candidate; cancel restores the whole value.
Calendar choices retain the staged clock and commit only a valid whole candidate.
Clock steppers commit a valid draft plus the adjustment, with exact carry through
midnight/leap/year boundaries. Invalid syntax disables both; invalid whole-candidate
validation disables clock stepping. Calendar choices may repair a date that violates
joint bounds, provided the new whole candidate passes every rule. Nothing clamps
an invalid committed model or silently discards uncommitted text.

Inclusive datetime bounds, disabled-date predicates and custom validators check
the whole value on every commit/choice/step. Date and clock configuration is shared,
including locale, 12/24-hour mode, optional seconds, step sizes and readonly policy.
`m6_datetime` demonstrates an overnight booking range. The local editor does not
resolve timezones; instant limits, DST decisions and named-zone changes remain M6.4.
