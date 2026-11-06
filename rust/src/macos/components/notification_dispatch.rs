//! Coalesced main-runloop notification posting is independent of drawable availability.
use super::*;
thread_local! {static POSTS:Cell<u64>=const{Cell::new(0)};}
/// Diagnostic counts actual calls to the AppKit posting function, not client receipt.
pub(crate) fn post_count() -> u64 {
    POSTS.with(Cell::get)
}
impl Mounted {
    pub(super) fn queue_notifications(self: &Rc<Self>) {
        if !self.alive.get() || self.notifications_pending.replace(true) {
            return;
        }
        let Some(app) = self.surface.upgrade().and_then(|s| s.state.app.upgrade()) else {
            self.notifications_pending.set(false);
            return;
        };
        let weak = Rc::downgrade(self);
        schedule(app.mtm, Duration::ZERO, move || {
            let Some(m) = weak.upgrade() else { return };
            struct Pending<'a>(&'a Cell<bool>);
            impl Drop for Pending<'_> {
                fn drop(&mut self) {
                    self.0.set(false)
                }
            }
            let _pending = Pending(&m.notifications_pending);
            if !m.alive.get() {
                return;
            }
            let Some(surface) = m
                .surface
                .upgrade()
                .filter(|s| s.is_open() && s.native_window().attachedSheet().is_none())
            else {
                return;
            };
            let Some(app) = surface.state.app.upgrade() else {
                return;
            };
            guarded(&app, || match m.sync(&surface) {
                Ok(()) => m.notify(&surface),
                Err(e) => report(&app, error(&e.to_string())),
            });
        });
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
        // SAFETY: Main-thread native objects and AppKit's exported notification names.
        unsafe {
            if self.generation.replace(generation) != generation {
                self.post_notification(
                    surface.native_view(),
                    NSAccessibilityLayoutChangedNotification,
                );
            }
            if self.focus.replace(focus) != focus {
                self.post_notification(
                    surface.native_view(),
                    NSAccessibilityFocusedUIElementChangedNotification,
                );
            }
            if *self.value.borrow() != value {
                if let Some(id) = focus {
                    let element = accessibility::element(self, id);
                    self.post_notification(&element, NSAccessibilityValueChangedNotification);
                    self.post_notification(
                        &element,
                        NSAccessibilitySelectedTextChangedNotification,
                    );
                }
                *self.value.borrow_mut() = value;
            }
        }
    }
    pub(super) fn post_notification(&self, target: &AnyObject, name: &NSString) {
        POSTS.with(|count| count.set(count.get().wrapping_add(1)));
        // SAFETY: All targets are live main-thread AppKit views/accessibility objects.
        unsafe { NSAccessibilityPostNotification(target, name) };
    }
}
