//! Callbacks for the native backend and its Metal-backed frame.
use crate::EventHandler;
use std::rc::Rc;

pub type DrawHandler = Rc<dyn Fn(&super::Frame<'_>)>;

#[derive(Clone, Default)]
pub struct Callbacks {
    /// Draw through `frame.canvas()` in logical points. Skia submission and
    /// presentation are managed by the crate after the callback returns.
    pub on_draw: Option<DrawHandler>,
    pub on_event: Option<EventHandler>,
    pub on_close_request: Option<Rc<dyn Fn() -> bool>>,
    pub on_closed: Option<Rc<dyn Fn()>>,
}
