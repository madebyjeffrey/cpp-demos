//! Portable Skia rendering for the component core. Native mounting is supplied by
//! the selected window backend through `WindowBuilder::view` / `Surface::set_view`.
use crate::skia::{
    self,
    textlayout::{
        FontCollection, Paragraph, ParagraphBuilder, ParagraphStyle, RectHeightStyle,
        RectWidthStyle, TextDirection, TextStyle as SkTextStyle,
    },
};
pub use scars_ui::*;
use std::{cell::RefCell, collections::HashMap, rc::Rc};
#[derive(Hash, PartialEq, Eq)]
struct TextKey {
    text: String,
    size: u64,
    color: u32,
    bold: bool,
    wrap: bool,
    width: u64,
}
struct Shaped {
    paragraph: Rc<RefCell<Paragraph>>,
    layout: TextLayout,
}
/// A reusable font/paragraph/image cache. Coordinates are logical points.
pub struct Renderer {
    fonts: FontCollection,
    family: Option<String>,
    text: RefCell<HashMap<TextKey, Shaped>>,
    images: RefCell<HashMap<u64, skia::Image>>,
}
impl Default for Renderer {
    fn default() -> Self {
        let mut fonts = FontCollection::new();
        fonts.set_default_font_manager(skia::FontMgr::new(), None);
        Self {
            fonts,
            family: None,
            text: RefCell::new(HashMap::new()),
            images: RefCell::new(HashMap::new()),
        }
    }
}
impl Renderer {
    pub fn with_font(bytes: &[u8]) -> Result<Self, Error> {
        let face = skia::FontMgr::new()
            .new_from_bytes(bytes, None)
            .ok_or_else(|| Error("Invalid font".into()))?;
        let mut provider = skia::textlayout::TypefaceFontProvider::new();
        provider.register_typeface(face, Some("SCARS"));
        let mut renderer = Self::default();
        renderer.fonts.set_asset_font_manager(Some(provider.into()));
        renderer.family = Some("SCARS".into());
        Ok(renderer)
    }
    fn key(text: &str, style: TextStyle, width: f64) -> TextKey {
        TextKey {
            text: text.into(),
            size: style.size.to_bits(),
            color: style.color,
            bold: style.bold,
            wrap: style.wrap,
            width: if style.wrap {
                width.max(1.).to_bits()
            } else {
                1_000_000f64.to_bits()
            },
        }
    }
    fn shaped(&self, text: &str, style: TextStyle, width: f64) -> Shaped {
        let key = Self::key(text, style, width);
        if let Some(shaped) = self.text.borrow().get(&key) {
            return Shaped {
                paragraph: shaped.paragraph.clone(),
                layout: shaped.layout.clone(),
            };
        }
        let mut ts = SkTextStyle::new();
        ts.set_font_size(style.size as f32)
            .set_color(skia::Color::new(style.color));
        if let Some(family) = &self.family {
            ts.set_font_families(&[family.as_str()]);
        }
        if style.bold {
            ts.set_font_style(skia::FontStyle::bold());
        }
        let mut ps = ParagraphStyle::new();
        ps.set_text_style(&ts);
        let mut builder = ParagraphBuilder::new(&ps, self.fonts.clone());
        builder.add_text(text);
        let mut paragraph = builder.build();
        paragraph.layout(f64::from_bits(key.width).min(1_000_000.) as f32);
        let size = Size::new(
            paragraph.longest_line() as f64,
            paragraph.height().max(style.size as f32 * 1.2) as f64,
        );
        let baseline = paragraph.alphabetic_baseline() as f64;
        let paragraph = Rc::new(RefCell::new(paragraph));
        let hit = paragraph.clone();
        let ranges = paragraph.clone();
        let source = text.to_owned();
        let source2 = source.clone();
        let font = style.size;
        let layout = TextLayout {
            size,
            baseline,
            hit_test: Rc::new(move |p| {
                let position = (hit
                    .borrow()
                    .get_glyph_position_at_coordinate((p.x as f32, p.y as f32))
                    .position
                    .max(0) as usize)
                    .min(source.encode_utf16().count());
                (0..=position)
                    .rev()
                    .find_map(|u| text::byte_offset(&source, u))
                    .unwrap_or(0)
            }),
            range_rects: Rc::new(move |range| {
                let start = text::utf16_offset(&source2, range.start).unwrap_or(0);
                let end = text::utf16_offset(&source2, range.end).unwrap_or(start);
                if start != end {
                    return ranges
                        .borrow()
                        .get_rects_for_range(
                            start..end,
                            RectHeightStyle::Tight,
                            RectWidthStyle::Tight,
                        )
                        .into_iter()
                        .map(|b| rect(b.rect))
                        .collect();
                }
                let mut p = ranges.borrow_mut();
                let units = source2.encode_utf16().count();
                if let Some(info) =
                    p.get_glyph_info_at_utf16_offset(start.min(units.saturating_sub(1)))
                {
                    let b = info.grapheme_layout_bounds;
                    let at_end = start >= units;
                    let rtl = info.text_direction == TextDirection::RTL;
                    let x = if at_end != rtl { b.right() } else { b.left() };
                    return vec![Rect::new(
                        x as f64,
                        b.top() as f64,
                        1.,
                        b.height().max(font as f32) as f64,
                    )];
                }
                vec![Rect::new(0., 0., 1., font * 1.2)]
            }),
        };
        if self.text.borrow().len() >= 512 {
            self.text.borrow_mut().clear();
        }
        self.text.borrow_mut().insert(
            key,
            Shaped {
                paragraph: paragraph.clone(),
                layout: layout.clone(),
            },
        );
        Shaped { paragraph, layout }
    }
    fn image(&self, bitmap: &Bitmap) -> Result<skia::Image, Error> {
        if let Some(image) = self.images.borrow().get(&bitmap.id()) {
            return Ok(image.clone());
        }
        let image = match bitmap.data.as_ref() {
            ImageData::Encoded(bytes) => skia::Image::from_encoded(skia::Data::new_copy(bytes))
                .and_then(|i| i.make_raster_image(None, None)),
            ImageData::Rgba {
                width,
                height,
                stride,
                pixels,
            } => skia::images::raster_from_data(
                &skia::ImageInfo::new(
                    (*width as i32, *height as i32),
                    skia::ColorType::RGBA8888,
                    skia::AlphaType::Unpremul,
                    skia::ColorSpace::new_srgb(),
                ),
                skia::Data::new_copy(pixels),
                *stride,
            ),
        }
        .ok_or_else(|| Error("Cannot decode image".into()))?;
        if self.images.borrow().len() >= 128 {
            self.images.borrow_mut().clear();
        }
        self.images.borrow_mut().insert(bitmap.id(), image.clone());
        Ok(image)
    }
    pub fn paint(&self, host: &Host, canvas: &skia::Canvas, elapsed: f64) -> Result<(), Error> {
        let theme = host.theme();
        let input = host.text_input()?;
        for n in host.scene()? {
            if n.hidden || n.clip.width <= 0. || n.clip.height <= 0. {
                continue;
            }
            canvas.save();
            canvas.clip_rect(skrect(n.clip), None, true);
            let b = n.bounds;
            let mut bg = n.style.background;
            let fg = if n.enabled {
                n.text_style.color
            } else {
                theme.secondary
            };
            match n.kind {
                Kind::Panel | Kind::GroupBox => bg = bg.or(Some(theme.panel)),
                Kind::Button | Kind::Toggle | Kind::Segment => {
                    bg = bg.or(Some(if n.value.selected() || n.pressed {
                        theme.selected
                    } else if n.hovered {
                        theme.border
                    } else {
                        theme.panel
                    }))
                }
                Kind::TextField => bg = bg.or(Some(theme.field)),
                _ => (),
            }
            if let Some(bg) = bg {
                round(canvas, b, n.style.radius, bg, false);
            }
            match n.kind {
                Kind::Button | Kind::Toggle | Kind::Segment | Kind::TextField | Kind::GroupBox => {
                    round(canvas, b.inset(0.5), n.style.radius, theme.border, true)
                }
                Kind::CheckBox | Kind::Radio => {
                    let r = Rect::new(b.x + 2., b.y + (b.height - 18.) / 2., 18., 18.);
                    let selected = n.value.selected();
                    round(
                        canvas,
                        r,
                        if n.kind == Kind::Radio { 9. } else { 3. },
                        if selected { theme.accent } else { theme.border },
                        !selected,
                    );
                    if selected {
                        if n.kind == Kind::Radio {
                            round(canvas, r.inset(5.), 4., theme.window, false);
                        } else {
                            let mut p = paint(theme.window);
                            p.set_style(skia::PaintStyle::Stroke).set_stroke_width(2.);
                            let mut path = skia::PathBuilder::new();
                            path.move_to((r.x as f32 + 4., r.y as f32 + 9.))
                                .line_to((r.x as f32 + 8., r.y as f32 + 13.))
                                .line_to((r.x as f32 + 15., r.y as f32 + 5.));
                            canvas.draw_path(&path.detach(), &p);
                        }
                    } else if n.value == Value::Check(CheckState::Indeterminate) {
                        round(canvas, r.inset(5.), 1., theme.accent, false);
                    }
                }
                Kind::Switch => {
                    let r = Rect::new(b.x, b.y + (b.height - 22.) / 2., 40., 22.);
                    round(
                        canvas,
                        r,
                        11.,
                        if n.value.selected() {
                            theme.accent
                        } else {
                            theme.border
                        },
                        false,
                    );
                    round(
                        canvas,
                        Rect::new(
                            r.x + if n.value.selected() { 20. } else { 2. },
                            r.y + 2.,
                            18.,
                            18.,
                        ),
                        9.,
                        theme.field,
                        false,
                    );
                }
                Kind::Slider => {
                    let (min, max, _) = n.range.unwrap();
                    let Value::Number(value) = n.value else {
                        continue;
                    };
                    let t = if max > min {
                        ((value - min) / (max - min)).clamp(0., 1.)
                    } else {
                        0.
                    };
                    let r = Rect::new(
                        b.x + 12.,
                        b.y + b.height / 2. - 2.,
                        (b.width - 24.).max(0.),
                        4.,
                    );
                    round(canvas, r, 2., theme.border, false);
                    round(
                        canvas,
                        Rect::new(r.x, r.y, r.width * t, 4.),
                        2.,
                        theme.accent,
                        false,
                    );
                    round(
                        canvas,
                        Rect::new(r.x + r.width * t - 7., b.y + b.height / 2. - 7., 14., 14.),
                        7.,
                        if n.enabled {
                            theme.accent
                        } else {
                            theme.secondary
                        },
                        false,
                    );
                }
                Kind::Progress => {
                    round(canvas, b, 4., theme.border, false);
                    let (start, width) = if let Value::Number(v) = n.value {
                        (0., v.clamp(0., 1.))
                    } else {
                        (((elapsed * 0.6) % 1.) * 0.7, 0.3)
                    };
                    round(
                        canvas,
                        Rect::new(b.x + b.width * start, b.y, b.width * width, b.height),
                        4.,
                        theme.accent,
                        false,
                    );
                }
                Kind::Separator => round(canvas, b, 0., theme.border, false),
                Kind::Image => {
                    if let Some((bitmap, mode)) = &n.image {
                        let image = self.image(bitmap)?;
                        let size = Size::new(image.width() as f64, image.height() as f64);
                        let scale = match mode {
                            ContentMode::Fit => (b.width / size.width).min(b.height / size.height),
                            ContentMode::Fill => (b.width / size.width).max(b.height / size.height),
                            ContentMode::Center => 1.,
                            ContentMode::Stretch => 0.,
                        };
                        let r = if *mode == ContentMode::Stretch {
                            b
                        } else {
                            Rect::new(
                                b.x + (b.width - size.width * scale) / 2.,
                                b.y + (b.height - size.height * scale) / 2.,
                                size.width * scale,
                                size.height * scale,
                            )
                        };
                        canvas.draw_image_rect(image, None, skrect(r), &paint(0xffffffff));
                    }
                }
                _ => (),
            }
            let masked = if n.secure {
                if let Value::Text(v) = &n.value {
                    "•".repeat(text::grapheme_boundaries(v).len().saturating_sub(1))
                } else {
                    String::new()
                }
            } else {
                String::new()
            };
            let text = if n.secure && !masked.is_empty() {
                masked.as_str()
            } else if n.kind == Kind::TextField {
                if let Value::Text(value) = &n.value {
                    if value.is_empty() {
                        n.text.as_str()
                    } else {
                        value.as_str()
                    }
                } else {
                    n.text.as_str()
                }
            } else {
                n.text.as_str()
            };
            if !text.is_empty() && !matches!(n.kind, Kind::Image | Kind::Slider | Kind::Progress) {
                let mut style = n.text_style;
                style.color = fg;
                if n.kind == Kind::TextField {
                    canvas.save();
                    canvas.clip_rect(skrect(b.inset(6.)), None, true);
                    if let Some(input) = &input
                        && input.id == n.id
                    {
                        for r in host.text_rectangles(n.id, input.selection.range()) {
                            round(canvas, r, 0., theme.selected, false);
                        }
                    }
                }
                self.shaped(
                    text,
                    style,
                    if style.wrap {
                        if n.kind == Kind::TextField {
                            b.width - 20.
                        } else {
                            b.width - 2. * n.style.padding
                        }
                    } else {
                        f64::INFINITY
                    },
                )
                .paragraph
                .borrow()
                .paint(canvas, (n.text_bounds.x as f32, n.text_bounds.y as f32));
                if n.kind == Kind::TextField {
                    canvas.restore();
                }
            }
            if let Some(input) = &input
                && input.id == n.id
                && input.active
            {
                canvas.save();
                canvas.clip_rect(skrect(b.inset(6.)), None, true);
                if input.selection.range().is_empty() {
                    for r in
                        host.text_rectangles(n.id, input.selection.caret..input.selection.caret)
                    {
                        round(canvas, r, 0., theme.accent, false);
                    }
                }
                if let Some(marked) = &input.marked {
                    for r in host.text_rectangles(n.id, marked.clone()) {
                        round(
                            canvas,
                            Rect::new(r.x, r.y + r.height - 1., r.width, 1.),
                            0.,
                            theme.accent,
                            false,
                        );
                    }
                }
                canvas.restore();
            }
            if n.focused {
                round(canvas, b.inset(1.), n.style.radius, theme.accent, true);
            }
            canvas.restore();
        }
        Ok(())
    }
}
impl Measure for Renderer {
    fn text(&self, text: &str, style: TextStyle, width: f64) -> TextLayout {
        self.shaped(text, style, width).layout
    }
    fn image(&self, image: &Bitmap) -> Result<Size, Error> {
        let image = self.image(image)?;
        Ok(Size::new(image.width() as f64, image.height() as f64))
    }
}
fn rect(r: skia::Rect) -> Rect {
    Rect::new(
        r.left() as f64,
        r.top() as f64,
        r.width() as f64,
        r.height() as f64,
    )
}
fn skrect(r: Rect) -> skia::Rect {
    skia::Rect::from_xywh(r.x as f32, r.y as f32, r.width as f32, r.height as f32)
}
fn paint(color: u32) -> skia::Paint {
    let mut p = skia::Paint::default();
    p.set_anti_alias(true).set_color(skia::Color::new(color));
    p
}
fn round(canvas: &skia::Canvas, r: Rect, radius: f64, color: u32, stroke: bool) {
    let mut p = paint(color);
    if stroke {
        p.set_style(skia::PaintStyle::Stroke).set_stroke_width(1.);
    }
    canvas.draw_round_rect(skrect(r), radius as f32, radius as f32, &p);
}
