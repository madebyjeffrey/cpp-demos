use super::*;
use crate::ui;
mod accessibility;
mod collection_accessibility;
mod input;
pub(super) use accessibility::{children, focused, hit_test};

pub(super) struct Mounted {
    pub host: ui::Host,
    pub renderer: crate::ui::Renderer,
    pub surface: WeakSurface,
    pub alive: Cell<bool>,
    previous: Callbacks,
    previous_input: Option<Rc<dyn TextInputClient>>,
    cache: RefCell<std::collections::HashMap<ui::NodeId, Retained<accessibility::Element>>>,
    generation: Cell<u64>,
    focus: Cell<Option<ui::NodeId>>,
    value: RefCell<String>,
}
impl Mounted {
    pub fn run<T>(
        &self,
        f: impl FnOnce(&Self, &Surface) -> std::result::Result<T, ui::Error>,
    ) -> Option<T> {
        if !self.alive.get() {
            return None;
        }
        let surface = self.surface.upgrade().filter(|s| s.is_open())?;
        let app = surface.state.app.upgrade()?;
        guarded(&app, || {
            match self.sync(&surface).and_then(|()| f(self, &surface)) {
                Ok(value) => Some(value),
                Err(err) => {
                    report(&app, error(&err.to_string()));
                    None
                }
            }
        })
        .flatten()
    }
    pub fn sync(&self, surface: &Surface) -> std::result::Result<(), ui::Error> {
        self.host
            .set_theme(if surface.effective_appearance() == Appearance::Light {
                ui::Theme::light()
            } else {
                ui::Theme::default()
            });
        let size = surface.content_size();
        self.host
            .layout(ui::Size::new(size.width, size.height), &self.renderer)
    }
    fn notify(&self, surface: &Surface) {
        let generation = self.host.generation();
        let focus = self.host.focused();
        let value = self
            .host
            .text_input()
            .ok()
            .flatten()
            .map(|i| {
                if i.secure {
                    format!(
                        "secure:{}:{:?}",
                        ui::text::grapheme_boundaries(&i.text).len(),
                        i.selection
                    )
                } else {
                    format!("{}:{:?}", i.text, i.selection)
                }
            })
            .unwrap_or_default();
        self.cache.borrow_mut().retain(|id, _| {
            self.host
                .snapshot()
                .is_ok_and(|nodes| nodes.iter().any(|n| n.id == *id && !n.hidden))
        });
        // SAFETY: Main-thread native objects and AppKit's exported notification names.
        unsafe {
            if self.generation.replace(generation) != generation {
                NSAccessibilityPostNotification(
                    surface.native_view(),
                    NSAccessibilityLayoutChangedNotification,
                );
            }
            if self.focus.replace(focus) != focus {
                NSAccessibilityPostNotification(
                    surface.native_view(),
                    NSAccessibilityFocusedUIElementChangedNotification,
                );
            }
            if *self.value.borrow() != value {
                if let Some(id) = focus {
                    let element = accessibility::element(self, id);
                    NSAccessibilityPostNotification(
                        &element,
                        NSAccessibilityValueChangedNotification,
                    );
                    NSAccessibilityPostNotification(
                        &element,
                        NSAccessibilitySelectedTextChangedNotification,
                    );
                }
                *self.value.borrow_mut() = value;
            }
        }
    }
    fn deactivate(&self) {
        self.alive.set(false);
        self.host.dismiss_overlays(ui::DismissReason::Unmounted);
        let _ = self.host.unmark_text(false);
        self.host.blur();
        self.host.set_active(false);
        if let Some(surface) = self.surface.upgrade() {
            surface.set_cursor(Cursor::Arrow);
        }
        self.cache.borrow_mut().clear();
    }
}
impl Surface {
    /// Attach components while preserving existing underlay, unhandled-event and lifecycle callbacks.
    /// A failed initial build/layout leaves the previous content installed.
    pub fn set_view(&self, builder: impl Fn() -> ui::View + 'static) -> Result<ui::Host> {
        if !self.is_open() {
            return Err(error("Window is closed"));
        }
        let host = ui::Host::new(builder);
        let renderer = ui::Renderer::default();
        let size = self.content_size();
        host.layout(ui::Size::new(size.width, size.height), &renderer)
            .map_err(|e| error(&e.to_string()))?;
        self.clear_view();
        let mounted = Rc::new(Mounted {
            host: host.clone(),
            renderer,
            surface: self.downgrade(),
            alive: Cell::new(true),
            previous: self.state.callbacks.borrow().clone(),
            previous_input: self.state.text_input.borrow().clone(),
            cache: RefCell::new(Default::default()),
            generation: Cell::new(0),
            focus: Cell::new(None),
            value: RefCell::new(String::new()),
        });
        let weak = Rc::downgrade(&mounted);
        host.on_invalidate(move || {
            if let Some(m) = weak.upgrade()
                && m.alive.get()
                && let Some(s) = m.surface.upgrade()
            {
                s.request_redraw();
            }
        });
        let draw = mounted.clone();
        let events = mounted.clone();
        let callbacks = Callbacks {
            on_draw: Some(Rc::new(move |frame| {
                if !draw.alive.get() {
                    return;
                }
                let Some(surface) = draw.surface.upgrade().filter(|s| s.is_open()) else {
                    return;
                };
                let result = (|| {
                    draw.host.run_due();
                    draw.sync(&surface)?;
                    if let Some(underlay) = &draw.previous.on_draw {
                        underlay(frame);
                    } else {
                        frame
                            .canvas()
                            .clear(crate::skia::Color::new(draw.host.theme().window));
                    }
                    if !draw.alive.get() || !surface.is_open() {
                        return Ok(());
                    }
                    draw.sync(&surface)?;
                    draw.renderer.paint(
                        &draw.host,
                        frame.canvas(),
                        frame.info.elapsed.as_secs_f64(),
                    )?;
                    let animate = draw.host.needs_animation();
                    if surface.is_animating() != animate {
                        surface.set_animating(animate);
                    }
                    surface
                        .set_redraw_delay(draw.host.next_deadline())
                        .map_err(|e| ui::Error(e.to_string()))?;
                    draw.notify(&surface);
                    Ok::<_, ui::Error>(())
                })();
                if let Err(err) = result {
                    surface.state.failed.set(true);
                    if let Some(app) = surface.state.app.upgrade() {
                        report(&app, error(&err.to_string()));
                    }
                }
            })),
            on_event: Some(Rc::new(move |event| {
                events.run(|m, s| {
                    let handled = input::event(m, event)?;
                    if let Event::Pointer(p) = event {
                        let cursor = m.host.pointer_cursor(ui::Point {
                            x: p.position.x,
                            y: p.position.y,
                        })?;
                        s.set_cursor(match cursor {
                            ui::PointerCursor::Arrow => Cursor::Arrow,
                            ui::PointerCursor::Text => Cursor::Text,
                            ui::PointerCursor::ResizeHorizontal => Cursor::ResizeHorizontal,
                            ui::PointerCursor::ResizeVertical => Cursor::ResizeVertical,
                        });
                    } else if matches!(event, Event::Focus(false)) {
                        s.set_cursor(Cursor::Arrow);
                    }

                    s.invalidate_text_input();
                    if !handled && let Some(unhandled) = &m.previous.on_event {
                        unhandled(event);
                    }
                    Ok(())
                });
            })),
            on_close_request: mounted.previous.on_close_request.clone(),
            on_closed: mounted.previous.on_closed.clone(),
        };
        let previous = self.state.callbacks.replace(callbacks);
        drop(previous);
        *self.state.components.borrow_mut() = Some(mounted.clone());
        self.set_text_input_client(Some(Rc::new(input::Client {
            owner: Rc::downgrade(&mounted),
        })));
        self.state.failed.set(false);
        self.request_redraw();
        Ok(host)
    }
    pub fn view_host(&self) -> Option<ui::Host> {
        self.state
            .components
            .borrow()
            .as_ref()
            .map(|m| m.host.clone())
    }
    pub fn clear_view(&self) {
        let previous = self.state.components.borrow_mut().take();
        if let Some(m) = previous {
            m.deactivate();
            let callbacks = self.state.callbacks.replace(m.previous.clone());
            drop(callbacks);
            self.set_text_input_client(m.previous_input.clone());
            self.set_animating(false);
            let _ = self.set_redraw_delay(None);
            self.request_redraw();
        }
    }
}
pub(super) fn closed(s: &SurfaceState) {
    let previous = s.components.borrow_mut().take();
    if let Some(m) = previous {
        m.deactivate();
    }
}
