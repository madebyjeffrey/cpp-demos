//! Explicit draft/commit values. Numbers default to ASCII; checked locale profiles are opt-in.
use crate::locale::ActionLabel;
use crate::{Binding, EditLocale, Repeat, State, View};
use std::{cell::Cell, fmt::Display, rc::Rc, str::FromStr};
type Parse<T> = Rc<dyn Fn(&str) -> Result<T, String>>;
type ModelStatus<T> = Rc<dyn Fn(&T) -> Validation>;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ValidationStatus {
    Valid,
    Incomplete,
    Invalid,
    OutOfRange,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Validation {
    pub status: ValidationStatus,
    pub message: Option<String>,
    pub dirty: bool,
}
#[derive(Clone, PartialEq)]
struct Edit<T> {
    text: String,
    base: T,
    base_text: String,
    revision: Option<u64>,
    rejected: Option<String>,
}
#[derive(Clone)]
pub struct Draft<T> {
    value: Binding<T>,
    edit: State<Edit<T>>,
    parse: Parse<T>,
    format: Rc<dyn Fn(&T) -> String>,
    classify: Rc<dyn Fn(&str) -> ValidationStatus>,
    model_status: Option<ModelStatus<T>>,
    notify: Option<Rc<dyn Fn(Validation)>>,
    notifying: Rc<Cell<bool>>,
    read_only: bool,
}
impl<T: Clone + PartialEq + 'static> Draft<T> {
    pub fn new(
        value: Binding<T>,
        parse: impl Fn(&str) -> Result<T, String> + 'static,
        format: impl Fn(&T) -> String + 'static,
    ) -> Self {
        let (base, text, revision) = crate::state::read_only(|| {
            let base = value.get();
            let text = format(&base);
            (base, text, value.revision())
        });
        let parse: Parse<T> = Rc::new(parse);
        let classify = parse.clone();
        Self {
            value,
            edit: State::new(Edit {
                base,
                base_text: text.clone(),
                text,
                revision,
                rejected: None,
            }),
            parse,
            format: Rc::new(format),
            classify: Rc::new(move |text| {
                if classify(text).is_ok() {
                    ValidationStatus::Valid
                } else if text.is_empty() {
                    ValidationStatus::Incomplete
                } else {
                    ValidationStatus::Invalid
                }
            }),
            model_status: None,
            notify: None,
            notifying: Rc::new(Cell::new(false)),
            read_only: false,
        }
    }
    pub fn read_only(mut self, read_only: bool) -> Self {
        self.read_only = read_only;
        self
    }
    pub fn is_read_only(&self) -> bool {
        self.read_only
    }
    pub fn on_validation(mut self, callback: impl Fn(Validation) + 'static) -> Self {
        self.notify = Some(Rc::new(callback));
        self
    }
    pub(crate) fn classifiers(
        mut self,
        classify: impl Fn(&str) -> ValidationStatus + 'static,
        model: impl Fn(&T) -> Validation + 'static,
    ) -> Self {
        self.classify = Rc::new(classify);
        self.model_status = Some(Rc::new(model));
        self
    }
    pub(crate) fn reconfigured(&self, mut draft: Self) -> Self {
        draft.notify = self.notify.clone();
        draft.notifying = self.notifying.clone();
        draft.read_only = self.read_only;
        draft
    }
    pub(crate) fn revision(&self) -> u64 {
        self.edit
            .version()
            .wrapping_add(self.value.revision().unwrap_or(0))
    }
    fn fresh(&self) -> Edit<T> {
        crate::state::read_only(|| {
            let base = self.value.get();
            let text = (self.format)(&base);
            Edit {
                base,
                base_text: text.clone(),
                text,
                revision: self.value.revision(),
                rejected: None,
            }
        })
    }
    fn same_source(a: &Edit<T>, b: &Edit<T>) -> bool {
        // Non-reflexive models such as NaN still need a stable editable baseline.
        // Revisions detect equal-looking external replacements; format text detects
        // metadata/context changes hidden by the model's PartialEq implementation.
        a.revision == b.revision
            && a.base_text == b.base_text
            && (a.base == b.base || !a.base.eq(&a.base) && !b.base.eq(&b.base))
    }
    fn current(&self) -> Edit<T> {
        crate::state::read_only(|| {
            let edit = self.edit.get();
            let source = self.fresh();
            if Self::same_source(&edit, &source) {
                edit
            } else {
                source
            }
        })
    }
    fn source_matches(&self, source: &Edit<T>) -> bool {
        crate::state::read_only(|| Self::same_source(source, &self.fresh()))
    }
    pub fn binding(&self) -> Binding<String> {
        let (read, write) = (self.clone(), self.clone());
        Binding::new(
            move || read.current().text,
            move |text| {
                write.stage_text(text);
            },
        )
    }
    pub(crate) fn stage_text(&self, text: String) -> bool {
        if self.read_only {
            return false;
        }
        crate::state::batch(|| {
            let mut edit = self.fresh();
            edit.text = text;
            self.edit.set(edit);
            let version = self.edit.version();
            let source = self.fresh();
            self.notify();
            self.edit.version() == version && self.source_matches(&source)
        })
    }
    fn status(&self, edit: &Edit<T>) -> Validation {
        let dirty = edit.text != edit.base_text;
        if !dirty && let Some(model) = &self.model_status {
            let mut value = model(&edit.base);
            value.dirty = false;
            return value;
        }
        let status = (self.classify)(&edit.text);
        Validation {
            status,
            dirty,
            message: if status == ValidationStatus::Valid {
                None
            } else {
                (self.parse)(&edit.text).err().or_else(|| {
                    Some(
                        if status == ValidationStatus::OutOfRange {
                            "The committed value is outside supported bounds"
                        } else {
                            "The committed value is invalid"
                        }
                        .into(),
                    )
                })
            },
        }
    }
    pub(crate) fn raw_validation(&self) -> Validation {
        crate::state::read_only(|| self.status(&self.current()))
    }
    pub fn validation(&self) -> Validation {
        crate::state::read_only(|| {
            let edit = self.current();
            let mut value = self.status(&edit);
            if value.status == ValidationStatus::Valid
                && let Some(message) = edit.rejected
            {
                value.status = ValidationStatus::Invalid;
                value.message = Some(message);
                value.dirty = true;
            }
            value
        })
    }
    pub fn error(&self) -> Option<String> {
        self.validation().message
    }
    fn notify(&self) {
        if let Some(callback) = &self.notify {
            if self.notifying.replace(true) {
                return;
            }
            struct Reset<'a>(&'a Cell<bool>);
            impl Drop for Reset<'_> {
                fn drop(&mut self) {
                    self.0.set(false)
                }
            }
            let _reset = Reset(&self.notifying);
            callback(self.validation());
        }
    }
    pub(crate) fn candidate(&self) -> Option<T> {
        crate::state::read_only(|| {
            let edit = self.current();
            if self.status(&edit).status != ValidationStatus::Valid {
                return None;
            }
            if edit.text == edit.base_text {
                Some(edit.base)
            } else {
                (self.parse)(&edit.text).ok()
            }
        })
    }
    // None means a reentrant setter has already staged a newer edit. Never reset
    // it or repeat its notification as part of the superseded outer operation.
    fn write(&self, value: T) -> Option<bool> {
        if self.read_only {
            return Some(false);
        }
        if self.model_status.as_ref().is_some_and(|f| {
            crate::state::read_only(|| f(&value).status != ValidationStatus::Valid)
        }) {
            return Some(false);
        }
        let version = self.edit.version();
        let before = self.current();
        let same = crate::state::read_only(|| {
            value == before.base && (self.format)(&value) == before.base_text
        });
        if !same {
            self.value.set(value);
        }
        if self.edit.version() != version {
            return None;
        }
        let accepted = self.fresh();
        if !same && crate::state::read_only(|| Self::same_source(&before, &accepted)) {
            let mut rejected = before;
            rejected.rejected = Some("The application rejected the value; Apply can retry".into());
            self.edit.set(rejected);
            return Some(false);
        }
        self.edit.set(accepted);
        Some(self.raw_validation().status == ValidationStatus::Valid)
    }
    fn finish(&self, result: Option<bool>) -> bool {
        let Some(accepted) = result else {
            return false;
        };
        let version = self.edit.version();
        let source = self.fresh();
        self.notify();
        accepted
            && self.edit.version() == version
            && self.source_matches(&source)
            && self.validation().status == ValidationStatus::Valid
    }
    pub(crate) fn accept(&self, value: T) -> bool {
        if self.read_only {
            return false;
        }
        crate::state::batch(|| {
            let result = self.write(value);
            self.finish(result)
        })
    }
    pub fn commit(&self) -> bool {
        if self.read_only {
            return false;
        }
        crate::state::batch(|| {
            let edit = self.current();
            let status = crate::state::read_only(|| self.status(&edit));
            let result = if status.status != ValidationStatus::Valid {
                Some(false)
            } else if !status.dirty {
                Some(true)
            } else {
                match crate::state::read_only(|| (self.parse)(&edit.text)) {
                    Ok(value) => self.write(value),
                    Err(_) => Some(false),
                }
            };
            self.finish(result)
        })
    }
    pub(crate) fn reset(&self) -> bool {
        crate::state::batch(|| {
            self.edit.set(self.fresh());
            let version = self.edit.version();
            let source = self.fresh();
            self.notify();
            self.edit.version() == version && self.source_matches(&source)
        })
    }
    pub fn cancel(&self) {
        self.reset();
    }
}
/// Implementors format finite values using ASCII numeric syntax; localized editing
/// converts that text directly, without passing integer values through floating point.
pub trait Number: Clone + Copy + PartialOrd + Display + FromStr + 'static {
    fn checked_step(self, step: Self, increase: bool) -> Option<Self>;
    fn positive(self) -> bool;
    fn valid(self) -> bool {
        true
    }
    fn format(self, _digits: Option<usize>) -> String {
        self.to_string()
    }
    fn incomplete(text: &str) -> bool {
        matches!(text, "" | "+" | "-")
    }
    fn overflow(_text: &str) -> bool {
        false
    }
}
macro_rules! integers{($($t:ty),*)=>{$(impl Number for $t{
    fn checked_step(self,step:Self,increase:bool)->Option<Self>{if increase{self.checked_add(step)}else{self.checked_sub(step)}}
    fn positive(self)->bool{self>0}
    fn overflow(text:&str)->bool{text.parse::<Self>().is_err_and(|e|matches!(e.kind(),std::num::IntErrorKind::PosOverflow|std::num::IntErrorKind::NegOverflow) || text.strip_prefix('-').is_some_and(|s|!s.is_empty() && s.bytes().all(|b|b.is_ascii_digit())))}
})*};}
integers!(i32, i64, u32, u64);
impl Number for f64 {
    fn checked_step(self, step: Self, increase: bool) -> Option<Self> {
        let v = if increase { self + step } else { self - step };
        v.is_finite().then_some(v)
    }
    fn positive(self) -> bool {
        self > 0.
    }
    fn valid(self) -> bool {
        self.is_finite()
    }
    fn incomplete(text: &str) -> bool {
        if matches!(text, "" | "+" | "-" | "." | "+." | "-.") {
            return true;
        }
        text.rsplit_once(['e', 'E'])
            .is_some_and(|(mantissa, exponent)| {
                matches!(exponent, "" | "+" | "-")
                    && mantissa.parse::<f64>().is_ok_and(f64::is_finite)
            })
    }
    fn format(self, digits: Option<usize>) -> String {
        let Some(digits) = digits else {
            return self.to_string();
        };
        if self == 0. || !self.is_finite() {
            return self.to_string();
        }
        let exponent = self.abs().log10().floor() as i32;
        let formatted = if exponent < -4 || exponent >= digits as i32 {
            format!("{:.*e}", digits - 1, self)
        } else {
            format!(
                "{:.*}",
                (digits as i32 - 1 - exponent).max(0) as usize,
                self
            )
        };
        let (mantissa, exponent) = formatted
            .split_once('e')
            .map_or((formatted.as_str(), None), |(a, b)| (a, Some(b)));
        let mantissa = if mantissa.contains('.') {
            mantissa.trim_end_matches('0').trim_end_matches('.')
        } else {
            mantissa
        };
        let text = if let Some(e) = exponent {
            format!("{mantissa}e{e}")
        } else {
            mantissa.into()
        };
        if text.parse::<f64>().is_ok_and(f64::is_finite) {
            text
        } else {
            self.to_string()
        }
    }
}
fn status<T: Number>(text: &str, min: T, max: T) -> ValidationStatus {
    if T::incomplete(text) {
        return ValidationStatus::Incomplete;
    }
    match text.parse::<T>() {
        Ok(v) if !v.valid() => ValidationStatus::Invalid,
        Ok(v) if v < min || v > max => ValidationStatus::OutOfRange,
        Ok(_) => ValidationStatus::Valid,
        Err(_) if T::overflow(text) => ValidationStatus::OutOfRange,
        Err(_) => ValidationStatus::Invalid,
    }
}
fn numeric_draft<T: Number>(
    value: Binding<T>,
    min: T,
    max: T,
    locale: &EditLocale,
    digits: Option<usize>,
) -> Draft<T> {
    let (parse_locale, format_locale, status_locale, model_locale) = (
        locale.clone(),
        locale.clone(),
        locale.clone(),
        locale.clone(),
    );
    let mut draft = Draft::new(
        value,
        move |text| {
            let ascii = parse_locale.number_input(text)?;
            match status::<T>(&ascii, min, max) {
                ValidationStatus::Valid => ascii
                    .parse::<T>()
                    .map_err(|_| parse_locale.incomplete_error().into()),
                ValidationStatus::Incomplete => Err(parse_locale.incomplete_error().into()),
                ValidationStatus::Invalid => Err(parse_locale.number_error().into()),
                ValidationStatus::OutOfRange => {
                    Err(parse_locale.range_error(&min.to_string(), &max.to_string()))
                }
            }
        },
        move |v| format_locale.number(&v.format(digits)),
    );
    draft.classify = Rc::new(move |text| {
        status_locale
            .number_input(text)
            .map_or(ValidationStatus::Invalid, |ascii| {
                status::<T>(&ascii, min, max)
            })
    });
    draft.model_status = Some(Rc::new(move |v| {
        let status = if !v.valid() {
            ValidationStatus::Invalid
        } else if *v < min || *v > max {
            ValidationStatus::OutOfRange
        } else if model_locale
            .number_input(&model_locale.number(&v.format(digits)))
            .is_ok_and(|s| s.parse::<T>().is_ok_and(|v| v.valid()))
        {
            ValidationStatus::Valid
        } else {
            ValidationStatus::Invalid
        };
        let message = match status {
            ValidationStatus::Valid => None,
            ValidationStatus::OutOfRange => {
                Some(model_locale.range_error(&min.to_string(), &max.to_string()))
            }
            _ => Some(model_locale.number_error().into()),
        };
        Validation {
            status,
            message,
            dirty: false,
        }
    }));
    draft
}
#[derive(Clone)]
pub struct Numeric<T: Number> {
    pub draft: Draft<T>,
    value: Binding<T>,
    min: T,
    max: T,
    step: T,
    locale: EditLocale,
    digits: Option<usize>,
    read_only: bool,
    commit_on_blur: bool,
    repeat: Repeat,
    submit: Option<Rc<dyn Fn()>>,
    cancel: Option<Rc<dyn Fn()>>,
}
impl<T: Number> Numeric<T> {
    pub fn new(value: Binding<T>, min: T, max: T, step: T) -> Self {
        assert!(
            min.valid() && max.valid() && step.valid() && step.positive() && min <= max,
            "Invalid numeric bounds/step"
        );
        let locale = EditLocale::ASCII;
        let draft = numeric_draft(value.clone(), min, max, &locale, None);
        Self {
            draft,
            value,
            min,
            max,
            step,
            locale,
            digits: None,
            read_only: false,
            commit_on_blur: true,
            repeat: Repeat::default(),
            submit: None,
            cancel: None,
        }
    }
    /// Maximum significant digits for decimal display; integers always remain exact.
    pub fn significant_digits(mut self, digits: usize) -> Self {
        assert!((1..=17).contains(&digits), "Use 1–17 significant digits");
        self.digits = Some(digits);
        self.reformat();
        self
    }
    /// Configure editing before mounting; rebuilding the format restores the committed value.
    pub fn locale(mut self, locale: EditLocale) -> Self {
        self.locale = locale;
        self.reformat();
        self
    }
    fn reformat(&mut self) {
        let draft = numeric_draft(
            self.value.clone(),
            self.min,
            self.max,
            &self.locale,
            self.digits,
        );
        self.draft.parse = draft.parse;
        self.draft.format = draft.format;
        self.draft.classify = draft.classify;
        self.draft.model_status = draft.model_status;
        self.draft.edit = draft.edit;
    }
    pub fn read_only(mut self, value: bool) -> Self {
        self.read_only = value;
        self.draft.read_only = value;
        self
    }
    pub fn commit_on_blur(mut self, value: bool) -> Self {
        self.commit_on_blur = value;
        self
    }
    pub fn repeat(mut self, repeat: Repeat) -> Self {
        self.repeat = repeat;
        self
    }
    pub fn on_validation(mut self, callback: impl Fn(Validation) + 'static) -> Self {
        self.draft.notify = Some(Rc::new(callback));
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
    pub fn validation(&self) -> Validation {
        self.draft.validation()
    }
    pub fn binding(&self) -> Binding<String> {
        self.draft.binding()
    }
    pub fn commit(&self) -> bool {
        self.draft.commit()
    }
    pub fn cancel(&self) {
        self.draft.cancel();
        if let Some(callback) = &self.cancel {
            callback();
        }
    }
    fn submit(&self) {
        if self.commit()
            && let Some(callback) = &self.submit
        {
            callback();
        }
    }
    fn blur(&self) {
        if self.commit_on_blur {
            self.commit();
        }
    }
    fn next(&self, increase: bool) -> Option<T> {
        crate::state::read_only(|| self.next_value(increase))
    }
    fn next_value(&self, increase: bool) -> Option<T> {
        if self.read_only || self.draft.raw_validation().status != ValidationStatus::Valid {
            return None;
        }
        let current = self.draft.candidate()?;
        let next = current
            .checked_step(self.step, increase)
            .unwrap_or(if increase { self.max } else { self.min });
        let next = if next < self.min {
            self.min
        } else if next > self.max {
            self.max
        } else {
            next
        };
        (next != current).then_some(next)
    }
    pub fn can_step(&self, increase: bool) -> bool {
        self.next(increase).is_some()
    }
    pub fn step(&self, increase: bool) -> bool {
        let Some(next) = self.next(increase) else {
            return false;
        };
        self.draft.accept(next)
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StepperInfo {
    pub value: String,
    pub min: String,
    pub max: String,
    pub can_increment: bool,
    pub can_decrement: bool,
}
#[derive(Clone)]
pub(crate) struct StepperData {
    pub read: Rc<dyn Fn() -> StepperInfo>,
    pub change: Rc<dyn Fn(bool) -> bool>,
}
impl crate::Host {
    pub fn step_control(&self, id: crate::NodeId, increase: bool) -> Result<bool, crate::Error> {
        self.refresh()?;
        let change = {
            let m = self.0.mounted.borrow();
            m.nodes
                .iter()
                .find(|n| {
                    n.id == id
                        && n.enabled
                        && !n.read_only
                        && !n.hidden
                        && !n.layout_hidden
                        && self.in_scope(id)
                })
                .and_then(|_| m.descriptions.get(&id))
                .and_then(|d| d.stepper.as_ref())
                .map(|s| s.change.clone())
        };
        let changed = change.is_some_and(|change| change(increase));
        if changed {
            self.redraw();
        }
        Ok(changed)
    }
}
impl View {
    pub fn stepper<T: Number>(label: impl Into<String>, number: &Numeric<T>) -> Self {
        let label = label.into();
        let validation = number.validation();
        let down = number.clone();
        let up = number.clone();
        let mut view = Self::row(vec![
            Self::button("−", move || {
                down.step(false);
            })
            .accessible_label(format!(
                "{} {label}",
                number.locale.action(ActionLabel::Decrease)
            ))
            .enabled(number.can_step(false))
            .repeat(number.repeat)
            .width(34),
            Self::button("+", move || {
                up.step(true);
            })
            .accessible_label(format!(
                "{} {label}",
                number.locale.action(ActionLabel::Increase)
            ))
            .enabled(number.can_step(true))
            .repeat(number.repeat)
            .width(34),
        ])
        .gap(4.)
        .width(72)
        .validation(validation)
        .shrink(0.);
        let (read, change) = (number.clone(), number.clone());
        let d = Rc::make_mut(&mut view.0);
        d.kind = crate::Kind::Stepper;
        d.accessible = label;
        d.read_only = number.read_only;
        d.stepper = Some(StepperData {
            read: Rc::new(move || StepperInfo {
                value: read
                    .draft
                    .candidate()
                    .unwrap_or_else(|| read.value.get())
                    .to_string(),
                min: read.min.to_string(),
                max: read.max.to_string(),
                can_increment: read.can_step(true),
                can_decrement: read.can_step(false),
            }),
            change: Rc::new(move |increase| change.step(increase)),
        });
        view
    }
    pub fn numeric_field<T: Number>(label: impl Into<String>, number: &Numeric<T>) -> Self {
        let label = label.into();
        let commit = number.clone();
        let cancel = number.clone();
        let blur = number.clone();
        let mut children = vec![
            Self::label(&label),
            Self::row(vec![
                Self::text_field(number.locale.number_hint(), number.draft.binding())
                    .accessible_help(number.locale.number_hint())
                    .validation(number.validation())
                    .accessible_label(&label)
                    .read_only(number.read_only)
                    .grow(1.)
                    .on_submit(move || commit.submit())
                    .on_cancel(move || cancel.cancel())
                    .on_blur(move || blur.blur()),
                Self::stepper(&label, number),
            ]),
        ];
        if let Some(error) = number.draft.error() {
            children.push(Self::label(error).foreground(0xffff8787).wrap(true));
        }
        Self::column(children)
    }
}
pub fn numeric_field<T: Number>(label: impl Into<String>, number: &Numeric<T>) -> View {
    View::numeric_field(label, number)
}
pub fn stepper<T: Number>(label: impl Into<String>, number: &Numeric<T>) -> View {
    View::stepper(label, number)
}
