use crate::{Align, Binding, Bitmap, ContentMode, Justify, Length, Position, Style};
use std::rc::Rc;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Kind {
    Fragment,
    Column,
    Row,
    Block,
    Inline,
    Layer,
    Label,
    Button,
    Toggle,
    CheckBox,
    RadioGroup,
    Radio,
    Segmented,
    Segment,
    Switch,
    Slider,
    Progress,
    Image,
    Section,
    GroupBox,
    Panel,
    Separator,
    Spacer,
    TextField,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CheckState {
    Unchecked,
    Checked,
    Indeterminate,
}
#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    None,
    Bool(bool),
    Check(CheckState),
    Number(f64),
    Text(String),
}
impl Value {
    pub fn selected(&self) -> bool {
        matches!(self, Self::Bool(true) | Self::Check(CheckState::Checked))
    }
}
#[derive(Clone, Debug)]
pub struct OptionItem {
    pub id: String,
    pub label: String,
    pub enabled: bool,
}
impl OptionItem {
    pub fn new(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            enabled: true,
        }
    }
}
#[derive(Clone)]
pub(crate) enum Data {
    None,
    Bool(Binding<bool>),
    Check(Binding<CheckState>),
    Number(Binding<f64>, f64, f64, f64),
    Text(Binding<String>),
    Image(Bitmap, ContentMode),
}
impl Data {
    pub fn value(&self) -> Value {
        match self {
            Self::None | Self::Image(..) => Value::None,
            Self::Bool(b) => Value::Bool(b.get()),
            Self::Check(b) => Value::Check(b.get()),
            Self::Number(b, ..) => Value::Number(b.get()),
            Self::Text(b) => Value::Text(b.get()),
        }
    }
}
#[derive(Clone)]
pub struct View(pub(crate) Rc<Description>);
#[derive(Clone)]
pub(crate) struct Description {
    pub kind: Kind,
    pub key: Option<String>,
    pub text: String,
    pub children: Vec<View>,
    pub action: Option<Rc<dyn Fn()>>,
    pub data: Data,
    pub style: Style,
    pub enabled: bool,
    pub visible: bool,
    pub hidden: bool,
    pub accessible: String,
    pub read_only: bool,
    pub secure: bool,
    pub multiline: bool,
}
impl View {
    fn new(kind: Kind, text: String, children: Vec<Self>) -> Self {
        Self(Rc::new(Description {
            kind,
            key: None,
            text,
            children,
            action: None,
            data: Data::None,
            style: Style::default(),
            enabled: true,
            visible: true,
            hidden: false,
            accessible: String::new(),
            read_only: false,
            secure: false,
            multiline: false,
        }))
    }
    pub fn fragment(v: Vec<Self>) -> Self {
        Self::new(Kind::Fragment, String::new(), v)
    }
    pub fn column(v: Vec<Self>) -> Self {
        Self::new(Kind::Column, String::new(), v)
    }
    pub fn row(v: Vec<Self>) -> Self {
        Self::new(Kind::Row, String::new(), v)
    }
    pub fn block(v: Vec<Self>) -> Self {
        Self::new(Kind::Block, String::new(), v)
    }
    pub fn inline(v: Vec<Self>) -> Self {
        Self::new(Kind::Inline, String::new(), v)
            .wrap(true)
            .align(Align::Baseline)
    }
    pub fn layer(v: Vec<Self>) -> Self {
        Self::new(Kind::Layer, String::new(), v)
    }
    pub fn label(text: impl Into<String>) -> Self {
        Self::new(Kind::Label, text.into(), Vec::new())
    }
    pub fn button(text: impl Into<String>, action: impl Fn() + 'static) -> Self {
        let mut v = Self::new(Kind::Button, text.into(), Vec::new());
        Rc::make_mut(&mut v.0).action = Some(Rc::new(action));
        v
    }
    fn bound(kind: Kind, text: impl Into<String>, data: Data) -> Self {
        let mut v = Self::new(kind, text.into(), Vec::new());
        Rc::make_mut(&mut v.0).data = data;
        v
    }
    pub fn checkbox(text: impl Into<String>, value: Binding<bool>) -> Self {
        Self::bound(Kind::CheckBox, text, Data::Bool(value))
    }
    pub fn tri_checkbox(text: impl Into<String>, value: Binding<CheckState>) -> Self {
        Self::bound(Kind::CheckBox, text, Data::Check(value))
    }
    pub fn toggle(text: impl Into<String>, value: Binding<bool>) -> Self {
        Self::bound(Kind::Toggle, text, Data::Bool(value))
    }
    pub fn switch(text: impl Into<String>, value: Binding<bool>) -> Self {
        Self::bound(Kind::Switch, text, Data::Bool(value))
    }
    pub fn text_field(placeholder: impl Into<String>, value: Binding<String>) -> Self {
        Self::bound(Kind::TextField, placeholder, Data::Text(value))
    }
    pub fn password_field(placeholder: impl Into<String>, value: Binding<String>) -> Self {
        let mut v = Self::text_field(placeholder, value);
        Rc::make_mut(&mut v.0).secure = true;
        v
    }
    pub fn text_editor(placeholder: impl Into<String>, value: Binding<String>) -> Self {
        let mut v = Self::text_field(placeholder, value).height(140.);
        Rc::make_mut(&mut v.0).multiline = true;
        v
    }
    pub fn search_field(placeholder: impl Into<String>, value: Binding<String>) -> Self {
        Self::text_field(placeholder, value)
    }
    pub fn slider(value: Binding<f64>, min: f64, max: f64, step: f64) -> Self {
        Self::bound(Kind::Slider, "", Data::Number(value, min, max, step))
    }
    pub fn progress(value: Binding<f64>) -> Self {
        Self::bound(Kind::Progress, "", Data::Number(value, 0., 1., 0.))
    }
    pub fn indeterminate_progress() -> Self {
        Self::new(Kind::Progress, String::new(), Vec::new())
    }
    pub fn image(bitmap: Bitmap, mode: ContentMode) -> Self {
        Self::bound(Kind::Image, "", Data::Image(bitmap, mode))
    }
    fn options(
        kind: Kind,
        title: impl Into<String>,
        binding: Binding<String>,
        options: Vec<OptionItem>,
    ) -> Self {
        let children = options
            .into_iter()
            .map(|item| {
                let read = binding.clone();
                let write = binding.clone();
                let id = item.id.clone();
                let target = item.id.clone();
                let selected = Binding::new(
                    move || read.get() == id,
                    move |v| {
                        if v {
                            write.set(target.clone())
                        }
                    },
                );
                Self::bound(
                    if kind == Kind::RadioGroup {
                        Kind::Radio
                    } else {
                        Kind::Segment
                    },
                    item.label,
                    Data::Bool(selected),
                )
                .key(item.id)
                .enabled(item.enabled)
            })
            .collect();
        Self::new(kind, title.into(), children)
    }
    pub fn radio_group(
        title: impl Into<String>,
        binding: Binding<String>,
        options: Vec<OptionItem>,
    ) -> Self {
        Self::options(Kind::RadioGroup, title, binding, options)
    }
    pub fn segmented(binding: Binding<String>, options: Vec<OptionItem>) -> Self {
        Self::options(Kind::Segmented, "", binding, options).gap(2.)
    }
    pub fn section(title: impl Into<String>, children: Vec<Self>) -> Self {
        Self::new(Kind::Section, title.into(), children).padding(12.)
    }
    pub fn group_box(title: impl Into<String>, children: Vec<Self>) -> Self {
        Self::new(Kind::GroupBox, title.into(), children).padding(12.)
    }
    pub fn panel(children: Vec<Self>) -> Self {
        Self::new(Kind::Panel, String::new(), children).padding(12.)
    }
    pub fn separator() -> Self {
        Self::new(Kind::Separator, String::new(), Vec::new())
    }
    pub fn spacer() -> Self {
        Self::new(Kind::Spacer, String::new(), Vec::new()).grow(1.)
    }
    pub fn key(mut self, key: impl ToString) -> Self {
        Rc::make_mut(&mut self.0).key = Some(key.to_string());
        self
    }
    pub fn enabled(mut self, v: bool) -> Self {
        Rc::make_mut(&mut self.0).enabled = v;
        self
    }
    pub fn visible(mut self, v: bool) -> Self {
        Rc::make_mut(&mut self.0).visible = v;
        self
    }
    pub fn hidden(mut self, v: bool) -> Self {
        Rc::make_mut(&mut self.0).hidden = v;
        self
    }
    pub fn read_only(mut self, v: bool) -> Self {
        Rc::make_mut(&mut self.0).read_only = v;
        self
    }
    pub fn accessible_label(mut self, v: impl Into<String>) -> Self {
        Rc::make_mut(&mut self.0).accessible = v.into();
        self
    }
    pub fn on_submit(mut self, action: impl Fn() + 'static) -> Self {
        Rc::make_mut(&mut self.0).action = Some(Rc::new(action));
        self
    }
}
macro_rules! property {
    ($name:ident,$ty:ty) => {
        impl View {
            pub fn $name(mut self, v: $ty) -> Self {
                Rc::make_mut(&mut self.0).style.$name = v;
                self
            }
        }
    };
}
property!(padding, f64);
property!(margin, f64);
property!(gap, f64);
property!(grow, f64);
property!(shrink, f64);
property!(min_width, f64);
property!(min_height, f64);
property!(max_width, f64);
property!(max_height, f64);
property!(wrap, bool);
property!(align, Align);
property!(justify, Justify);
property!(bold, bool);
property!(radius, f64);
property!(position, Position);
property!(z_index, i32);
macro_rules! length {
    ($name:ident) => {
        impl View {
            pub fn $name(mut self, v: impl Into<Length>) -> Self {
                Rc::make_mut(&mut self.0).style.$name = v.into();
                self
            }
        }
    };
}
length!(width);
length!(height);
length!(left);
length!(right);
length!(top);
length!(bottom);
impl View {
    pub fn foreground(mut self, v: u32) -> Self {
        Rc::make_mut(&mut self.0).style.foreground = Some(v);
        self
    }
    pub fn background(mut self, v: u32) -> Self {
        Rc::make_mut(&mut self.0).style.background = Some(v);
        self
    }
    pub fn font_size(mut self, v: f64) -> Self {
        Rc::make_mut(&mut self.0).style.font_size = Some(v);
        self
    }
}
