//! One RGB/HSV/hex session, with explicit publication and owned-preview rollback.
use crate::{Binding, Color, Error, OptionItem, State, Validation, ValidationStatus, View};
use std::{cell::Cell, rc::Rc};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ColorModel {
    #[default]
    Rgb,
    Hsv,
}
#[derive(Clone, Copy)]
pub(crate) enum ColorAction {
    Apply,
    Cancel,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ColorChannel {
    Red,
    Green,
    Blue,
    Hue,
    Saturation,
    Value,
    Alpha,
}
impl ColorChannel {
    fn label(self) -> &'static str {
        match self {
            Self::Red => "Red",
            Self::Green => "Green",
            Self::Blue => "Blue",
            Self::Hue => "Hue",
            Self::Saturation => "Saturation",
            Self::Value => "Value",
            Self::Alpha => "Alpha",
        }
    }
    fn max(self) -> f64 {
        match self {
            Self::Hue => 360.,
            Self::Saturation | Self::Value => 100.,
            _ => 255.,
        }
    }
    fn byte(self) -> bool {
        !matches!(self, Self::Hue | Self::Saturation | Self::Value)
    }
}
/// HSV over encoded sRGB: hue in degrees 0–360, saturation/value in 0–1.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Hsv {
    pub hue: f64,
    pub saturation: f64,
    pub value: f64,
}
impl Hsv {
    pub fn new(hue: f64, saturation: f64, value: f64) -> Result<Self, Error> {
        if !hue.is_finite()
            || !saturation.is_finite()
            || !value.is_finite()
            || !(0. ..=360.).contains(&hue)
            || !(0. ..=1.).contains(&saturation)
            || !(0. ..=1.).contains(&value)
        {
            return Err(Error(
                "HSV requires finite hue 0–360 and saturation/value 0–1".into(),
            ));
        }
        Ok(Self {
            hue,
            saturation,
            value,
        })
    }
}
impl Color {
    pub fn hsv(self) -> Hsv {
        let [r, g, b, _] = self.channels();
        let (r, g, b) = (r as f64 / 255., g as f64 / 255., b as f64 / 255.);
        let max = r.max(g).max(b);
        let min = r.min(g).min(b);
        let delta = max - min;
        let hue = if delta == 0. {
            0.
        } else if max == r {
            60. * ((g - b) / delta).rem_euclid(6.)
        } else if max == g {
            60. * ((b - r) / delta + 2.)
        } else {
            60. * ((r - g) / delta + 4.)
        };
        Hsv {
            hue,
            saturation: if max == 0. { 0. } else { delta / max },
            value: max,
        }
    }
    pub fn from_hsv(hsv: Hsv, alpha: u8) -> Result<Self, Error> {
        Hsv::new(hsv.hue, hsv.saturation, hsv.value)?;
        let h = hsv.hue.rem_euclid(360.) / 60.;
        let c = hsv.value * hsv.saturation;
        let x = c * (1. - (h.rem_euclid(2.) - 1.).abs());
        let m = hsv.value - c;
        let rgb = match h as u8 {
            0 => [c, x, 0.],
            1 => [x, c, 0.],
            2 => [0., c, x],
            3 => [0., x, c],
            4 => [x, 0., c],
            _ => [c, 0., x],
        };
        let rgb = rgb.map(|v| ((v + m) * 255.).round().clamp(0., 255.) as u8);
        Ok(Self::rgba(rgb[0], rgb[1], rgb[2], alpha))
    }
}
#[derive(Clone, Copy, PartialEq, Eq)]
struct Config {
    alpha: bool,
    live: bool,
    readonly: bool,
    initial: ColorModel,
}
#[derive(Clone, Copy, PartialEq, Eq)]
struct Stamp {
    value: Color,
    revision: Option<u64>,
}
fn stamp(value: &Binding<Color>) -> Stamp {
    Stamp {
        value: value.get(),
        revision: value.revision(),
    }
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum Form {
    Hex,
    Channels,
}
#[derive(Clone, PartialEq)]
struct Edit {
    cfg: Config,
    base: Color,
    base_hsv: Hsv,
    last: Stamp,
    color: Color,
    hsv: Hsv,
    model: ColorModel,
    fields: [String; 4],
    fields_edited: u8,
    hex: String,
    form: Form,
    active: bool,
    owned: bool,
    rejected: Option<String>,
}
fn channels(model: ColorModel) -> [ColorChannel; 4] {
    match model {
        ColorModel::Rgb => [
            ColorChannel::Red,
            ColorChannel::Green,
            ColorChannel::Blue,
            ColorChannel::Alpha,
        ],
        ColorModel::Hsv => [
            ColorChannel::Hue,
            ColorChannel::Saturation,
            ColorChannel::Value,
            ColorChannel::Alpha,
        ],
    }
}
fn hex(color: Color, alpha: bool) -> String {
    let text = color.to_string();
    if alpha { text } else { text[..7].into() }
}
fn number(value: f64) -> String {
    format!("{value:.3}")
        .trim_end_matches('0')
        .trim_end_matches('.')
        .to_owned()
}
fn validation(status: ValidationStatus, message: Option<String>, dirty: bool) -> Validation {
    Validation {
        status,
        message,
        dirty,
    }
}
fn parse_channel(channel: ColorChannel, text: &str) -> Result<f64, Validation> {
    let fail = |s, msg: String| {
        Err(validation(
            s,
            Some(format!("{}: {msg}", channel.label())),
            true,
        ))
    };
    if <f64 as crate::draft::Number>::incomplete(text) {
        return fail(
            ValidationStatus::Incomplete,
            "Enter a complete value".into(),
        );
    }
    let Ok(value) = text.parse::<f64>() else {
        return fail(ValidationStatus::Invalid, "Use an ASCII number".into());
    };
    if !value.is_finite() {
        return fail(ValidationStatus::Invalid, "Use a finite value".into());
    }
    if value < 0. || value > channel.max() {
        return fail(
            ValidationStatus::OutOfRange,
            format!("Enter 0–{}", channel.max()),
        );
    }
    if channel.byte() && value.fract() != 0. {
        return fail(ValidationStatus::Invalid, "Use a whole channel byte".into());
    }
    Ok(value)
}
impl Edit {
    fn fresh(cfg: Config, last: Stamp, model: ColorModel) -> Self {
        let hsv = last.value.hsv();
        let mut e = Self {
            cfg,
            base: last.value,
            base_hsv: hsv,
            last,
            color: last.value,
            hsv,
            model,
            fields: Default::default(),
            fields_edited: 0,
            hex: String::new(),
            form: Form::Hex,
            active: false,
            owned: false,
            rejected: None,
        };
        e.canonical();
        e
    }
    fn component(&self, c: ColorChannel) -> f64 {
        let [r, g, b, a] = self.color.channels();
        match c {
            ColorChannel::Red => r as f64,
            ColorChannel::Green => g as f64,
            ColorChannel::Blue => b as f64,
            ColorChannel::Alpha => a as f64,
            ColorChannel::Hue => self.hsv.hue,
            ColorChannel::Saturation => self.hsv.saturation * 100.,
            ColorChannel::Value => self.hsv.value * 100.,
        }
    }
    fn canonical(&mut self) {
        self.fields = channels(self.model).map(|c| number(self.component(c)));
        self.fields_edited = 0;
        self.hex = hex(self.color, self.cfg.alpha);
    }
    fn derive(&mut self, color: Color) {
        let mut hsv = color.hsv();
        if hsv.saturation == 0. {
            hsv.hue = self.hsv.hue;
        }
        if hsv.value == 0. {
            hsv.saturation = self.hsv.saturation;
        }
        self.color = color;
        self.hsv = hsv;
    }
    fn raw_status(&self) -> Validation {
        let dirty = self.color != self.base
            || self.hsv != self.base_hsv
            || match self.form {
                Form::Hex => self.hex != hex(self.color, self.cfg.alpha),
                Form::Channels => channels(self.model).iter().enumerate().any(|(i, c)| {
                    (i != 3 || self.cfg.alpha) && self.fields[i] != number(self.component(*c))
                }),
            };
        match self.form {
            Form::Hex => {
                let message = if !self.cfg.alpha && self.hex.len() == 9 {
                    Some("Alpha hex input is disabled".into())
                } else {
                    self.hex.parse::<Color>().err()
                };
                if let Some(message) = message {
                    return validation(
                        if self.hex.is_empty()
                            || self.hex.strip_prefix('#').is_some_and(|s| {
                                s.len() < if self.cfg.alpha { 8 } else { 6 }
                                    && s.bytes().all(|b| b.is_ascii_hexdigit())
                            })
                        {
                            ValidationStatus::Incomplete
                        } else {
                            ValidationStatus::Invalid
                        },
                        Some(message),
                        true,
                    );
                }
            }
            Form::Channels => {
                for (i, c) in channels(self.model)
                    .iter()
                    .enumerate()
                    .take(if self.cfg.alpha { 4 } else { 3 })
                {
                    if let Err(v) = parse_channel(*c, &self.fields[i]) {
                        return v;
                    }
                }
            }
        }
        validation(ValidationStatus::Valid, None, dirty)
    }
    fn status(&self) -> Validation {
        let v = self.raw_status();
        if v.status == ValidationStatus::Valid
            && let Some(message) = &self.rejected
        {
            validation(ValidationStatus::Invalid, Some(message.clone()), true)
        } else {
            v
        }
    }
    fn collect(&mut self) -> bool {
        if self.raw_status().status != ValidationStatus::Valid {
            return false;
        }
        match self.form {
            Form::Hex => {
                let mut color = self.hex.parse::<Color>().unwrap();
                if !self.cfg.alpha {
                    color = Color((color.0 & 0x00ffffff) | (self.color.0 & 0xff000000));
                }
                self.derive(color);
                self.fields = channels(self.model).map(|c| number(self.component(c)));
                self.fields_edited = 0;
            }
            Form::Channels => {
                let cs = channels(self.model);
                let mut v = [0.; 4];
                for i in 0..if self.cfg.alpha { 4 } else { 3 } {
                    v[i] = if self.fields_edited & (1 << i) != 0 {
                        parse_channel(cs[i], &self.fields[i]).unwrap()
                    } else {
                        self.component(cs[i])
                    };
                }
                let alpha = if self.cfg.alpha {
                    v[3] as u8
                } else {
                    self.color.channels()[3]
                };
                match self.model {
                    ColorModel::Rgb => {
                        self.derive(Color::rgba(v[0] as u8, v[1] as u8, v[2] as u8, alpha))
                    }
                    ColorModel::Hsv => {
                        self.hsv = Hsv::new(v[0], v[1] / 100., v[2] / 100.).unwrap();
                        self.color = Color::from_hsv(self.hsv, alpha).unwrap();
                    }
                }
                self.hex = hex(self.color, self.cfg.alpha);
            }
        }
        true
    }
}
struct Controller {
    source: Binding<Color>,
    edit: State<Edit>,
}
impl Drop for Controller {
    fn drop(&mut self) {
        let e = self.edit.get();
        let source = self.source.clone();
        if e.owned {
            crate::state::cleanup(move || {
                if stamp(&source) == e.last {
                    source.set(e.base);
                }
            });
        }
    }
}
/// Read-only inspection of one shared color edit session.
#[derive(Clone, Debug, PartialEq)]
pub struct ColorInfo {
    pub source: Color,
    pub original: Color,
    pub draft: Color,
    pub hsv: Hsv,
    pub model: ColorModel,
    pub alpha: bool,
    pub live_preview: bool,
    pub validation: Validation,
}
#[derive(Clone)]
pub struct ColorPicker {
    inner: Rc<Controller>,
    cfg: Config,
    notify: Option<Rc<dyn Fn(Validation)>>,
    notifying: Rc<Cell<bool>>,
    submit: Option<Rc<dyn Fn()>>,
    cancel: Option<Rc<dyn Fn()>>,
}
impl ColorPicker {
    pub fn new(value: Binding<Color>) -> Self {
        let cfg = Config {
            alpha: true,
            live: false,
            readonly: false,
            initial: ColorModel::Rgb,
        };
        let edit = State::new(Edit::fresh(cfg, stamp(&value), cfg.initial));
        Self {
            inner: Rc::new(Controller {
                source: value,
                edit,
            }),
            cfg,
            notify: None,
            notifying: Rc::new(Cell::new(false)),
            submit: None,
            cancel: None,
        }
    }
    fn edit(&self) -> Edit {
        let e = self.inner.edit.get();
        let current = stamp(&self.inner.source);
        if e.cfg != self.cfg {
            Edit::fresh(
                self.cfg,
                current,
                if e.cfg.initial == self.cfg.initial {
                    e.model
                } else {
                    self.cfg.initial
                },
            )
        } else if e.last != current {
            Edit::fresh(self.cfg, current, e.model)
        } else {
            e
        }
    }
    fn configure(mut self, update: impl FnOnce(&mut Config)) -> Self {
        let old = self.clone();
        update(&mut self.cfg);
        if old.cfg != self.cfg && self.inner.edit.get().active {
            let version = self.inner.edit.version();
            crate::state::cleanup(move || {
                if old.inner.edit.version() == version {
                    old.retire();
                }
            });
        }
        self
    }
    pub fn alpha(self, enabled: bool) -> Self {
        self.configure(|c| c.alpha = enabled)
    }
    pub fn live_preview(self, enabled: bool) -> Self {
        self.configure(|c| c.live = enabled)
    }
    pub fn read_only(self, enabled: bool) -> Self {
        self.configure(|c| c.readonly = enabled)
    }
    pub fn model(self, model: ColorModel) -> Self {
        self.configure(|c| c.initial = model)
    }
    pub fn on_validation(mut self, f: impl Fn(Validation) + 'static) -> Self {
        self.notify = Some(Rc::new(f));
        self
    }
    pub fn on_submit(mut self, f: impl Fn() + 'static) -> Self {
        self.submit = Some(Rc::new(f));
        self
    }
    pub fn on_cancel(mut self, f: impl Fn() + 'static) -> Self {
        self.cancel = Some(Rc::new(f));
        self
    }
    fn notify(&self) {
        if let Some(f) = &self.notify {
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
            f(self.validation());
        }
    }
    pub fn info(&self) -> ColorInfo {
        let e = self.edit();
        ColorInfo {
            source: self.inner.source.get(),
            original: e.base,
            draft: e.color,
            hsv: e.hsv,
            model: e.model,
            alpha: self.cfg.alpha,
            live_preview: self.cfg.live,
            validation: e.status(),
        }
    }
    pub fn validation(&self) -> Validation {
        self.edit().status()
    }
    pub fn component(&self, channel: ColorChannel) -> f64 {
        self.edit().component(channel)
    }
    pub fn binding(&self) -> Binding<String> {
        let (read, write, revision) = (self.clone(), self.clone(), self.clone());
        Binding::new(
            move || read.edit().hex,
            move |text| {
                if write.cfg.readonly {
                    return;
                }
                crate::state::batch(|| {
                    let mut e = write.edit();
                    if e.form == Form::Channels && e.raw_status().status != ValidationStatus::Valid
                    {
                        return;
                    }
                    e.form = Form::Hex;
                    e.hex = text;
                    write.stage(e, true);
                });
            },
        )
        .with_revision(move || revision.revision())
    }
    fn revision(&self) -> u64 {
        self.inner
            .edit
            .version()
            .wrapping_add(self.inner.source.revision().unwrap_or(0))
    }
    fn field_binding(&self, index: usize) -> Binding<String> {
        let (read, write, revision) = (self.clone(), self.clone(), self.clone());
        Binding::new(
            move || read.edit().fields[index].clone(),
            move |text| {
                if write.cfg.readonly {
                    return;
                }
                crate::state::batch(|| {
                    let mut e = write.edit();
                    if e.form == Form::Hex && e.raw_status().status != ValidationStatus::Valid {
                        return;
                    }
                    e.form = Form::Channels;
                    e.fields[index] = text;
                    e.fields_edited |= 1 << index;
                    write.stage(e, true);
                });
            },
        )
        .with_revision(move || revision.revision())
    }
    fn stage(&self, mut e: Edit, collect: bool) -> bool {
        e.active = true;
        e.rejected = None;
        let valid = !collect || e.collect();
        self.inner.edit.set(e);
        let accepted = valid && (!self.cfg.live || self.publish(false));
        let version = self.inner.edit.version();
        let source = stamp(&self.inner.source);
        self.notify();
        accepted && self.inner.edit.version() == version && stamp(&self.inner.source) == source
    }
    fn publish(&self, finalize: bool) -> bool {
        let mut e = self.edit();
        if e.raw_status().status != ValidationStatus::Valid || self.cfg.readonly {
            return false;
        }
        let version = self.inner.edit.version();
        let before = stamp(&self.inner.source);
        if before.value != e.color {
            self.inner.source.set(e.color);
        }
        let accepted = stamp(&self.inner.source);
        if self.inner.edit.version() != version {
            return false;
        }
        if accepted == before && accepted.value != e.color {
            e.rejected = Some("The application rejected the color; Apply can retry".into());
            self.inner.edit.set(e);
            return false;
        }
        if accepted.value != e.color {
            e.derive(accepted.value);
            e.canonical();
        }
        e.last = accepted;
        e.rejected = None;
        if finalize {
            e.base = e.color;
            e.base_hsv = e.hsv;
            e.owned = false;
            e.active = false;
            e.canonical();
        } else {
            e.owned = true;
        }
        self.inner.edit.set(e);
        true
    }
    pub fn commit(&self) -> bool {
        if self.cfg.readonly {
            return false;
        }
        crate::state::batch(|| {
            let accepted = self.publish(true);
            let version = self.inner.edit.version();
            let source = stamp(&self.inner.source);
            self.notify();
            accepted
                && self.inner.edit.version() == version
                && stamp(&self.inner.source) == source
                && self.validation().status == ValidationStatus::Valid
        })
    }
    pub fn cancel(&self) {
        crate::state::batch(|| {
            let e = self.edit();
            let before = self.inner.edit.version();
            if e.owned && stamp(&self.inner.source) == e.last {
                self.inner.source.set(e.base);
            }
            if self.inner.edit.version() != before {
                return;
            }
            let accepted = stamp(&self.inner.source);
            let mut fresh = Edit::fresh(self.cfg, accepted, e.model);
            if accepted.value == e.base {
                fresh.hsv = e.base_hsv;
                fresh.base_hsv = e.base_hsv;
                fresh.canonical();
            }
            self.inner.edit.set(fresh);
            self.notify();
            if let Some(f) = &self.cancel {
                f()
            }
        })
    }
    pub(crate) fn retire(&self) {
        let e = self.inner.edit.get();
        if e.cfg == self.cfg && e.active {
            self.cancel();
        }
    }
    pub(crate) fn same_session(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.inner, &other.inner) && self.cfg == other.cfg
    }
    pub fn set_model(&self, model: ColorModel) -> bool {
        if self.cfg.readonly {
            return false;
        }
        crate::state::batch(|| {
            let mut e = self.edit();
            if e.raw_status().status != ValidationStatus::Valid {
                return false;
            }
            e.model = model;
            e.canonical();
            self.inner.edit.set(e);
            true
        })
    }
    pub fn set_component(&self, channel: ColorChannel, value: f64) -> bool {
        if self.cfg.readonly
            || !value.is_finite()
            || value < 0.
            || value > channel.max()
            || channel.byte() && value.fract() != 0.
            || channel == ColorChannel::Alpha && !self.cfg.alpha
        {
            return false;
        }
        crate::state::batch(|| {
            let mut e = self.edit();
            if e.raw_status().status != ValidationStatus::Valid {
                return false;
            }
            match channel {
                ColorChannel::Hue | ColorChannel::Saturation | ColorChannel::Value => {
                    match channel {
                        ColorChannel::Hue => e.hsv.hue = value,
                        ColorChannel::Saturation => e.hsv.saturation = value / 100.,
                        _ => e.hsv.value = value / 100.,
                    }
                    e.color = Color::from_hsv(e.hsv, e.color.channels()[3]).unwrap();
                }
                _ => {
                    let index = match channel {
                        ColorChannel::Red => 0,
                        ColorChannel::Green => 1,
                        ColorChannel::Blue => 2,
                        _ => 3,
                    };
                    let mut cs = e.color.channels();
                    cs[index] = value as u8;
                    e.derive(Color::rgba(cs[0], cs[1], cs[2], cs[3]));
                }
            }
            e.form = Form::Channels;
            e.canonical();
            self.stage(e, false)
        })
    }
    fn submit(&self) {
        if self.commit()
            && let Some(f) = &self.submit
        {
            f()
        }
    }
    pub fn view(&self) -> View {
        let e = self.edit();
        let (mode_read, mode_write) = (self.clone(), self.clone());
        let mode = Binding::new(
            move || {
                if mode_read.edit().model == ColorModel::Rgb {
                    "rgb".into()
                } else {
                    "hsv".into()
                }
            },
            move |value: String| {
                mode_write.set_model(if value == "hsv" {
                    ColorModel::Hsv
                } else {
                    ColorModel::Rgb
                });
            },
        );
        let mut preview = View::panel(vec![])
            .height(56)
            .accessible_label(format!("Color preview {}", e.color));
        Rc::make_mut(&mut preview.0).color_preview = Some(e.color);
        let valid = e.raw_status().status == ValidationStatus::Valid;
        let mut children = vec![
            preview,
            View::segmented(
                mode,
                vec![OptionItem::new("rgb", "RGB"), OptionItem::new("hsv", "HSV")],
            )
            .enabled(!self.cfg.readonly && valid)
            .accessible_label("Color model"),
        ];
        for (index, c) in channels(e.model)
            .into_iter()
            .enumerate()
            .take(if self.cfg.alpha { 4 } else { 3 })
        {
            let (read, write, submit, cancel) =
                (self.clone(), self.clone(), self.clone(), self.clone());
            let value = Binding::new(
                move || read.component(c),
                move |v| {
                    write.set_component(c, v);
                },
            );
            let field_status = parse_channel(c, &e.fields[index]).map_or_else(
                |v| v,
                |_| validation(ValidationStatus::Valid, None, e.raw_status().dirty),
            );
            children.push(
                View::row(vec![
                    View::label(c.label()).width(82),
                    View::slider(
                        value,
                        0.,
                        c.max(),
                        if c.byte() || c == ColorChannel::Hue {
                            1.
                        } else {
                            0.1
                        },
                    )
                    .grow(1.)
                    .accessible_label(c.label())
                    .enabled(!self.cfg.readonly && valid),
                    View::text_field(c.label(), self.field_binding(index))
                        .width(100)
                        .accessible_label(if c == ColorChannel::Value {
                            "Value percentage".into()
                        } else {
                            format!("{} value", c.label())
                        })
                        .validation(field_status)
                        .read_only(self.cfg.readonly || !valid && e.form == Form::Hex)
                        .on_submit(move || submit.submit())
                        .on_cancel(move || cancel.cancel())
                        .key(format!("$color-{}", c.label())),
                ])
                .gap(10.)
                .key(format!("$color-row-{}", c.label())),
            );
        }
        let (submit, cancel, apply, revert) =
            (self.clone(), self.clone(), self.clone(), self.clone());
        children.push(
            View::text_field(
                if self.cfg.alpha {
                    "#RRGGBB or #RRGGBBAA"
                } else {
                    "#RRGGBB"
                },
                self.binding(),
            )
            .key("$color-hex")
            .accessible_label("Hex color")
            .validation(e.status())
            .read_only(self.cfg.readonly || !valid && e.form == Form::Channels)
            .on_submit(move || submit.submit())
            .on_cancel(move || cancel.cancel()),
        );
        let mut apply_button =
            View::button("Apply", move || apply.submit()).enabled(!self.cfg.readonly && valid);
        Rc::make_mut(&mut apply_button.0).color_action = Some(ColorAction::Apply);
        let mut cancel_button = View::button("Cancel", move || revert.cancel())
            .value_cancel()
            .enabled(!self.cfg.readonly);
        Rc::make_mut(&mut cancel_button.0).color_action = Some(ColorAction::Cancel);
        children.push(View::row(vec![apply_button, cancel_button]));
        if let Some(error) = e.status().message {
            children.push(View::label(error).wrap(true).foreground(0xffff8787));
        }
        let mut root = View::column(children)
            .accessible_label("Color editor")
            .value_editor();
        Rc::make_mut(&mut root.0).color_picker = Some(self.clone());
        root
    }
    pub fn well(&self, popup: &crate::Popup, key: &str) -> (View, View) {
        let (open, begin) = (popup.clone(), self.clone());
        let color = self.inner.source.get();
        let [r, g, b, a] = color.channels();
        let alpha = a as f64 / 255.;
        let luminance =
            alpha * (0.299 * r as f64 + 0.587 * g as f64 + 0.114 * b as f64) + (1. - alpha) * 224.;
        let mut button = View::button(color.to_string(), move || {
            begin.retire();
            open.show();
        })
        .key(key)
        .accessible_label("Color")
        .foreground(if luminance > 140. {
            0xff111111
        } else {
            0xffffffff
        })
        .enabled(!self.cfg.readonly);
        Rc::make_mut(&mut button.0).color_well = Some(color);
        Rc::make_mut(&mut button.0).color_preview = Some(color);
        let (close, generation, owner) = (self.clone(), popup.generation(), popup.clone());
        let decorated = popup.clone().before_dismiss(move |_| {
            if owner.generation() == generation {
                close.retire();
            }
        });
        let mut editor = self.clone();
        let (submit, close_submit, cancel, close_cancel) = (
            self.submit.clone(),
            popup.clone(),
            self.cancel.clone(),
            popup.clone(),
        );
        editor.submit = Some(Rc::new(move || {
            if close_submit.generation() == generation {
                close_submit.dismiss();
            }
            if let Some(f) = &submit {
                f()
            }
        }));
        editor.cancel = Some(Rc::new(move || {
            if close_cancel.generation() == generation {
                close_cancel.dismiss();
            }
            if let Some(f) = &cancel {
                f()
            }
        }));
        (
            button,
            decorated
                .popover(
                    key,
                    View::group_box("", vec![editor.view()])
                        .padding(16)
                        .width(440),
                )
                .accessible_label("Color"),
        )
    }
}

impl crate::Host {
    fn color_owner(&self, id: crate::NodeId) -> Option<(crate::NodeId, ColorPicker)> {
        let mounted = self.0.mounted.borrow();
        let mut current = Some(id);
        while let Some(id) = current {
            if let Some(picker) = mounted
                .descriptions
                .get(&id)
                .and_then(|d| d.color_picker.clone())
            {
                return Some((id, picker));
            }
            current = mounted.nodes.iter().find(|n| n.id == id)?.parent;
        }
        None
    }
    pub fn color_validation(&self, id: crate::NodeId) -> Result<Option<Validation>, Error> {
        self.refresh()?;
        Ok(self.color_owner(id).map(|(_, p)| p.validation()))
    }
    pub fn apply_color(&self, id: crate::NodeId) -> Result<bool, Error> {
        self.color_action(id, ColorAction::Apply)
    }
    pub fn cancel_color(&self, id: crate::NodeId) -> Result<bool, Error> {
        self.color_action(id, ColorAction::Cancel)
    }
    pub(crate) fn color_action(
        &self,
        id: crate::NodeId,
        action: ColorAction,
    ) -> Result<bool, Error> {
        self.refresh()?;
        let Some((root, picker)) = self.color_owner(id) else {
            return Ok(false);
        };
        let nodes = self.0.mounted.borrow().nodes.clone();
        if picker.cfg.readonly
            || !self.in_scope(root)
            || !nodes
                .iter()
                .any(|n| n.id == root && n.enabled && !n.hidden && !n.layout_hidden)
        {
            return Ok(false);
        }
        if matches!(action, ColorAction::Apply)
            && self
                .0
                .editors
                .borrow()
                .iter()
                .any(|(id, e)| e.marked.is_some() && Self::within(&nodes, *id, root))
        {
            return Ok(false);
        }
        match action {
            ColorAction::Apply => {
                let accepted = picker.commit();
                if accepted && let Some(f) = &picker.submit {
                    f();
                }
                Ok(accepted)
            }
            ColorAction::Cancel => {
                if self
                    .focused()
                    .is_some_and(|id| Self::within(&nodes, id, root))
                {
                    self.unmark_text(false)?;
                }
                picker.cancel();
                Ok(true)
            }
        }
    }
    pub(crate) fn retire_colors(&self, old: &crate::host::Mounted) {
        let retired = old
            .descriptions
            .iter()
            .filter_map(|(id, d)| d.color_picker.as_ref().map(|p| (*id, p.clone())))
            .collect::<Vec<_>>();
        for (id, picker) in retired {
            let keep = {
                let next = self.0.mounted.borrow();
                next.nodes
                    .iter()
                    .any(|n| n.id == id && n.enabled && !n.hidden && !n.layout_hidden)
                    && next
                        .descriptions
                        .get(&id)
                        .and_then(|d| d.color_picker.as_ref())
                        .is_some_and(|p| picker.same_session(p))
            };
            if !keep {
                picker.retire();
            }
        }
    }
    pub(crate) fn reconcile_colors(&self) {
        let retired = {
            let m = self.0.mounted.borrow();
            m.nodes
                .iter()
                .filter(|n| !n.enabled || n.hidden || n.layout_hidden)
                .filter_map(|n| m.descriptions[&n.id].color_picker.clone())
                .collect::<Vec<_>>()
        };
        for picker in retired {
            picker.retire();
        }
    }
    pub(crate) fn retire_all_colors(&self) {
        let pickers = self
            .0
            .mounted
            .borrow()
            .descriptions
            .values()
            .filter_map(|d| d.color_picker.clone())
            .collect::<Vec<_>>();
        for picker in pickers {
            picker.retire();
        }
    }
}
