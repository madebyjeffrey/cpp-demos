//! Exact civil-time editing; component steps preserve subsecond precision.
use crate::{Binding, Draft, EditLocale, Error, Time, Validation, ValidationStatus, View};
use crate::{
    draft::{StepperData, StepperInfo},
    locale::ActionLabel,
};
use std::rc::Rc;
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
/// Explicit clock display policy, independent of the system locale.
pub enum HourCycle {
    #[default]
    H24,
    H12,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
/// The component to adjust; minute/second steps carry into larger components.
pub enum TimePart {
    Hour,
    Minute,
    Second,
    Period,
}
#[derive(Clone)]
pub(crate) struct Clock {
    pub cycle: HourCycle,
    pub seconds: bool,
    pub minute_step: u8,
    pub second_step: u8,
    pub locale: EditLocale,
}
impl Default for Clock {
    fn default() -> Self {
        Self {
            cycle: HourCycle::H24,
            seconds: false,
            minute_step: 1,
            second_step: 1,
            locale: EditLocale::ASCII,
        }
    }
}
impl Clock {
    pub fn format(&self, time: Time) -> String {
        let hour = if self.cycle == HourCycle::H12 {
            (time.hour() + 11) % 12 + 1
        } else {
            time.hour()
        };
        let mut text = format!("{hour:02}:{:02}", time.minute());
        if self.seconds {
            text.push_str(&format!(":{:02}", time.second()));
            if time.subsec_nanosecond() != 0 {
                text.push('.');
                text.push_str(format!("{:09}", time.subsec_nanosecond()).trim_end_matches('0'));
            }
        }
        if self.cycle == HourCycle::H12 {
            text.push(' ');
            text.push_str(self.locale.periods()[(time.hour() / 12) as usize]);
        }
        self.locale.clock_digits(&text)
    }
    pub fn hint(&self) -> String {
        let periods = self.locale.periods().join("/");
        format!(
            "{} · HH:MM{}{}",
            self.locale.clock_label(),
            if self.seconds {
                self.locale.clock_digits(":SS[.fraction]")
            } else {
                String::new()
            },
            if self.cycle == HourCycle::H12 {
                format!(" {periods}")
            } else {
                String::new()
            }
        )
    }
    pub fn parse(&self, text: &str, base: Time) -> Result<Time, String> {
        let text = self.locale.clock_input(text)?;
        let (clock, period) = if self.cycle == HourCycle::H12 {
            let (clock, period) = text.split_once(' ').ok_or_else(|| self.hint())?;
            let labels = self.locale.periods();
            let period = if period.eq_ignore_ascii_case(labels[0]) {
                0
            } else if period.eq_ignore_ascii_case(labels[1]) {
                1
            } else {
                return Err(self.hint());
            };
            (clock, Some(period))
        } else {
            (text.as_str(), None)
        };
        let (hour, rest) = clock.split_once(':').ok_or_else(|| self.hint())?;
        if hour.is_empty() || hour.len() > 2 || !hour.bytes().all(|b| b.is_ascii_digit()) {
            return Err(self.hint());
        }
        let mut hour = hour.parse::<i8>().map_err(|_| self.hint())?;
        if let Some(period) = period {
            if !(1..=12).contains(&hour) {
                return Err(self.hint());
            }
            hour = hour % 12 + period * 12;
        } else if clock.len() < 5 || clock.as_bytes().get(2) != Some(&b':') {
            return Err(self.hint());
        }
        let has_seconds = rest.contains(':');
        if has_seconds && !self.seconds {
            return Err(self.hint());
        }
        let parsed = crate::values::parse_time(&format!("{hour:02}:{rest}"))?;
        if has_seconds {
            Ok(parsed)
        } else {
            Time::new(
                parsed.hour(),
                parsed.minute(),
                base.second(),
                base.subsec_nanosecond(),
            )
            .map_err(|e| e.to_string())
        }
    }
    pub(crate) fn step_seconds(&self, time: Time, part: TimePart, increase: bool) -> Option<i64> {
        if part == TimePart::Second && !self.seconds
            || part == TimePart::Period && self.cycle != HourCycle::H12
        {
            return None;
        }
        let seconds = match part {
            TimePart::Hour => 3600,
            TimePart::Minute => i64::from(self.minute_step) * 60,
            TimePart::Second => i64::from(self.second_step),
            TimePart::Period => {
                if time.hour() < 12 {
                    43200
                } else {
                    -43200
                }
            }
        };
        Some(if part == TimePart::Period || increase {
            seconds
        } else {
            -seconds
        })
    }
    pub fn step(&self, time: Time, part: TimePart, increase: bool) -> Option<Time> {
        let delta = self.step_seconds(time, part, increase)?;
        let nanos = (i64::from(time.hour()) * 3600
            + i64::from(time.minute()) * 60
            + i64::from(time.second())
            + delta)
            * 1_000_000_000
            + i64::from(time.subsec_nanosecond());
        if !(0..86_400_000_000_000).contains(&nanos) {
            return None;
        }
        let seconds = nanos / 1_000_000_000;
        Time::new(
            (seconds / 3600) as i8,
            ((seconds / 60) % 60) as i8,
            (seconds % 60) as i8,
            (nanos % 1_000_000_000) as i32,
        )
        .ok()
    }
    pub fn part(&self, time: Time, part: TimePart) -> i8 {
        match part {
            TimePart::Hour => {
                if self.cycle == HourCycle::H12 {
                    (time.hour() + 11) % 12 + 1
                } else {
                    time.hour()
                }
            }
            TimePart::Minute => time.minute(),
            TimePart::Second => time.second(),
            TimePart::Period => time.hour() / 12,
        }
    }
}
type Validator = Rc<dyn Fn(Time) -> Result<(), Error>>;
#[derive(Clone)]
struct Rules {
    clock: Clock,
    min: Time,
    max: Time,
    validate: Option<Validator>,
}
impl Rules {
    fn check(&self, time: Time) -> Validation {
        let (status, message) = if time < self.min || time > self.max {
            (
                ValidationStatus::OutOfRange,
                Some(format!(
                    "{} – {}",
                    self.clock.format(self.min),
                    self.clock.format(self.max)
                )),
            )
        } else if let Some(error) =
            crate::state::read_only(|| self.validate.as_ref().and_then(|f| f(time).err()))
        {
            (ValidationStatus::Invalid, Some(error.0))
        } else {
            (ValidationStatus::Valid, None)
        };
        Validation {
            status,
            message,
            dirty: false,
        }
    }
    fn parse(&self, text: &str, base: Time) -> Result<Time, String> {
        let time = self.clock.parse(text, base)?;
        let v = self.check(time);
        if v.status == ValidationStatus::Valid {
            Ok(time)
        } else {
            Err(v.message.unwrap())
        }
    }
}
#[derive(Clone)]
/// A transactional civil-time editor. Construct/configure once, then render with
/// `view()`. Hidden seconds and nanoseconds survive hour/minute edits. Steps reject
/// invalid drafts, bounds violations and day crossings without changing either value.
pub struct TimePicker {
    value: Binding<Time>,
    draft: Draft<Time>,
    rules: Rules,
    read_only: bool,
    commit_on_blur: bool,
    submit: Option<Rc<dyn Fn()>>,
    cancel: Option<Rc<dyn Fn()>>,
}
impl TimePicker {
    pub fn new(value: Binding<Time>) -> Self {
        let draft = crate::time_draft(value.clone());
        Self {
            value,
            draft,
            rules: Rules {
                clock: Clock::default(),
                min: Time::MIN,
                max: Time::MAX,
                validate: None,
            },
            read_only: false,
            commit_on_blur: true,
            submit: None,
            cancel: None,
        }
        .configured()
    }
    fn configured(mut self) -> Self {
        assert!(self.rules.min <= self.rules.max, "Invalid time bounds");
        let (parse, format, status, model) = (
            self.rules.clone(),
            self.rules.clock.clone(),
            self.rules.clone(),
            self.rules.clone(),
        );
        let (source, source_status) = (self.value.clone(), self.value.clone());
        let draft = Draft::new(
            self.value.clone(),
            move |s| parse.parse(s, source.get()),
            move |v| format.format(*v),
        )
        .classifiers(
            move |s| {
                if s.is_empty() {
                    ValidationStatus::Incomplete
                } else {
                    match status.clock.parse(s, source_status.get()) {
                        Ok(t) => status.check(t).status,
                        Err(_) => ValidationStatus::Invalid,
                    }
                }
            },
            move |v| model.check(*v),
        );
        self.draft = self.draft.reconfigured(draft).read_only(self.read_only);
        self
    }
    pub fn binding(&self) -> Binding<String> {
        self.draft.binding()
    }
    pub fn draft(&self) -> &Draft<Time> {
        &self.draft
    }
    pub fn validation(&self) -> Validation {
        self.draft.validation()
    }
    pub fn bounds(mut self, min: Time, max: Time) -> Self {
        self.rules.min = min;
        self.rules.max = max;
        self.configured()
    }
    pub fn hour_cycle(mut self, cycle: HourCycle) -> Self {
        self.rules.clock.cycle = cycle;
        self.configured()
    }
    pub fn show_seconds(mut self, show: bool) -> Self {
        self.rules.clock.seconds = show;
        self.configured()
    }
    pub fn minute_step(mut self, step: u8) -> Self {
        assert!((1..=60).contains(&step));
        self.rules.clock.minute_step = step;
        self
    }
    pub fn second_step(mut self, step: u8) -> Self {
        assert!((1..=60).contains(&step));
        self.rules.clock.second_step = step;
        self
    }
    pub fn locale(mut self, locale: EditLocale) -> Self {
        self.rules.clock.locale = locale;
        self.configured()
    }
    pub fn validate(mut self, validate: impl Fn(Time) -> Result<(), Error> + 'static) -> Self {
        self.rules.validate = Some(Rc::new(validate));
        self.configured()
    }
    pub fn read_only(mut self, read_only: bool) -> Self {
        self.read_only = read_only;
        self.draft = self.draft.read_only(read_only);
        self
    }
    pub fn commit_on_blur(mut self, commit: bool) -> Self {
        self.commit_on_blur = commit;
        self
    }
    pub fn on_validation(mut self, callback: impl Fn(Validation) + 'static) -> Self {
        self.draft = self.draft.on_validation(callback);
        self
    }
    pub fn on_submit(mut self, callback: impl Fn() + 'static) -> Self {
        self.submit = Some(Rc::new(callback));
        self
    }
    pub fn on_cancel(mut self, callback: impl Fn() + 'static) -> Self {
        self.cancel = Some(Rc::new(callback));
        self
    }
    pub fn commit(&self) -> bool {
        crate::state::batch(|| {
            self.draft.commit() && self.validation().status == ValidationStatus::Valid
        })
    }
    pub fn cancel(&self) {
        self.draft.cancel();
        if let Some(callback) = &self.cancel {
            callback()
        }
    }
    fn submit(&self) {
        if self.commit()
            && let Some(callback) = &self.submit
        {
            callback()
        }
    }
    fn current(&self) -> Option<Time> {
        crate::state::read_only(|| {
            self.rules
                .parse(&self.binding().get(), self.value.get())
                .ok()
        })
    }
    fn next(&self, part: TimePart, increase: bool) -> Option<Time> {
        if self.read_only {
            return None;
        }
        let current = self.current()?;
        let next = self.rules.clock.step(current, part, increase)?;
        (next != current && self.rules.check(next).status == ValidationStatus::Valid)
            .then_some(next)
    }
    pub fn can_step(&self, part: TimePart, increase: bool) -> bool {
        self.next(part, increase).is_some()
    }
    pub fn step(&self, part: TimePart, increase: bool) -> bool {
        crate::state::batch(|| {
            let Some(next) = self.next(part, increase) else {
                return false;
            };
            self.draft.accept(next)
        })
    }
    fn parts(&self) -> View {
        let (read, can, change) = (self.clone(), self.clone(), self.clone());
        ClockControls {
            clock: self.rules.clock.clone(),
            read: Rc::new(move || read.current().unwrap_or_else(|| read.value.get())),
            can: Rc::new(move |part, up| can.can_step(part, up)),
            change: Rc::new(move |part, up| change.step(part, up)),
            read_only: self.read_only,
        }
        .view()
    }
    pub fn view(&self) -> View {
        let (submit, cancel, blur, apply, revert) = (
            self.clone(),
            self.clone(),
            self.clone(),
            self.clone(),
            self.clone(),
        );
        let mut children = vec![
            View::label(self.rules.clock.hint()),
            View::text_field(self.rules.clock.hint(), self.binding())
                .key("$time-field")
                .read_only(self.read_only)
                .accessible_label(self.rules.clock.hint())
                .validation(self.validation())
                .on_submit(move || submit.submit())
                .on_cancel(move || cancel.cancel())
                .on_blur(move || {
                    if blur.commit_on_blur {
                        blur.commit();
                    }
                }),
            self.parts(),
            View::row(vec![
                View::button(
                    self.rules.clock.locale.action(ActionLabel::Apply),
                    move || apply.submit(),
                )
                .enabled(!self.read_only),
                View::button(
                    self.rules.clock.locale.action(ActionLabel::Revert),
                    move || revert.cancel(),
                )
                .enabled(!self.read_only),
            ]),
        ];
        if let Some(error) = self.draft.error() {
            children.push(View::label(error).wrap(true).foreground(0xffff8787));
        }
        View::panel(children).accessible_label(self.rules.clock.locale.clock_label())
    }
}

// Shared rendering; each owner supplies whole-candidate validation and arithmetic.
#[derive(Clone)]
pub(crate) struct ClockControls {
    pub clock: Clock,
    pub read: Rc<dyn Fn() -> Time>,
    pub can: Rc<dyn Fn(TimePart, bool) -> bool>,
    pub change: Rc<dyn Fn(TimePart, bool) -> bool>,
    pub read_only: bool,
}
impl ClockControls {
    fn part_view(&self, part: TimePart) -> View {
        let index = match part {
            TimePart::Hour => 0,
            TimePart::Minute => 1,
            TimePart::Second => 2,
            TimePart::Period => unreachable!(),
        };
        let label = self.clock.locale.clock_part(index);
        let (down, up, read, change) = (self.clone(), self.clone(), self.clone(), self.clone());
        let mut stepper = View::row(vec![
            View::button("−", move || {
                (down.change)(part, false);
            })
            .enabled((self.can)(part, false))
            .accessible_label(format!(
                "{} {label}",
                self.clock.locale.action(ActionLabel::Decrease)
            ))
            .width(34),
            View::button("+", move || {
                (up.change)(part, true);
            })
            .enabled((self.can)(part, true))
            .accessible_label(format!(
                "{} {label}",
                self.clock.locale.action(ActionLabel::Increase)
            ))
            .width(34),
        ])
        .gap(4.)
        .width(72)
        .shrink(0.)
        .read_only(self.read_only)
        .accessible_label(label);
        let data = Rc::make_mut(&mut stepper.0);
        data.kind = crate::Kind::Stepper;
        data.stepper = Some(StepperData {
            read: Rc::new(move || {
                let time = (read.read)();
                let (min, max) = if part == TimePart::Hour {
                    if read.clock.cycle == HourCycle::H12 {
                        (1, 12)
                    } else {
                        (0, 23)
                    }
                } else {
                    (0, 59)
                };
                StepperInfo {
                    value: read.clock.part(time, part).to_string(),
                    min: min.to_string(),
                    max: max.to_string(),
                    can_increment: (read.can)(part, true),
                    can_decrement: (read.can)(part, false),
                }
            }),
            change: Rc::new(move |increase| (change.change)(part, increase)),
        });
        let time = (self.read)();
        View::column(vec![
            View::label(format!(
                "{label} · {}",
                self.clock
                    .locale
                    .clock_digits(&format!("{:02}", self.clock.part(time, part)))
            )),
            stepper,
        ])
        .grow(1.)
        .min_width(0.)
    }
    pub fn view(&self) -> View {
        let mut parts = vec![
            self.part_view(TimePart::Hour),
            self.part_view(TimePart::Minute),
        ];
        if self.clock.seconds {
            parts.push(self.part_view(TimePart::Second));
        }
        if self.clock.cycle == HourCycle::H12 {
            let read = self.clone();
            let write = self.clone();
            let period = Binding::new(
                move || {
                    if (read.read)().hour() < 12 {
                        "am".into()
                    } else {
                        "pm".into()
                    }
                },
                move |v: String| {
                    let time = (write.read)();
                    if (write.can)(TimePart::Period, true)
                        && (v == "pm" && time.hour() < 12 || v == "am" && time.hour() >= 12)
                    {
                        (write.change)(TimePart::Period, true);
                    }
                },
            );
            let labels = self.clock.locale.periods();
            parts.push(
                View::segmented(
                    period,
                    vec![
                        crate::OptionItem::new("am", labels[0]),
                        crate::OptionItem::new("pm", labels[1]),
                    ],
                )
                .enabled(!self.read_only && (self.can)(TimePart::Period, true))
                .accessible_label(labels.join("/")),
            );
        }
        View::row(parts).gap(12.).key("$time-parts")
    }
}
