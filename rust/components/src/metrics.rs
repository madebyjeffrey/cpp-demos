use crate::{Error, Point, Rect, Size};
use std::{ops::Range, rc::Rc};
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextStyle {
    pub size: f64,
    pub color: u32,
    pub bold: bool,
    pub wrap: bool,
}
/// Shaped geometry in logical points and UTF-8 bytes, supplied by a renderer.
#[derive(Clone)]
pub struct TextLayout {
    pub size: Size,
    pub baseline: f64,
    pub hit_test: Rc<dyn Fn(Point) -> usize>,
    pub range_rects: Rc<dyn Fn(Range<usize>) -> Vec<Rect>>,
}
/// Pure measurement seam; implementations must not mutate state or reenter a host.
pub trait Measure {
    fn text(&self, text: &str, style: TextStyle, width: f64) -> TextLayout;
    fn image(&self, image: &Bitmap) -> Result<Size, Error>;
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ContentMode {
    #[default]
    Fit,
    Fill,
    Stretch,
    Center,
}
#[derive(Clone)]
pub struct Bitmap {
    pub(crate) id: u64,
    pub data: Rc<ImageData>,
}
pub enum ImageData {
    Encoded(Vec<u8>),
    Rgba {
        width: u32,
        height: u32,
        stride: usize,
        pixels: Vec<u8>,
    },
}
impl Bitmap {
    fn new(data: ImageData) -> Self {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
        Self {
            id: NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
            data: Rc::new(data),
        }
    }
    pub fn id(&self) -> u64 {
        self.id
    }
    /// Decoding is checked by the renderer at layout/mount.
    pub fn load(path: impl AsRef<std::path::Path>) -> Result<Self, Error> {
        std::fs::read(path)
            .map(|v| Self::new(ImageData::Encoded(v)))
            .map_err(|e| Error(e.to_string()))
    }
    pub fn from_encoded(bytes: Vec<u8>) -> Self {
        Self::new(ImageData::Encoded(bytes))
    }
    pub fn from_rgba(width: u32, height: u32, pixels: &[u8], stride: usize) -> Result<Self, Error> {
        let stride = if stride == 0 {
            width as usize * 4
        } else {
            stride
        };
        if width == 0
            || height == 0
            || width > 32768
            || height > 32768
            || stride < width as usize * 4
            || stride
                .checked_mul(height as usize)
                .is_none_or(|n| n > pixels.len())
        {
            return Err(Error(
                "Invalid bitmap dimensions, stride or pixel buffer".into(),
            ));
        }
        Ok(Self::new(ImageData::Rgba {
            width,
            height,
            stride,
            pixels: pixels.to_vec(),
        }))
    }
}
