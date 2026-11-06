# Editable value formats

M6.1 contract. ASCII/ISO editing is implemented and remains the default. M6.2 adds
explicit locale profiles; the table below specifies that next slice, not currently
available localized controls. Read-only display formatters may support more locales
than editable controls. A display formatter does not define an input grammar.

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
exist; unifying those policies for every typed value is M6.6.

Integer parse/format/step never passes through floating point. Decimal precision is
an explicit display option; an untouched rounded display cannot overwrite the exact
model. Canonical date/time output round-trips at the supported extremes and retains
nanoseconds. Generic `Draft::new` still permits application-defined grammars.

## Explicit locale profiles for M6.2

Use one checked locale setting shared by numbers and Gregorian date editors. The
initial supported profiles are intentionally finite and testable:

| Profile | Decimal / grouping | Digits | Date edit |
| --- | --- | --- | --- |
| `en-US` | `.` / `,` | ASCII | `MM/DD/YYYY` |
| `en-GB` | `.` / `,` | ASCII | `DD/MM/YYYY` |
| `de-DE` | `,` / `.` | ASCII | `DD.MM.YYYY` |
| `fr-FR` | `,` / narrow no-break space (U+202F) | ASCII | `DD/MM/YYYY` |
| `ar-EG` | Arabic decimal U+066B / grouping U+066C | Arabic-Indic | `DD/MM/YYYY` in the selected digits |

These are advertised editing patterns with four-digit years, not a claim to cover
every CLDR date pattern/calendar. Calendar labels and hints must follow the chosen
profile. Time/datetime controls retain ISO until their explicit hour-cycle policy
is implemented in M6.4. BCE/extended-year editing remains available in ISO mode.

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
- Accept the selected digit set or an entirely ASCII numeric entry for practical
  keyboards; reject mixed digit sets. ASCII signs and `e`/`E` scientific notation
  remain available, with no grouping in fractions or exponents. Output must parse
  back under the same profile, including signed values and rounded decimal display.
- Localized date input uses only its stated field order, separators and valid
  Gregorian dates. No month/day guessing, two-digit year pivot or timezone inference.

Sources: the [ICU4X decimal interface](https://docs.rs/icu_decimal/2.4.0/icu_decimal/)
provides formatting; this document defines the stricter SCARS editing policy. Jiff's
[Temporal parser source](https://github.com/BurntSushi/jiff/blob/0.2.39/src/fmt/temporal/parser.rs)
explains why the civil editor checks consumption and rejects leap-second clamping.

Example: `cargo run --example m6_formats --locked --offline` from `rust/`.
Four focused tests in `components/tests/editable_formats.rs` cover exact integers,
ambiguous inputs, extreme date/time round trips, suffix rejection and named zones.
