//! Partial style patches: absence follows the application; explicit None restores inheritance.
use crate::{
    Align, Color, Density, Error, Insets, Justify, LabelStyle, Length, Orientation, Position,
    Style, TextOverflow, TextWrap,
};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Page {
    Layout,
    Style,
}
macro_rules! properties {
    ($($variant:ident, $field:ident: $ty:ty, $page:ident;)*) => {
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
        pub enum StyleProperty { $($variant,)* }
        /// Only explicitly set properties replace the latest application style.
        #[derive(Clone, Copy, Debug, Default, PartialEq)]
        pub struct StyleOverride { $(pub $field: Option<$ty>,)* }
        impl StyleOverride {
            $(pub fn $field(mut self, value: impl Into<$ty>) -> Self {
                self.$field = Some(value.into()); self
            })*
            pub fn apply(self, mut source: Style) -> Style {
                $(if let Some(value) = self.$field { source.$field = value; })*
                source
            }
            pub fn properties(self) -> Vec<StyleProperty> {
                StyleProperty::ALL.iter().copied().filter(|p| self.contains(*p)).collect()
            }
            pub fn contains(self, property: StyleProperty) -> bool {
                match property { $(StyleProperty::$variant => self.$field.is_some(),)* }
            }
            pub fn clear(&mut self, property: StyleProperty) {
                match property { $(StyleProperty::$variant => self.$field = None,)* }
            }
            pub fn is_empty(self) -> bool { self == Self::default() }
        }
        impl StyleProperty {
            pub const ALL: &'static [Self] = &[$(Self::$variant,)*];
            pub fn name(self) -> &'static str {
                match self { $(Self::$variant => stringify!($field),)* }
            }
            pub(super) fn page(self) -> Page {
                match self { $(Self::$variant => Page::$page,)* }
            }
            pub(super) fn display(self, style: Style) -> String {
                match self { $(Self::$variant => style.$field.display(),)* }
            }
            pub(super) fn syntax(self) -> &'static str {
                match self { $(Self::$variant => <$ty as Value>::SYNTAX,)* }
            }
            pub(super) fn parse(self, text: &str, mut patch: StyleOverride, source: Style) -> Result<StyleOverride, Error> {
                match self { $(Self::$variant => patch.$field = Some(<$ty as Value>::parse(text.trim()).map_err(Error)?),)* }
                crate::host::validate_style(patch.apply(source))?;
                Ok(patch)
            }
        }
    }
}
properties! {
    Width, width: Length, Layout;
    Height, height: Length, Layout;
    MinWidth, min_width: f64, Layout;
    MinHeight, min_height: f64, Layout;
    MaxWidth, max_width: f64, Layout;
    MaxHeight, max_height: f64, Layout;
    Padding, padding: Insets, Layout;
    Margin, margin: Insets, Layout;
    Basis, basis: Length, Layout;
    AlignSelf, align_self: Option<Align>, Layout;
    Gap, gap: f64, Layout;
    Grow, grow: f64, Layout;
    Shrink, shrink: f64, Layout;
    Wrap, wrap: bool, Layout;
    Align, align: Align, Layout;
    Justify, justify: Justify, Layout;
    Position, position: Position, Layout;
    Left, left: Length, Layout;
    Top, top: Length, Layout;
    Right, right: Length, Layout;
    Bottom, bottom: Length, Layout;
    ZIndex, z_index: i32, Layout;
    Accent, accent: Option<u32>, Style;
    Density, density: Option<Density>, Style;
    TextWrap, text_wrap: Option<TextWrap>, Style;
    TextOverflow, text_overflow: TextOverflow, Style;
    TextAlign, text_align: Align, Style;
    MaxLines, max_lines: usize, Style;
    LabelStyle, label_style: Option<LabelStyle>, Style;
    Uppercase, uppercase: bool, Style;
    LineHeight, line_height: Option<f64>, Style;
    LetterSpacing, letter_spacing: f64, Style;
    Foreground, foreground: Option<u32>, Style;
    Background, background: Option<u32>, Style;
    FontSize, font_size: Option<f64>, Style;
    Bold, bold: bool, Style;
    Radius, radius: f64, Style;
    Orientation, orientation: Orientation, Style;
}
trait Value: Sized {
    const SYNTAX: &'static str;
    fn display(self) -> String;
    fn parse(text: &str) -> Result<Self, String>;
}
macro_rules! number {
    ($ty:ty, $syntax:literal) => {
        impl Value for $ty {
            const SYNTAX: &'static str = $syntax;
            fn display(self) -> String {
                self.to_string()
            }
            fn parse(text: &str) -> Result<Self, String> {
                text.parse().map_err(|_| format!("Use {}", Self::SYNTAX))
            }
        }
    };
}
number!(f64, "points; inf for an unbounded maximum");
number!(i32, "an integer");
number!(usize, "a nonnegative integer; 0 means unlimited");
impl Value for u32 {
    const SYNTAX: &'static str = "#RRGGBB or #RRGGBBAA";
    fn display(self) -> String {
        Color(self).to_string()
    }
    fn parse(text: &str) -> Result<Self, String> {
        text.parse::<Color>().map(|c| c.0)
    }
}
impl<T: Value> Value for Option<T> {
    const SYNTAX: &'static str = T::SYNTAX;
    fn display(self) -> String {
        self.map_or_else(|| "inherit".into(), Value::display)
    }
    fn parse(text: &str) -> Result<Self, String> {
        if text == "inherit" {
            Ok(None)
        } else {
            T::parse(text).map(Some)
        }
    }
}
impl Value for Length {
    const SYNTAX: &'static str = "auto, points or percent (e.g. 50%)";
    fn display(self) -> String {
        match self {
            Self::Auto => "auto".into(),
            Self::Points(v) => v.to_string(),
            Self::Percent(v) => format!("{}%", v * 100.),
        }
    }
    fn parse(text: &str) -> Result<Self, String> {
        if text == "auto" {
            Ok(Self::Auto)
        } else if let Some(v) = text.strip_suffix('%') {
            f64::parse(v).map(|v| Self::Percent(v / 100.))
        } else {
            f64::parse(text.strip_suffix("pt").unwrap_or(text)).map(Self::Points)
        }
    }
}
impl Value for Insets {
    const SYNTAX: &'static str = "1, 2 or 4 points: top right bottom left";
    fn display(self) -> String {
        format!("{} {} {} {}", self.top, self.right, self.bottom, self.left)
    }
    fn parse(text: &str) -> Result<Self, String> {
        let values: Vec<f64> = text
            .split_whitespace()
            .map(f64::parse)
            .collect::<Result<_, _>>()?;
        match values.as_slice() {
            [all] => Ok(Self::all(*all)),
            [vertical, horizontal] => Ok(Self::symmetric(*vertical, *horizontal)),
            [top, right, bottom, left] => Ok(Self {
                top: *top,
                right: *right,
                bottom: *bottom,
                left: *left,
            }),
            _ => Err(format!("Use {}", Self::SYNTAX)),
        }
    }
}
macro_rules! choices {
    ($ty:ty; $($name:literal => $variant:ident),+ $(,)?) => { impl Value for $ty {
        const SYNTAX: &'static str = concat!($($name, " "),+);
        fn display(self) -> String { match self { $(Self::$variant => $name.into(),)+ } }
        fn parse(text: &str) -> Result<Self, String> { match text { $($name => Ok(Self::$variant),)+ _ => Err(format!("Use {}", Self::SYNTAX)) } }
    } };
}
impl Value for bool {
    const SYNTAX: &'static str = "true or false";
    fn display(self) -> String {
        self.to_string()
    }
    fn parse(text: &str) -> Result<Self, String> {
        text.parse().map_err(|_| "Use true or false".into())
    }
}
choices!(Align; "start" => Start, "center" => Center, "end" => End, "stretch" => Stretch, "baseline" => Baseline);
choices!(Justify; "start" => Start, "center" => Center, "end" => End, "space-between" => SpaceBetween, "space-around" => SpaceAround);
choices!(Position; "flow" => Flow, "relative" => Relative, "absolute" => Absolute);
choices!(Orientation; "horizontal" => Horizontal, "vertical" => Vertical);
choices!(TextWrap; "nowrap" => NoWrap, "wrap" => Wrap, "break-word" => BreakWord);
choices!(TextOverflow; "clip" => Clip, "ellipsis" => Ellipsis);
choices!(Density; "compact" => Compact, "standard" => Standard, "spacious" => Spacious);
choices!(LabelStyle; "standard" => Standard, "heading" => Heading, "section" => Section, "secondary" => Secondary, "micro" => Micro, "status" => Status, "error" => Error);
