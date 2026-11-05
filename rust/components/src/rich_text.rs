//! Styled paragraph runs and real inline views; all offsets remain UTF-8 bytes.
use crate::{Error, TextStyle, View};
use std::rc::Rc;
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SpanStyle {
    pub font_size: Option<f64>,
    pub foreground: Option<u32>,
    pub bold: Option<bool>,
    pub line_height: Option<f64>,
    pub letter_spacing: Option<f64>,
}
impl SpanStyle {
    pub fn resolve(&self, mut base: TextStyle) -> TextStyle {
        base.size = self.font_size.unwrap_or(base.size);
        base.color = self.foreground.unwrap_or(base.color);
        base.bold = self.bold.unwrap_or(base.bold);
        base.line_height = self.line_height.unwrap_or(base.line_height);
        base.letter_spacing = self.letter_spacing.unwrap_or(base.letter_spacing);
        base
    }
    pub(crate) fn validate(&self) -> Result<(), Error> {
        if self.font_size.is_some_and(|v| !v.is_finite() || v <= 0.)
            || self.line_height.is_some_and(|v| !v.is_finite() || v <= 0.)
            || self.letter_spacing.is_some_and(|v| !v.is_finite())
        {
            return Err(Error("Invalid paragraph span style".into()));
        }
        Ok(())
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct Span {
    pub text: String,
    pub style: SpanStyle,
}
impl Span {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            style: SpanStyle::default(),
        }
    }
    pub fn font_size(mut self, size: f64) -> Self {
        self.style.font_size = Some(size);
        self
    }
    pub fn foreground(mut self, color: u32) -> Self {
        self.style.foreground = Some(color);
        self
    }
    pub fn bold(mut self, bold: bool) -> Self {
        self.style.bold = Some(bold);
        self
    }
    pub fn line_height(mut self, height: f64) -> Self {
        self.style.line_height = Some(height);
        self
    }
    pub fn letter_spacing(mut self, spacing: f64) -> Self {
        self.style.letter_spacing = Some(spacing);
        self
    }
}
#[derive(Clone)]
pub enum Inline {
    Text(Span),
    View(View),
}
impl From<Span> for Inline {
    fn from(span: Span) -> Self {
        Self::Text(span)
    }
}
impl From<&str> for Inline {
    fn from(text: &str) -> Self {
        Self::Text(Span::new(text))
    }
}
impl From<String> for Inline {
    fn from(text: String) -> Self {
        Self::Text(Span::new(text))
    }
}
impl From<View> for Inline {
    fn from(view: View) -> Self {
        Self::View(view)
    }
}
#[derive(Clone)]
pub(crate) enum Run {
    Text(Span),
    Child(usize),
}
impl View {
    /// Text spans and inline views share one shaped paragraph and line-breaking context.
    pub fn rich_text(runs: impl IntoIterator<Item = Inline>) -> Self {
        let mut children = Vec::new();
        let mut text = String::new();
        let mut spans = Vec::new();
        for run in runs {
            match run {
                Inline::Text(span) => {
                    text.push_str(&span.text);
                    spans.push(Run::Text(span));
                }
                Inline::View(view) => {
                    if !view.0.visible {
                        continue;
                    }
                    text.push_str(&view.0.text);
                    spans.push(Run::Child(children.len()));
                    children.push(view);
                }
            }
        }
        let mut view = Self::new(crate::Kind::RichText, text, children).wrap(true);
        Rc::make_mut(&mut view.0).rich = Some(spans);
        view
    }
}
