//! Explicit draft/commit values. Editable numbers use ungrouped ASCII syntax.
use crate::{Binding, State, View};
use std::{fmt::Display, rc::Rc, str::FromStr};
type Parse<T> = Rc<dyn Fn(&str) -> Result<T, String>>;
#[derive(Clone, PartialEq)]
struct Edit {
    text: String,
    base: String,
    error: Option<String>,
}
#[derive(Clone)]
pub struct Draft<T> {
    value: Binding<T>,
    edit: State<Edit>,
    parse: Parse<T>,
    format: Rc<dyn Fn(&T) -> String>,
}
impl<T: Clone + 'static> Draft<T> {
    pub fn new(
        value: Binding<T>,
        parse: impl Fn(&str) -> Result<T, String> + 'static,
        format: impl Fn(&T) -> String + 'static,
    ) -> Self {
        let text = format(&value.get());
        Self {
            value,
            edit: State::new(Edit {
                text: text.clone(),
                base: text,
                error: None,
            }),
            parse: Rc::new(parse),
            format: Rc::new(format),
        }
    }
    pub fn binding(&self) -> Binding<String> {
        let read = self.clone();
        let write = self.clone();
        Binding::new(
            move || {
                let e = read.edit.get();
                let current = (read.format)(&read.value.get());
                if current != e.base { current } else { e.text }
            },
            move |text: String| {
                let base = (write.format)(&write.value.get());
                let error = (write.parse)(&text).err();
                write.edit.set(Edit { text, base, error });
            },
        )
    }
    pub fn error(&self) -> Option<String> {
        let e = self.edit.get();
        if e.base == (self.format)(&self.value.get()) {
            e.error
        } else {
            None
        }
    }
    pub fn commit(&self) -> bool {
        let text = self.binding().get();
        match (self.parse)(&text) {
            Ok(value) => {
                self.value.set(value);
                self.cancel();
                true
            }
            Err(error) => {
                let mut e = self.edit.get();
                e.error = Some(error);
                self.edit.set(e);
                false
            }
        }
    }
    pub fn cancel(&self) {
        let text = (self.format)(&self.value.get());
        self.edit.set(Edit {
            text: text.clone(),
            base: text,
            error: None,
        });
    }
}
pub trait Number: Clone + Copy + PartialOrd + Display + FromStr + 'static {
    fn checked_step(self, step: Self, increase: bool) -> Option<Self>;
    fn valid(self) -> bool {
        true
    }
}
macro_rules! integers {($($t:ty),*)=>{$(impl Number for $t { fn checked_step(self,step:Self,increase:bool)->Option<Self>{if increase {self.checked_add(step)} else {self.checked_sub(step)}} })*};}
integers!(i32, i64, u32, u64);
impl Number for f64 {
    fn checked_step(self, step: Self, increase: bool) -> Option<Self> {
        let v = if increase { self + step } else { self - step };
        v.is_finite().then_some(v)
    }
    fn valid(self) -> bool {
        self.is_finite()
    }
}
#[derive(Clone)]
pub struct Numeric<T: Number> {
    pub draft: Draft<T>,
    value: Binding<T>,
    min: T,
    max: T,
    step: T,
}
impl<T: Number> Numeric<T> {
    pub fn new(value: Binding<T>, min: T, max: T, step: T) -> Self {
        assert!(
            min.valid() && max.valid() && step.valid() && min <= max,
            "Invalid numeric bounds"
        );
        let draft = Draft::new(
            value.clone(),
            move |text| {
                let v = text
                    .parse::<T>()
                    .map_err(|_| "Enter a complete number (ASCII, no grouping)".to_owned())?;
                if !v.valid() || v < min || v > max {
                    Err(format!("Enter a value from {min} to {max}"))
                } else {
                    Ok(v)
                }
            },
            |v| v.to_string(),
        );
        Self {
            draft,
            value,
            min,
            max,
            step,
        }
    }
    pub fn step(&self, increase: bool) {
        if !self.draft.commit() {
            return;
        }
        if let Some(v) = self.value.get().checked_step(self.step, increase) {
            self.value.set(if v < self.min {
                self.min
            } else if v > self.max {
                self.max
            } else {
                v
            });
            self.draft.cancel();
        }
    }
}
impl View {
    pub fn numeric_field<T: Number>(label: impl Into<String>, number: &Numeric<T>) -> Self {
        let commit = number.draft.clone();
        let down = number.clone();
        let up = number.clone();
        let mut children = vec![
            Self::label(label),
            Self::row(vec![
                Self::text_field("ASCII number", number.draft.binding())
                    .grow(1.)
                    .on_submit(move || {
                        commit.commit();
                    }),
                Self::button("−", move || down.step(false)).width(34),
                Self::button("+", move || up.step(true)).width(34),
            ]),
        ];
        if let Some(error) = number.draft.error() {
            children.push(Self::label(error).foreground(0xffff8787).wrap(true));
        }
        Self::column(children)
    }
}
/// An editable number and a pair of exact, checked step buttons.
pub fn numeric_field<T: Number>(label: impl Into<String>, number: &Numeric<T>) -> View {
    View::numeric_field(label, number)
}
