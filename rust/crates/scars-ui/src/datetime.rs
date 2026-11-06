//! One civil datetime draft, with a calendar and exact clock component controls.
use crate::{
    Binding, CalendarInfo, Date, DatePicker, DateTime, Draft, EditLocale, Error, HourCycle,
    TimePart, Validation, ValidationStatus, View,
    locale::{ActionLabel, DateProblem},
    time::{Clock, ClockControls},
};
use std::rc::Rc;
type Validator = Rc<dyn Fn(DateTime) -> Result<(), Error>>;
type Disabled = Rc<dyn Fn(Date) -> bool>;
type ExternalCheck = Rc<dyn Fn(DateTime) -> Validation>;

#[derive(Clone)]
struct Rules {
    clock: Clock,
    min: DateTime,
    max: DateTime,
    disabled: Option<Disabled>,
    validate: Option<Validator>,
    external: Option<ExternalCheck>,
}
impl Rules {
    fn format(&self, value: DateTime) -> String {
        format!(
            "{}T{}",
            self.clock.locale.date(value.date()),
            self.clock.format(value.time())
        )
    }
    fn hint(&self) -> String {
        format!(
            "{} · {}T{}",
            self.clock.locale.datetime_label(),
            self.clock.locale.date_hint().split_once(" · ").unwrap().1,
            self.clock.hint().split_once(" · ").unwrap().1
        )
    }
    fn raw(&self, text: &str, base: DateTime) -> Result<DateTime, String> {
        self.clock.locale.check_digits(text)?;
        let (date, time) = text.split_once('T').ok_or_else(|| self.hint())?;
        let date = self.clock.locale.date_input(date)?;
        Ok(date.to_datetime(self.clock.parse(time, base.time())?))
    }
    fn civil_check(&self, value: DateTime) -> Validation {
        let locale = &self.clock.locale;
        let (status, message) = if !locale.date_supported(value.date()) {
            (
                ValidationStatus::Invalid,
                Some(locale.date_problem(DateProblem::UnsupportedYear).into()),
            )
        } else if value < self.min || value > self.max {
            (
                ValidationStatus::OutOfRange,
                Some(format!(
                    "{} – {}",
                    self.format(self.min),
                    self.format(self.max)
                )),
            )
        } else {
            crate::state::read_only(|| {
                if self.disabled.as_ref().is_some_and(|f| f(value.date())) {
                    (
                        ValidationStatus::Invalid,
                        Some(locale.date_problem(DateProblem::Unavailable).into()),
                    )
                } else if let Some(e) = self.validate.as_ref().and_then(|f| f(value).err()) {
                    (ValidationStatus::Invalid, Some(e.0))
                } else {
                    (ValidationStatus::Valid, None)
                }
            })
        };
        Validation {
            status,
            message,
            dirty: false,
        }
    }
    fn check(&self, value: DateTime) -> Validation {
        let civil = self.civil_check(value);
        if civil.status == ValidationStatus::Valid
            && let Some(check) = &self.external
        {
            crate::state::read_only(|| check(value))
        } else {
            civil
        }
    }
    fn parse(&self, text: &str, base: DateTime) -> Result<DateTime, String> {
        let value = self.raw(text, base).map_err(|error| {
            if self.incomplete(text) {
                self.hint()
            } else {
                error
            }
        })?;
        let status = self.check(value);
        if status.status == ValidationStatus::Valid {
            Ok(value)
        } else {
            Err(status.message.unwrap())
        }
    }
    fn incomplete(&self, text: &str) -> bool {
        let locale = &self.clock.locale;
        if locale.check_digits(text).is_err() {
            return false;
        }
        if let Some((date, clock)) = text.split_once('T') {
            locale.date_input(date).is_ok() && self.clock.incomplete(clock)
        } else {
            locale.date_input(text).is_ok() || locale.date_prefix(text)
        }
    }
}

// Controls capture this editor rather than their owning DateTimePicker, avoiding
// ownership cycles and keeping all candidate writes behind the same validation.
#[derive(Clone)]
struct Editor {
    value: Binding<DateTime>,
    draft: Draft<DateTime>,
    rules: Rules,
    read_only: bool,
    blur: bool,
    submit: Option<Rc<dyn Fn()>>,
    cancel: Option<Rc<dyn Fn()>>,
}
impl Editor {
    fn configured(mut self) -> Self {
        assert!(self.rules.min <= self.rules.max, "Invalid datetime bounds");
        let (parse, status, format, model) = (
            self.rules.clone(),
            self.rules.clone(),
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
            move |s| match status.raw(s, source_status.get()) {
                Ok(v) => status.check(v).status,
                Err(_) if status.incomplete(s) => ValidationStatus::Incomplete,
                Err(_) => ValidationStatus::Invalid,
            },
            move |v| model.check(*v),
        );
        self.draft = self.draft.reconfigured(draft).read_only(self.read_only);
        self
    }
    fn raw(&self) -> Option<DateTime> {
        crate::state::read_only(|| {
            self.rules
                .raw(&self.draft.binding().get(), self.value.get())
                .ok()
        })
    }
    fn current(&self) -> Option<DateTime> {
        self.raw()
            .filter(|v| self.rules.check(*v).status == ValidationStatus::Valid)
    }
    fn commit(&self) -> bool {
        crate::state::batch(|| {
            self.draft.commit() && self.draft.validation().status == ValidationStatus::Valid
        })
    }
    fn publish(&self, value: DateTime) -> bool {
        if self.read_only || self.rules.check(value).status != ValidationStatus::Valid {
            return false;
        }
        self.draft.accept(value)
    }
    fn select(&self, date: Date) -> bool {
        crate::state::batch(|| {
            self.raw()
                .is_some_and(|v| self.publish(date.to_datetime(v.time())))
        })
    }
    fn next(&self, part: TimePart, up: bool) -> Option<DateTime> {
        if self.read_only {
            return None;
        }
        let value = self.current()?;
        let seconds = self.rules.clock.step_seconds(value.time(), part, up)?;
        let next = value
            .checked_add(jiff::SignedDuration::from_secs(seconds))
            .ok()?;
        (self.rules.check(next).status == ValidationStatus::Valid).then_some(next)
    }
    fn step(&self, part: TimePart, up: bool) -> bool {
        crate::state::batch(|| self.next(part, up).is_some_and(|v| self.publish(v)))
    }
    fn submit(&self) {
        if self.commit()
            && let Some(f) = &self.submit
        {
            f()
        }
    }
    fn cancel(&self) {
        crate::state::batch(|| {
            self.draft.cancel();
            if let Some(f) = &self.cancel {
                f()
            }
        })
    }
    fn parts(&self) -> View {
        let (read, can, change) = (self.clone(), self.clone(), self.clone());
        ClockControls {
            clock: self.rules.clock.clone(),
            read: Rc::new(move || read.raw().unwrap_or_else(|| read.value.get()).time()),
            can: Rc::new(move |part, up| can.next(part, up).is_some()),
            change: Rc::new(move |part, up| change.step(part, up)),
            read_only: self.read_only,
        }
        .view()
    }
}

/// A civil datetime editor with a single draft and whole-candidate validation.
/// Date and time can be edited together before one valid application update.
/// Clock steps carry across midnight; calendar choices keep the current clock.
#[derive(Clone)]
pub struct DateTimePicker {
    editor: Editor,
    calendar: DatePicker,
}
impl DateTimePicker {
    pub fn new(value: Binding<DateTime>) -> Self {
        let draft = crate::datetime_draft(value.clone());
        Self::from_editor(
            Editor {
                value,
                draft,
                rules: Rules {
                    clock: Clock::default(),
                    min: DateTime::MIN,
                    max: DateTime::MAX,
                    disabled: None,
                    validate: None,
                    external: None,
                },
                read_only: false,
                blur: true,
                submit: None,
                cancel: None,
            }
            .configured(),
        )
    }
    fn from_editor(editor: Editor) -> Self {
        let (read, write, revision, check) = (
            editor.clone(),
            editor.clone(),
            editor.clone(),
            editor.clone(),
        );
        let binding = Binding::new(
            move || read.raw().unwrap_or_else(|| read.value.get()).date(),
            move |date| {
                write.select(date);
            },
        )
        .with_revision(move || revision.draft.revision());
        let calendar = DatePicker::new(binding)
            .locale(editor.rules.clock.locale.clone())
            .bounds(editor.rules.min.date(), editor.rules.max.date())
            .validate(move |date| {
                let value = check
                    .raw()
                    .ok_or_else(|| Error("Complete the datetime first".into()))?;
                let status = check.rules.check(date.to_datetime(value.time()));
                if status.status == ValidationStatus::Valid {
                    Ok(())
                } else {
                    Err(Error(status.message.unwrap()))
                }
            });
        Self { editor, calendar }
    }
    fn configure(mut self, update: impl FnOnce(&mut Rules)) -> Self {
        update(&mut self.editor.rules);
        Self::from_editor(self.editor.configured())
    }
    pub fn bounds(self, min: DateTime, max: DateTime) -> Self {
        self.configure(|r| {
            r.min = min;
            r.max = max
        })
    }
    pub fn hour_cycle(self, cycle: HourCycle) -> Self {
        self.configure(|r| r.clock.cycle = cycle)
    }
    pub fn show_seconds(self, show: bool) -> Self {
        self.configure(|r| r.clock.seconds = show)
    }
    pub fn minute_step(self, step: u8) -> Self {
        assert!((1..=60).contains(&step));
        self.configure(|r| r.clock.minute_step = step)
    }
    pub fn second_step(self, step: u8) -> Self {
        assert!((1..=60).contains(&step));
        self.configure(|r| r.clock.second_step = step)
    }
    pub fn locale(self, locale: EditLocale) -> Self {
        self.configure(|r| r.clock.locale = locale)
    }
    pub fn disabled_dates(self, disabled: impl Fn(Date) -> bool + 'static) -> Self {
        self.configure(|r| r.disabled = Some(Rc::new(disabled)))
    }
    pub fn validate(self, validate: impl Fn(DateTime) -> Result<(), Error> + 'static) -> Self {
        self.configure(|r| r.validate = Some(Rc::new(validate)))
    }
    pub fn read_only(mut self, read_only: bool) -> Self {
        self.editor.read_only = read_only;
        self.editor.draft = self.editor.draft.read_only(read_only);
        self
    }
    pub fn commit_on_blur(mut self, commit: bool) -> Self {
        self.editor.blur = commit;
        self
    }
    pub fn on_validation(mut self, callback: impl Fn(Validation) + 'static) -> Self {
        self.editor.draft = self.editor.draft.on_validation(callback);
        Self::from_editor(self.editor)
    }
    pub fn on_submit(mut self, callback: impl Fn() + 'static) -> Self {
        self.editor.submit = Some(Rc::new(callback));
        self
    }
    pub fn on_cancel(mut self, callback: impl Fn() + 'static) -> Self {
        self.editor.cancel = Some(Rc::new(callback));
        self
    }
    pub fn binding(&self) -> Binding<String> {
        self.editor.draft.binding()
    }
    pub(crate) fn external_check(self, check: impl Fn(DateTime) -> Validation + 'static) -> Self {
        self.configure(|r| r.external = Some(Rc::new(check)))
    }
    pub(crate) fn raw_candidate(&self) -> Option<DateTime> {
        self.editor.raw()
    }
    pub(crate) fn civil_check(&self, value: DateTime) -> Validation {
        self.editor.rules.civil_check(value)
    }
    pub(crate) fn reset_draft(&self) -> bool {
        self.editor.draft.reset()
    }
    pub fn draft(&self) -> &Draft<DateTime> {
        &self.editor.draft
    }
    pub fn validation(&self) -> Validation {
        self.editor.draft.validation()
    }
    pub fn commit(&self) -> bool {
        self.editor.commit()
    }
    pub fn cancel(&self) {
        self.calendar.cancel();
        self.editor.cancel()
    }
    pub fn can_step(&self, part: TimePart, increase: bool) -> bool {
        self.editor.next(part, increase).is_some()
    }
    pub fn step(&self, part: TimePart, increase: bool) -> bool {
        self.editor.step(part, increase)
    }
    pub fn select(&self, date: Date) -> bool {
        self.editor.select(date)
    }
    pub fn calendar(&self) -> CalendarInfo {
        self.calendar.calendar()
    }
    pub fn view(&self) -> View {
        let (submit, cancel, blur, apply, revert, calendar_cancel) = (
            self.editor.clone(),
            self.clone(),
            self.editor.clone(),
            self.editor.clone(),
            self.clone(),
            self.editor.clone(),
        );
        let locale = &self.editor.rules.clock.locale;
        let hint = self.editor.rules.hint();
        let mut children = vec![
            View::label(&hint),
            View::text_field(&hint, self.binding())
                .key("$datetime-field")
                .accessible_label(&hint)
                .validation(self.validation())
                .read_only(self.editor.read_only)
                .on_submit(move || submit.submit())
                .on_cancel(move || cancel.cancel()),
            self.editor.parts(),
            self.calendar
                .clone()
                .read_only(self.editor.read_only)
                .on_cancel(move || calendar_cancel.cancel())
                .calendar_view()
                .enabled(self.editor.raw().is_some())
                .key("$datetime-calendar"),
            View::row(vec![
                View::button(locale.action(ActionLabel::Apply), move || apply.submit())
                    .enabled(!self.editor.read_only),
                View::button(locale.action(ActionLabel::Revert), move || revert.cancel())
                    .value_cancel()
                    .enabled(!self.editor.read_only),
            ]),
        ];
        if let Some(e) = self.editor.draft.error() {
            children.push(View::label(e).wrap(true).foreground(0xffff8787));
        }
        View::panel(children)
            .accessible_label(locale.datetime_label())
            .value_editor()
            .on_blur(move || {
                if blur.blur {
                    blur.commit();
                }
            })
    }
}
