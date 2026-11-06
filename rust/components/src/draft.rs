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
    revision: Option<u64>,
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
        let base = value.get();
        let text = format(&base);
        let revision = value.revision();
        let parse: Parse<T> = Rc::new(parse);
        let classify = parse.clone();
        Self {
            value,
            edit: State::new(Edit {
                text,
                base,
                revision,
            }),
            parse,
            format: Rc::new(format),
            classify: Rc::new(move |text| {
                if classify(text).is_ok() {
                    ValidationStatus::Valid
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
    pub fn binding(&self) -> Binding<String> {
        let read = self.clone();
        let write = self.clone();
        Binding::new(
            move || {
                let e = read.edit.get();
                let current = read.value.get();
                if read.value.revision() != e.revision
                    || current != e.base
                    || (read.format)(&current) != (read.format)(&e.base)
                {
                    (read.format)(&current)
                } else {
                    e.text
                }
            },
            move |text: String| {
                if write.read_only {
                    return;
                }
                let base = write.value.get();
                let revision = write.value.revision();
                write.edit.set(Edit {
                    text,
                    base,
                    revision,
                });
                write.notify();
            },
        )
    }
    pub fn validation(&self) -> Validation {
        let text = self.binding().get();
        let current = self.value.get();
        let dirty = text != (self.format)(&current);
        if !dirty && let Some(status) = &self.model_status {
            return status(&current);
        }
        let status = (self.classify)(&text);
        Validation {
            status,
            message: if status == ValidationStatus::Valid {
                None
            } else {
                (self.parse)(&text).err().or_else(|| {
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
            dirty,
        }
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
                    self.0.set(false);
                }
            }
            let _reset = Reset(&self.notifying);
            callback(self.validation());
        }
    }
    pub fn commit(&self) -> bool {
        if self.read_only {
            return false;
        }
        let validation = self.validation();
        // Formatting precision must never round an untouched model on focus loss.
        if !validation.dirty {
            self.notify();
            return validation.status == ValidationStatus::Valid;
        }
        match (self.parse)(&self.binding().get()) {
            Ok(value) => {
                let before = self.value.get();
                self.value.set(value.clone());
                let accepted = self.value.get();
                if accepted == before && accepted != value {
                    self.notify();
                    false
                } else {
                    self.cancel();
                    true
                }
            }
            Err(_) => {
                self.notify();
                false
            }
        }
    }
    pub fn cancel(&self) {
        let base = self.value.get();
        self.edit.set(Edit {
            text: (self.format)(&base),
            revision: self.value.revision(),
            base,
        });
        self.notify();
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
        if self.read_only || self.validation().status != ValidationStatus::Valid {
            return None;
        }
        let current = if self.validation().dirty {
            (self.draft.parse)(&self.draft.binding().get()).ok()?
        } else {
            self.value.get()
        };
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
        // Publish one accepted value; never round integers or overwrite invalid drafts.
        let before = self.value.get();
        self.value.set(next);
        if self.value.get() == before {
            return false;
        }
        self.draft.cancel();
        true
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
                value: read.value.get().to_string(),
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
