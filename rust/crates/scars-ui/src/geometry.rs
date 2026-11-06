#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Size {
    pub width: f64,
    pub height: f64,
}
impl Size {
    pub const fn new(width: f64, height: f64) -> Self {
        Self { width, height }
    }
}
/// Logical-point insets, clockwise from the top. Scalars apply to every edge.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Insets {
    pub top: f64,
    pub right: f64,
    pub bottom: f64,
    pub left: f64,
}
impl Insets {
    pub const fn all(n: f64) -> Self {
        Self {
            top: n,
            right: n,
            bottom: n,
            left: n,
        }
    }
    pub const fn symmetric(vertical: f64, horizontal: f64) -> Self {
        Self {
            top: vertical,
            right: horizontal,
            bottom: vertical,
            left: horizontal,
        }
    }
    pub fn horizontal(self) -> f64 {
        self.left + self.right
    }
    pub fn vertical(self) -> f64 {
        self.top + self.bottom
    }
    pub(crate) fn main(self, horizontal: bool) -> f64 {
        if horizontal {
            self.horizontal()
        } else {
            self.vertical()
        }
    }
    pub(crate) fn before(self, horizontal: bool) -> f64 {
        if horizontal { self.left } else { self.top }
    }
    pub(crate) fn after(self, horizontal: bool) -> f64 {
        if horizontal { self.right } else { self.bottom }
    }
}
impl From<f64> for Insets {
    fn from(n: f64) -> Self {
        Self::all(n)
    }
}
impl From<i32> for Insets {
    fn from(n: i32) -> Self {
        Self::all(n as f64)
    }
}
impl From<[f64; 4]> for Insets {
    fn from([top, right, bottom, left]: [f64; 4]) -> Self {
        Self {
            top,
            right,
            bottom,
            left,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}
impl Rect {
    pub const fn new(x: f64, y: f64, width: f64, height: f64) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }
    pub fn contains(self, p: Point) -> bool {
        p.x >= self.x && p.y >= self.y && p.x < self.x + self.width && p.y < self.y + self.height
    }
    pub fn inset(self, insets: impl Into<Insets>) -> Self {
        let i = insets.into();
        Self::new(
            self.x + i.left,
            self.y + i.top,
            (self.width - i.horizontal()).max(0.),
            (self.height - i.vertical()).max(0.),
        )
    }
    pub fn intersection(self, b: Self) -> Self {
        let x = self.x.max(b.x);
        let y = self.y.max(b.y);
        Self::new(
            x,
            y,
            (self.x + self.width).min(b.x + b.width).max(x) - x,
            (self.y + self.height).min(b.y + b.height).max(y) - y,
        )
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum Length {
    #[default]
    Auto,
    Points(f64),
    Percent(f64),
}
impl From<f64> for Length {
    fn from(v: f64) -> Self {
        Self::Points(v)
    }
}
impl From<i32> for Length {
    fn from(v: i32) -> Self {
        Self::Points(v as f64)
    }
}
pub fn percent(value: f64) -> Length {
    Length::Percent(value / 100.)
}
impl Length {
    pub(crate) fn resolve(self, available: f64) -> Option<f64> {
        match self {
            Self::Auto => None,
            Self::Points(v) => Some(v),
            Self::Percent(v) => available.is_finite().then_some(v * available),
        }
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Align {
    #[default]
    Start,
    Center,
    End,
    Stretch,
    Baseline,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Justify {
    #[default]
    Start,
    Center,
    End,
    SpaceBetween,
    SpaceAround,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Position {
    #[default]
    Flow,
    Relative,
    Absolute,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TextWrap {
    #[default]
    NoWrap,
    Wrap,
    BreakWord,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TextOverflow {
    #[default]
    Clip,
    Ellipsis,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LabelStyle {
    #[default]
    Standard,
    Heading,
    Section,
    Secondary,
    Micro,
    Status,
    Error,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Density {
    Compact,
    #[default]
    Standard,
    Spacious,
}
impl Density {
    pub fn control_height(self) -> f64 {
        match self {
            Self::Compact => 28.,
            Self::Standard => 34.,
            Self::Spacious => 44.,
        }
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Orientation {
    #[default]
    Horizontal,
    Vertical,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Style {
    pub width: Length,
    pub height: Length,
    pub min_width: f64,
    pub min_height: f64,
    pub max_width: f64,
    pub max_height: f64,
    pub padding: Insets,
    pub margin: Insets,
    pub basis: Length,
    pub align_self: Option<Align>,
    pub accent: Option<u32>,
    pub density: Option<Density>,
    pub text_wrap: Option<TextWrap>,
    pub text_overflow: TextOverflow,
    pub text_align: Align,
    pub max_lines: usize,
    pub label_style: Option<LabelStyle>,
    pub uppercase: bool,
    pub line_height: Option<f64>,
    pub letter_spacing: f64,
    pub orientation: Orientation,
    pub gap: f64,
    pub grow: f64,
    pub shrink: f64,
    pub wrap: bool,
    pub align: Align,
    pub justify: Justify,
    pub foreground: Option<u32>,
    pub background: Option<u32>,
    pub font_size: Option<f64>,
    pub bold: bool,
    pub radius: f64,
    pub position: Position,
    pub left: Length,
    pub top: Length,
    pub right: Length,
    pub bottom: Length,
    pub z_index: i32,
}
impl Default for Style {
    fn default() -> Self {
        Self {
            width: Length::Auto,
            height: Length::Auto,
            min_width: 0.,
            min_height: 0.,
            max_width: f64::INFINITY,
            max_height: f64::INFINITY,
            padding: Insets::default(),
            margin: Insets::default(),
            basis: Length::Auto,
            align_self: None,
            accent: None,
            density: None,
            text_wrap: None,
            text_overflow: TextOverflow::Clip,
            text_align: Align::Start,
            max_lines: 0,
            label_style: None,
            uppercase: false,
            line_height: None,
            letter_spacing: 0.,
            orientation: Orientation::Horizontal,
            gap: 8.,
            grow: 0.,
            shrink: 1.,
            wrap: false,
            align: Align::Stretch,
            justify: Justify::Start,
            foreground: None,
            background: None,
            font_size: None,
            bold: false,
            radius: 6.,
            position: Position::Flow,
            left: Length::Auto,
            top: Length::Auto,
            right: Length::Auto,
            bottom: Length::Auto,
            z_index: 0,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Theme {
    pub window: u32,
    pub panel: u32,
    pub text: u32,
    pub secondary: u32,
    pub field: u32,
    pub border: u32,
    pub accent: u32,
    pub selected: u32,
    pub font_size: f64,
    pub control_height: f64,
}
impl Default for Theme {
    fn default() -> Self {
        Self {
            window: 0xff101923,
            panel: 0xff1b2937,
            text: 0xffe7eff9,
            secondary: 0xff9aaec2,
            field: 0xff0c141d,
            border: 0xff415568,
            accent: 0xff65b0ff,
            selected: 0xff274d71,
            font_size: 15.,
            control_height: 34.,
        }
    }
}
impl Theme {
    pub fn light() -> Self {
        Self {
            window: 0xfff4f6f9,
            panel: 0xffe8edf3,
            text: 0xff172638,
            secondary: 0xff53677a,
            field: 0xffffffff,
            border: 0xffa5b3c1,
            accent: 0xff1261ac,
            selected: 0xffc8e1fb,
            ..Self::default()
        }
    }
}
