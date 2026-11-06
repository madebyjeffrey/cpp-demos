//! Instant-backed civil editing with explicit display zone and DST decisions.
use crate::{
    Binding, CalendarInfo, Date, DateTime, DateTimePicker, EditLocale, Error, HourCycle,
    OptionItem, State, TimePart, TimeZone, Timestamp, Validation, ValidationStatus, View, Zoned,
};
use jiff::tz::AmbiguousOffset;
use std::rc::Rc;

/// Policy for wall times repeated during an offset transition. Gaps always fail.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RepeatedTimePolicy {
    #[default]
    Reject,
    Earlier,
    Later,
}
/// What a successful explicit display-zone change preserves.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ZoneChange {
    #[default]
    KeepInstant,
    KeepLocal,
}
type Validator = Rc<dyn Fn(&Zoned) -> Result<(), Error>>;
fn zone(name: &str) -> Result<TimeZone, Error> {
    let zone = jiff::fmt::temporal::DateTimeParser::new()
        .parse_time_zone(name)
        .map_err(|e| Error(e.to_string()))?;
    if zone.iana_name().is_none() && zone.to_fixed_offset().is_err() {
        return Err(Error("Use an IANA zone or a fixed offset".into()));
    }
    Ok(zone)
}
fn name(zone: &TimeZone) -> String {
    zone.iana_name()
        .map(str::to_owned)
        .unwrap_or_else(|| zone.to_fixed_offset().unwrap().to_string())
}
fn status(status: ValidationStatus, message: Option<String>) -> Validation {
    Validation {
        status,
        message,
        dirty: false,
    }
}
#[derive(Clone)]
struct Context {
    source: Binding<Timestamp>,
    zone: State<TimeZone>,
    repeated: State<RepeatedTimePolicy>,
    min: Option<Timestamp>,
    max: Option<Timestamp>,
    validate: Option<Validator>,
}
impl Context {
    fn zoned(&self) -> Zoned {
        self.source.get().to_zoned(self.zone.get())
    }
    fn resolve(
        &self,
        local: DateTime,
        zone: &TimeZone,
        policy: RepeatedTimePolicy,
        known: bool,
    ) -> Result<Timestamp, Error> {
        let source = self.source.get();
        if known && source.to_zoned(zone.clone()).datetime() == local {
            return Ok(source);
        }
        let ambiguous = zone.to_ambiguous_timestamp(local);
        let resolved = match ambiguous.offset() {
            AmbiguousOffset::Gap { .. } => {
                return Err(Error(
                    "This wall time does not exist in the selected zone".into(),
                ));
            }
            AmbiguousOffset::Fold { .. } => match policy {
                RepeatedTimePolicy::Reject => {
                    return Err(Error(
                        "This wall time occurs twice; choose earlier or later".into(),
                    ));
                }
                RepeatedTimePolicy::Earlier => ambiguous.earlier(),
                RepeatedTimePolicy::Later => ambiguous.later(),
            },
            _ => ambiguous.unambiguous(),
        };
        resolved.map_err(|e| Error(e.to_string()))
    }
    fn instant_check(&self, instant: Timestamp, zone: &TimeZone) -> Validation {
        if self.min.is_some_and(|min| instant < min) || self.max.is_some_and(|max| instant > max) {
            return status(
                ValidationStatus::OutOfRange,
                Some(format!(
                    "Instant bounds: {} – {}",
                    self.min
                        .map_or_else(|| "unbounded".into(), |v| v.to_string()),
                    self.max
                        .map_or_else(|| "unbounded".into(), |v| v.to_string())
                )),
            );
        }
        if let Some(error) = crate::state::read_only(|| {
            self.validate
                .as_ref()
                .and_then(|f| f(&instant.to_zoned(zone.clone())).err())
        }) {
            return status(ValidationStatus::Invalid, Some(error.0));
        }
        status(ValidationStatus::Valid, None)
    }
    fn check(&self, local: DateTime) -> Validation {
        let zone = self.zone.get();
        match self.resolve(local, &zone, self.repeated.get(), true) {
            Ok(instant) => self.instant_check(instant, &zone),
            Err(e) => status(ValidationStatus::Invalid, Some(e.0)),
        }
    }
}

/// One civil draft editing an instant in an explicit zone. Applications bind a
/// `Timestamp`, so zone-only changes cannot disappear as equal `Zoned` writes.
#[derive(Clone)]
pub struct ZonedPicker {
    context: Context,
    local: DateTimePicker,
    zone_input: State<String>,
    mode: State<ZoneChange>,
    error: State<Option<String>>,
    read_only: bool,
}
impl ZonedPicker {
    pub fn new(value: Binding<Timestamp>, zone_name: &str) -> Result<Self, Error> {
        let zone = zone(zone_name)?;
        let context = Context {
            source: value,
            zone: State::new(zone),
            repeated: State::new(RepeatedTimePolicy::Reject),
            min: None,
            max: None,
            validate: None,
        };
        let (read, write, revision) = (context.clone(), context.clone(), context.clone());
        let binding = Binding::new(
            move || read.zoned().datetime(),
            move |local| {
                let zone = write.zone.get();
                if let Ok(instant) = write.resolve(local, &zone, write.repeated.get(), true) {
                    write.source.set(instant);
                }
            },
        )
        .with_revision(move || {
            revision
                .source
                .revision()
                .unwrap_or(0)
                .wrapping_add(revision.zone.version())
        });
        let local = DateTimePicker::new(binding);
        let picker = Self {
            context,
            local,
            zone_input: State::new(zone_name.into()),
            mode: State::new(ZoneChange::KeepInstant),
            error: State::new(None),
            read_only: false,
        };
        Ok(picker.configured())
    }
    fn configured(mut self) -> Self {
        let context = self.context.clone();
        self.local = self.local.external_check(move |value| context.check(value));
        self
    }
    pub fn bounds(mut self, min: Timestamp, max: Timestamp) -> Self {
        assert!(min <= max, "Invalid instant bounds");
        self.context.min = Some(min);
        self.context.max = Some(max);
        self.configured()
    }
    pub fn civil_bounds(mut self, min: DateTime, max: DateTime) -> Self {
        self.local = self.local.bounds(min, max);
        self
    }
    pub fn hour_cycle(mut self, cycle: HourCycle) -> Self {
        self.local = self.local.hour_cycle(cycle);
        self
    }
    pub fn show_seconds(mut self, show: bool) -> Self {
        self.local = self.local.show_seconds(show);
        self
    }
    pub fn minute_step(mut self, step: u8) -> Self {
        self.local = self.local.minute_step(step);
        self
    }
    pub fn second_step(mut self, step: u8) -> Self {
        self.local = self.local.second_step(step);
        self
    }
    pub fn locale(mut self, locale: EditLocale) -> Self {
        self.local = self.local.locale(locale);
        self
    }
    pub fn disabled_dates(mut self, disabled: impl Fn(Date) -> bool + 'static) -> Self {
        self.local = self.local.disabled_dates(disabled);
        self
    }
    pub fn validate(mut self, validate: impl Fn(&Zoned) -> Result<(), Error> + 'static) -> Self {
        self.context.validate = Some(Rc::new(validate));
        self.configured()
    }
    pub fn repeated_time(self, policy: RepeatedTimePolicy) -> Self {
        self.context.repeated.set(policy);
        self
    }
    pub fn repeated_time_policy(&self) -> RepeatedTimePolicy {
        self.context.repeated.get()
    }
    /// Revalidate the existing text under a new fold policy without resetting it.
    pub fn set_repeated_time(&self, policy: RepeatedTimePolicy) -> bool {
        if self.read_only {
            return false;
        }
        crate::state::batch(|| {
            self.context.repeated.set(policy);
            self.error.set(None);
        });
        true
    }
    pub fn read_only(mut self, value: bool) -> Self {
        self.read_only = value;
        self.local = self.local.read_only(value);
        self
    }
    pub fn commit_on_blur(mut self, value: bool) -> Self {
        self.local = self.local.commit_on_blur(value);
        self
    }
    pub fn on_validation(mut self, f: impl Fn(Validation) + 'static) -> Self {
        self.local = self.local.on_validation(f);
        self
    }
    pub fn on_submit(mut self, f: impl Fn() + 'static) -> Self {
        self.local = self.local.on_submit(f);
        self
    }
    pub fn on_cancel(mut self, f: impl Fn() + 'static) -> Self {
        self.local = self.local.on_cancel(f);
        self
    }
    pub fn binding(&self) -> Binding<String> {
        self.local.binding()
    }
    pub fn validation(&self) -> Validation {
        self.local.validation()
    }
    pub fn commit(&self) -> bool {
        self.local.commit()
    }
    pub fn cancel(&self) {
        crate::state::batch(|| {
            self.zone_input.set(name(&self.context.zone.get()));
            self.error.set(None);
            self.local.cancel();
        })
    }
    pub fn step(&self, part: TimePart, increase: bool) -> bool {
        self.local.step(part, increase)
    }
    pub fn can_step(&self, part: TimePart, increase: bool) -> bool {
        self.local.can_step(part, increase)
    }
    pub fn select(&self, date: Date) -> bool {
        self.local.select(date)
    }
    pub fn calendar(&self) -> CalendarInfo {
        self.local.calendar()
    }
    pub fn zoned(&self) -> Zoned {
        self.context.zoned()
    }
    pub fn time_zone(&self) -> TimeZone {
        self.context.zone.get()
    }
    fn candidate(&self) -> Result<DateTime, Error> {
        self.local.raw_candidate().ok_or_else(|| {
            Error(
                self.validation()
                    .message
                    .unwrap_or_else(|| "Complete the datetime first".into()),
            )
        })
    }
    fn publish(&self, instant: Timestamp, zone: TimeZone) -> Result<(), Error> {
        if self.read_only {
            return Err(Error("This control is read only".into()));
        }
        let civil = self
            .local
            .civil_check(instant.to_zoned(zone.clone()).datetime());
        let validation = if civil.status == ValidationStatus::Valid {
            self.context.instant_check(instant, &zone)
        } else {
            civil
        };
        if validation.status != ValidationStatus::Valid {
            return Err(Error(validation.message.unwrap()));
        }
        let version = self.context.zone.version();
        if self.context.source.get() != instant {
            self.context.source.set(instant);
        }
        if self.context.source.get() != instant {
            return Err(Error("The application rejected the instant".into()));
        }
        if self.context.zone.version() != version {
            return Err(Error("The application superseded the zone change".into()));
        }
        self.context.zone.set(zone.clone());
        self.zone_input.set(name(&zone));
        self.error.set(None);
        let current = self.local.reset_draft();
        if current && self.context.source.get() == instant && self.context.zone.get() == zone {
            Ok(())
        } else {
            Err(Error("The application superseded the change".into()))
        }
    }
    /// Explicitly apply an occurrence. Otherwise an unchanged fold keeps its known instant.
    pub fn choose_occurrence(&self, policy: RepeatedTimePolicy) -> Result<(), Error> {
        crate::state::batch(|| {
            let local = self.candidate()?;
            let zone = self.context.zone.get();
            let instant = self.context.resolve(local, &zone, policy, false)?;
            self.publish(instant, zone)
        })
    }
    /// Commit a valid candidate while changing zone. Failures retain the draft;
    /// normalized/reentrant application writes remain authoritative.
    pub fn change_zone(&self, zone_name: &str, mode: ZoneChange) -> Result<(), Error> {
        crate::state::batch(|| {
            let target = zone(zone_name)?;
            let validation = self.validation();
            if validation.status != ValidationStatus::Valid {
                return Err(Error(validation.message.unwrap()));
            }
            let local = self.candidate()?;
            let instant = match mode {
                ZoneChange::KeepInstant => self.context.resolve(
                    local,
                    &self.context.zone.get(),
                    self.context.repeated.get(),
                    true,
                )?,
                ZoneChange::KeepLocal => {
                    self.context
                        .resolve(local, &target, self.context.repeated.get(), true)?
                }
            };
            self.publish(instant, target)
        })
    }
    fn apply_zone_input(&self) {
        if let Err(e) = self.change_zone(&self.zone_input.get(), self.mode.get()) {
            self.error.set(Some(e.0));
        }
    }
    pub fn view(&self) -> View {
        let (zone_read, zone_write, zone_error) = (
            self.zone_input.clone(),
            self.zone_input.clone(),
            self.error.clone(),
        );
        let zone_binding = Binding::new(
            move || zone_read.get(),
            move |value| {
                crate::state::batch(|| {
                    zone_write.set(value);
                    zone_error.set(None);
                })
            },
        );
        let zone_message = zone(&self.zone_input.get())
            .err()
            .map(|e| e.0)
            .or_else(|| self.error.get());
        let zone_validation = Validation {
            status: if zone_message.is_some() {
                ValidationStatus::Invalid
            } else {
                ValidationStatus::Valid
            },
            message: zone_message,
            dirty: self.zone_input.get() != name(&self.context.zone.get()),
        };
        let (
            zone_action,
            zone_submit,
            occurrence,
            policy_read,
            policy_write,
            mode_read,
            mode_write,
        ) = (
            self.clone(),
            self.clone(),
            self.clone(),
            self.clone(),
            self.clone(),
            self.mode.clone(),
            self.mode.clone(),
        );
        let policy = Binding::new(
            move || format!("{:?}", policy_read.repeated_time_policy()),
            move |value: String| {
                policy_write.set_repeated_time(match value.as_str() {
                    "Earlier" => RepeatedTimePolicy::Earlier,
                    "Later" => RepeatedTimePolicy::Later,
                    _ => RepeatedTimePolicy::Reject,
                });
            },
        );
        let mode = Binding::new(
            move || format!("{:?}", mode_read.get()),
            move |value: String| {
                mode_write.set(if value == "KeepLocal" {
                    ZoneChange::KeepLocal
                } else {
                    ZoneChange::KeepInstant
                })
            },
        );
        let mut children = vec![
            View::label(format!(
                "{} · offset {}",
                name(&self.context.zone.get()),
                self.zoned().offset()
            ))
            .wrap(true),
            View::text_field("IANA zone or fixed offset", zone_binding)
                .key("$zone-field")
                .accessible_label("Time zone")
                .validation(zone_validation)
                .read_only(self.read_only)
                .on_submit(move || zone_submit.apply_zone_input()),
            View::segmented(
                mode,
                vec![
                    OptionItem::new("KeepInstant", "Keep instant"),
                    OptionItem::new("KeepLocal", "Keep wall time"),
                ],
            )
            .enabled(!self.read_only)
            .accessible_label("Zone change"),
            View::button("Change zone", move || zone_action.apply_zone_input())
                .enabled(!self.read_only),
            View::segmented(
                policy,
                vec![
                    OptionItem::new("Reject", "Reject repeat"),
                    OptionItem::new("Earlier", "Earlier"),
                    OptionItem::new("Later", "Later"),
                ],
            )
            .enabled(!self.read_only)
            .accessible_label("Repeated wall time"),
        ];
        let fold = self.local.raw_candidate().is_some_and(|local| {
            matches!(
                self.context
                    .zone
                    .get()
                    .to_ambiguous_timestamp(local)
                    .offset(),
                AmbiguousOffset::Fold { .. }
            )
        });
        if fold {
            children.push(
                View::button("Apply occurrence", move || {
                    if let Err(e) = occurrence.choose_occurrence(occurrence.repeated_time_policy())
                    {
                        occurrence.error.set(Some(e.0));
                    }
                })
                .enabled(
                    !self.read_only && self.repeated_time_policy() != RepeatedTimePolicy::Reject,
                ),
            );
        }
        if let Some(error) = self.error.get() {
            children.push(View::label(error).wrap(true).foreground(0xffff8787));
        }
        children.push(self.local.view().key("$zoned-civil"));
        View::column(children).accessible_label("Zoned datetime")
    }
}
