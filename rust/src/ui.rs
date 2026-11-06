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
    runs: Vec<SpanKey>,
    size: u64,
    color: u32,
    bold: bool,
    wrap: bool,
    width: u64,
    align: u8,
    overflow: u8,
    max_lines: usize,
    line_height: u64,
    letter_spacing: u64,
}
#[derive(Hash, PartialEq, Eq)]
enum SpanKey {
    Text(Box<TextKey>),
    Box(u64, u64, u64),
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
            runs: Vec::new(),
            size: style.size.to_bits(),
            color: style.color,
            bold: style.bold,
            wrap: style.wrap,
            align: style.align as u8,
            overflow: style.overflow as u8,
            max_lines: style.max_lines,
            line_height: style.line_height.to_bits(),
            letter_spacing: style.letter_spacing.to_bits(),
            width: if style.constrained() {
                width.max(1.).to_bits()
            } else {
                1_000_000f64.to_bits()
            },
        }
    }
    fn shaped(&self, text: &str, style: TextStyle, width: f64) -> Shaped {
        self.shaped_runs(
            &[ParagraphSpan::Text {
                text: text.into(),
                style,
            }],
            style,
            width,
        )
    }
    fn sk_text_style(&self, style: TextStyle) -> SkTextStyle {
        let mut ts = SkTextStyle::new();
        ts.set_font_size(style.size as f32)
            .set_color(skia::Color::new(style.color));
        if let Some(family) = &self.family {
            ts.set_font_families(&[family.as_str()]);
        }
        if style.bold {
            ts.set_font_style(skia::FontStyle::bold());
        }
        ts.set_letter_spacing(style.letter_spacing as f32);
        if style.line_height > 0. {
            ts.set_height((style.line_height / style.size) as f32)
                .set_height_override(true);
        }
        ts
    }
    fn shaped_runs(&self, spans: &[ParagraphSpan], style: TextStyle, width: f64) -> Shaped {
        self.shape_runs(spans, style, width, true)
    }
    fn shape_runs(
        &self,
        spans: &[ParagraphSpan],
        style: TextStyle,
        width: f64,
        cache: bool,
    ) -> Shaped {
        let mut key = Self::key("", style, width);
        key.runs = spans
            .iter()
            .map(|span| match span {
                ParagraphSpan::Text { text, style } => {
                    SpanKey::Text(Box::new(Self::key(text, *style, 0.)))
                }
                ParagraphSpan::Box { size, baseline } => SpanKey::Box(
                    size.width.to_bits(),
                    size.height.to_bits(),
                    baseline.to_bits(),
                ),
            })
            .collect();
        if cache && let Some(shaped) = self.text.borrow().get(&key) {
            return Shaped {
                paragraph: shaped.paragraph.clone(),
                layout: shaped.layout.clone(),
            };
        }
        let ts = self.sk_text_style(style);
        let mut ps = ParagraphStyle::new();
        ps.set_text_align(match style.align {
            Align::Center => skia::textlayout::TextAlign::Center,
            Align::End => skia::textlayout::TextAlign::Right,
            _ => skia::textlayout::TextAlign::Left,
        });
        let max_lines = if !style.wrap && style.overflow == TextOverflow::Ellipsis {
            Some(style.max_lines.max(1))
        } else {
            (style.max_lines > 0).then_some(style.max_lines)
        };
        ps.set_max_lines(max_lines);
        if style.overflow == TextOverflow::Ellipsis {
            ps.set_ellipsis("…");
        }
        ps.set_text_style(&ts);
        let mut builder = ParagraphBuilder::new(&ps, self.fonts.clone());
        let mut source = String::new();
        for span in spans {
            match span {
                ParagraphSpan::Text { text, style } => {
                    builder.push_style(&self.sk_text_style(*style));
                    builder.add_text(text);
                    builder.pop();
                    source.push_str(text);
                }
                ParagraphSpan::Box { size, baseline } => {
                    builder.add_placeholder(&skia::textlayout::PlaceholderStyle::new(
                        size.width as f32,
                        size.height as f32,
                        skia::textlayout::PlaceholderAlignment::Baseline,
                        skia::textlayout::TextBaseline::Alphabetic,
                        *baseline as f32,
                    ));
                    source.push('\u{fffc}');
                }
            }
        }
        let mut paragraph = builder.build();
        paragraph.layout(f64::from_bits(key.width).min(1_000_000.) as f32);
        let size = Size::new(
            (paragraph.longest_line() as f64).min(f64::from_bits(key.width)),
            paragraph.height().max(style.size as f32 * 1.2) as f64,
        );
        let baseline = paragraph.alphabetic_baseline() as f64;
        let placeholders = paragraph
            .get_rects_for_placeholders()
            .into_iter()
            .map(|b| rect(b.rect))
            .collect();
        let mut lines = Vec::new();
        let metrics: Vec<_> = paragraph
            .get_line_metrics()
            .into_iter()
            .map(|l| {
                (
                    l.start_index,
                    l.end_index,
                    l.end_including_newline,
                    l.left,
                    l.baseline,
                    l.ascent,
                    l.width,
                    l.height,
                )
            })
            .collect();
        let mut previous_end = 0;
        for (
            start_index,
            end_index,
            end_including_newline,
            left,
            baseline,
            ascent,
            width,
            height,
        ) in metrics
        {
            let byte = |index| text::byte_offset(&source, index).unwrap_or(source.len());
            let start = byte(start_index).max(previous_end);
            let mut end = byte(end_index).max(start);
            if end > start && source[..end].ends_with('\n') {
                end -= 1;
            }
            let bounds = Rect::new(left, baseline - ascent, width, height.max(style.size));
            let mut caret = |index: usize, at_end: bool| {
                let units = text::utf16_offset(&source, index).unwrap_or(0);
                let info = paragraph.get_glyph_info_at_utf16_offset(if at_end {
                    units.saturating_sub(1)
                } else {
                    units
                });
                let x = info.map_or(left, |info| {
                    let rtl = info.text_direction == TextDirection::RTL;
                    if at_end != rtl {
                        info.grapheme_layout_bounds.right() as f64
                    } else {
                        info.grapheme_layout_bounds.left() as f64
                    }
                });
                Rect::new(x, bounds.y, 1., bounds.height)
            };
            let start_caret = caret(start, false);
            let end_caret = if start == end {
                start_caret
            } else {
                caret(end, true)
            };
            previous_end = byte(end_including_newline).max(end);
            lines.push(TextLine {
                range: start..end,
                end_including_newline: previous_end,
                bounds,
                start_caret,
                end_caret,
            });
        }
        let paragraph = Rc::new(RefCell::new(paragraph));
        let hit = paragraph.clone();
        let ranges = paragraph.clone();
        let source2 = source.clone();
        let font = style.size;
        let layout = TextLayout {
            lines,
            size,
            placeholders,
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
        if cache {
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
        }
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
        let mut scrollbars: Vec<(usize, Rect, ScrollInfo, bool)> = Vec::new();
        for n in host.scene()? {
            while scrollbars
                .last()
                .is_some_and(|(depth, ..)| n.kind == Kind::Modal || *depth >= n.depth)
            {
                let (_, clip, info, enabled) = scrollbars.pop().unwrap();
                draw_scrollbars(
                    canvas,
                    clip,
                    info,
                    if enabled {
                        theme.secondary
                    } else {
                        theme.border
                    },
                );
            }
            if let Some(info) = n.scroll.filter(|_| !n.hidden) {
                scrollbars.push((n.depth, n.clip, info, n.enabled));
            }
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
            if n.combo_owner.is_some() && n.kind != Kind::Modal {
                bg = bg.or(Some(theme.panel));
            }
            if n.menu_panel {
                bg = bg.or(Some(theme.panel));
            }
            match n.kind {
                Kind::Button | Kind::CollectionRow if n.table_cell.is_some() => {
                    bg = bg.or_else(|| (n.value.selected() || n.pressed).then_some(theme.selected));
                }
                Kind::Button if n.menu_item.is_some() => {
                    bg = bg.or_else(|| (n.focused || n.pressed).then_some(theme.selected));
                }
                Kind::Panel | Kind::GroupBox => bg = bg.or(Some(theme.panel)),
                Kind::Button | Kind::Tab | Kind::Toggle | Kind::Segment | Kind::CollectionRow => {
                    bg = bg.or(Some(
                        if n.value.selected()
                            || n.pressed
                            || n.alert_role == Some(AlertActionRole::Default)
                        {
                            theme.selected
                        } else if n.hovered {
                            theme.border
                        } else {
                            theme.panel
                        },
                    ))
                }
                Kind::TextField => bg = bg.or(Some(theme.field)),
                _ => (),
            }
            if let Some(bg) = bg {
                round(canvas, b, n.style.radius, bg, false);
            }
            if let Some(color) = n.color_preview {
                color_swatch(canvas, b, n.clip, n.style.radius, color.0);
            }
            match n.kind {
                Kind::Button if n.menu_item.is_some() || n.table_cell.is_some() => (),
                Kind::Button
                | Kind::Tab
                | Kind::Toggle
                | Kind::Segment
                | Kind::TextField
                | Kind::GroupBox => round(
                    canvas,
                    b.inset(0.5),
                    n.style.radius,
                    match n.alert_role {
                        _ if n.table_edit_error.is_some() => 0xffff6868,
                        Some(AlertActionRole::Default) => n.accent,
                        Some(AlertActionRole::Destructive) => 0xffe85555,
                        _ => theme.border,
                    },
                    true,
                ),
                Kind::CheckBox | Kind::Radio => {
                    let r = Rect::new(b.x + 2., b.y + (b.height - 18.) / 2., 18., 18.);
                    let selected = n.value.selected();
                    round(
                        canvas,
                        r,
                        if n.kind == Kind::Radio { 9. } else { 3. },
                        if selected { n.accent } else { theme.border },
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
                        round(canvas, r.inset(5.), 1., n.accent, false);
                    }
                }
                Kind::Switch => {
                    let r = Rect::new(b.x, b.y + (b.height - 22.) / 2., 40., 22.);
                    round(
                        canvas,
                        r,
                        11.,
                        if n.value.selected() {
                            n.accent
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
                        n.accent,
                        false,
                    );
                    round(
                        canvas,
                        Rect::new(r.x + r.width * t - 7., b.y + b.height / 2. - 7., 14., 14.),
                        7.,
                        if n.enabled { n.accent } else { theme.secondary },
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
                        n.accent,
                        false,
                    );
                }
                Kind::Separator => round(canvas, b, 0., theme.border, false),
                Kind::SplitDivider => {
                    let horizontal = n
                        .split
                        .is_none_or(|s| s.orientation == Orientation::Horizontal);
                    if n.hovered || n.pressed {
                        round(canvas, b, 3., theme.panel, false);
                    }
                    let grip = if horizontal {
                        Rect::new(
                            b.x + b.width / 2. - 1.,
                            b.y + (b.height - 28.).max(0.) / 2.,
                            2.,
                            b.height.min(28.),
                        )
                    } else {
                        Rect::new(
                            b.x + (b.width - 28.).max(0.) / 2.,
                            b.y + b.height / 2. - 1.,
                            b.width.min(28.),
                            2.,
                        )
                    };
                    round(
                        canvas,
                        grip,
                        1.,
                        if n.pressed { n.accent } else { theme.border },
                        false,
                    );
                }

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
            if let Some(item) = &n.menu_item {
                let mut style = n.text_style;
                style.color = fg;
                let trailing = if item.submenu { "›" } else { &item.shortcut };
                for (text, right) in [
                    (
                        if item.checked == Some(true) {
                            "✓"
                        } else {
                            ""
                        },
                        false,
                    ),
                    (trailing, true),
                ] {
                    if text.is_empty() {
                        continue;
                    }
                    let shaped = self.shaped_runs(
                        &[ParagraphSpan::Text {
                            text: text.into(),
                            style,
                        }],
                        style,
                        f64::INFINITY,
                    );
                    let x = if right {
                        b.x + b.width - shaped.layout.size.width - 10.
                    } else {
                        b.x + 8.
                    };
                    shaped
                        .paragraph
                        .borrow()
                        .paint(canvas, (x as f32, n.text_bounds.y as f32));
                }
            }
            if let Some(spans) = &n.spans {
                let mut spans = spans.clone();
                if !n.enabled {
                    for span in &mut spans {
                        if let ParagraphSpan::Text { style, .. } = span {
                            style.color = theme.secondary;
                        }
                    }
                }
                self.shaped_runs(&spans, n.text_style, n.text_bounds.width)
                    .paragraph
                    .borrow()
                    .paint(canvas, (n.text_bounds.x as f32, n.text_bounds.y as f32));
            }
            let masked = if n.secure && !n.revealed {
                if let Value::Text(v) = &n.value {
                    "•".repeat(text::grapheme_boundaries(v).len().saturating_sub(1))
                } else {
                    String::new()
                }
            } else {
                String::new()
            };
            let text = if n.secure && !n.revealed && !masked.is_empty() {
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
            if !text.is_empty()
                && !matches!(
                    n.kind,
                    Kind::Image | Kind::Slider | Kind::Progress | Kind::RichText
                )
            {
                let mut style = n.text_style;
                style.color = fg;
                if n.kind == Kind::TextField {
                    canvas.save();
                    canvas.clip_rect(skrect(n.edit_bounds()), None, true);
                    if let Some(input) = &input
                        && input.id == n.id
                    {
                        for r in host.text_rectangles(n.id, input.selection.range()) {
                            round(canvas, r, 0., theme.selected, false);
                        }
                    }
                }
                self.shape_runs(
                    &[ParagraphSpan::Text {
                        text: text.into(),
                        style,
                    }],
                    style,
                    if style.constrained() {
                        if n.kind == Kind::TextField {
                            n.edit_bounds().width
                        } else {
                            n.text_bounds.width
                        }
                    } else {
                        f64::INFINITY
                    },
                    !n.secure || !n.revealed,
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
                canvas.clip_rect(skrect(n.edit_bounds()), None, true);
                if input.selection.range().is_empty() {
                    for r in
                        host.text_rectangles(n.id, input.selection.caret..input.selection.caret)
                    {
                        round(canvas, r, 0., n.accent, false);
                    }
                }
                if let Some(marked) = &input.marked {
                    for r in host.text_rectangles(n.id, marked.clone()) {
                        round(
                            canvas,
                            Rect::new(r.x, r.y + r.height - 1., r.width, 1.),
                            0.,
                            n.accent,
                            false,
                        );
                    }
                }
                canvas.restore();
            }
            if n.focused {
                round(canvas, b.inset(1.), n.style.radius, n.accent, true);
            }
            canvas.restore();
        }
        if let Some(info) = host
            .inspection()
            .filter(|info| !info.hidden && info.clip.width > 0. && info.clip.height > 0.)
        {
            canvas.save();
            canvas.clip_rect(skrect(info.clip), None, true);
            round(canvas, info.bounds.inset(1.), 2., theme.accent, true);
            canvas.restore();
        }
        for (_, clip, info, enabled) in scrollbars.into_iter().rev() {
            draw_scrollbars(
                canvas,
                clip,
                info,
                if enabled {
                    theme.secondary
                } else {
                    theme.border
                },
            );
        }
        if let Some(tip) = host.tooltip_presentation(self)? {
            canvas.save();
            canvas.clip_rect(skrect(tip.bounds), None, true);
            round(canvas, tip.bounds, 5., theme.panel, false);
            self.shaped(&tip.text, tip.style, tip.text_bounds.width)
                .paragraph
                .borrow()
                .paint(canvas, (tip.text_bounds.x as f32, tip.text_bounds.y as f32));
            canvas.restore();
        }
        Ok(())
    }
}
impl Measure for Renderer {
    fn sensitive_text(
        &self,
        text: &str,
        style: TextStyle,
        width: f64,
    ) -> Result<TextLayout, Error> {
        Ok(self
            .shape_runs(
                &[ParagraphSpan::Text {
                    text: text.into(),
                    style,
                }],
                style,
                width,
                false,
            )
            .layout)
    }

    fn text(&self, text: &str, style: TextStyle, width: f64) -> TextLayout {
        self.shaped(text, style, width).layout
    }
    fn paragraph(
        &self,
        spans: &[ParagraphSpan],
        style: TextStyle,
        width: f64,
    ) -> Result<TextLayout, Error> {
        for span in spans {
            match span {
                ParagraphSpan::Box { size, baseline }
                    if !size.width.is_finite()
                        || !size.height.is_finite()
                        || !baseline.is_finite()
                        || size.width < 0.
                        || size.height < 0. =>
                {
                    return Err(Error("Invalid inline box geometry".into()));
                }
                ParagraphSpan::Text { style, .. }
                    if !style.size.is_finite()
                        || style.size <= 0.
                        || !style.line_height.is_finite()
                        || !style.letter_spacing.is_finite() =>
                {
                    return Err(Error("Invalid paragraph span style".into()));
                }
                _ => {}
            }
        }
        Ok(self.shaped_runs(spans, style, width).layout)
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
fn color_swatch(canvas: &skia::Canvas, b: Rect, clip: Rect, radius: f64, color: u32) {
    canvas.save();
    canvas.clip_rrect(
        skia::RRect::new_rect_xy(skrect(b), radius as f32, radius as f32),
        None,
        true,
    );
    canvas.draw_rect(skrect(b), &paint(0xffffffff));
    let visible = b.intersection(clip);
    for y in ((visible.y - b.y) / 8.).floor().max(0.) as usize
        ..((visible.y + visible.height - b.y) / 8.).ceil().max(0.) as usize
    {
        for x in ((visible.x - b.x) / 8.).floor().max(0.) as usize
            ..((visible.x + visible.width - b.x) / 8.).ceil().max(0.) as usize
        {
            if x % 2 == y % 2 {
                canvas.draw_rect(
                    skrect(Rect::new(b.x + x as f64 * 8., b.y + y as f64 * 8., 8., 8.)),
                    &paint(0xffbfbfbf),
                );
            }
        }
    }
    canvas.draw_rect(skrect(b), &paint(color));
    canvas.restore();
}

fn draw_scrollbars(canvas: &skia::Canvas, clip: Rect, info: ScrollInfo, color: u32) {
    canvas.save();
    canvas.clip_rect(skrect(clip), None, true);
    for bar in info.bars() {
        round(
            canvas,
            bar.track,
            4.,
            (color & 0x00ffffff) | 0x20000000,
            false,
        );
        round(canvas, bar.thumb, 4., color, false);
    }
    canvas.restore();
}
#[cfg(test)]
mod field_tests {
    use super::*;
    #[test]
    fn revealed_password_is_painted_without_entering_paragraph_cache() {
        let secret = "unique-password-49385";
        let value = State::new(secret.to_owned());
        let host = Host::new(
            move || view! {password_field("Secret",bind:value,allow_reveal:true,undo_enabled:true);},
        );
        let renderer = Renderer::default();
        host.layout(Size::new(300., 50.), &renderer).unwrap();
        let id = host
            .snapshot()
            .unwrap()
            .iter()
            .find(|n| n.kind == Kind::TextField)
            .unwrap()
            .id;
        host.focus(id).unwrap();
        host.set_password_revealed(id, true).unwrap();
        host.layout(Size::new(300., 50.), &renderer).unwrap();
        let mut surface = skia::surfaces::raster_n32_premul((300, 50)).unwrap();
        renderer.paint(&host, surface.canvas(), 0.).unwrap();
        assert!(
            host.snapshot()
                .unwrap()
                .iter()
                .find(|n| n.id == id)
                .unwrap()
                .revealed
        );
        for key in renderer.text.borrow().keys() {
            assert!(!key.text.contains(secret));
            for span in &key.runs {
                if let SpanKey::Text(text) = span {
                    assert!(!text.text.contains(secret));
                }
            }
        }
        let action = host
            .snapshot()
            .unwrap()
            .into_iter()
            .find(|n| n.label == "Hide password")
            .unwrap();
        assert!(action.bounds.width > 0.);
        host.activate(action.id).unwrap();
        host.layout(Size::new(300., 50.), &renderer).unwrap();
        assert!(
            !host
                .snapshot()
                .unwrap()
                .iter()
                .find(|n| n.id == id)
                .unwrap()
                .revealed
        );
    }
}
