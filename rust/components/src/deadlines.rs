use crate::Host;
use std::{
    rc::Rc,
    time::{Duration, Instant},
};
pub(crate) struct Deadline {
    pub key: String,
    pub at: Instant,
    pub action: Rc<dyn Fn()>,
}
impl Host {
    /// Replace pending work with the same key. Tasks run on the owning thread.
    pub fn debounce(&self, key: impl Into<String>, delay: Duration, action: impl Fn() + 'static) {
        let key = key.into();
        let old = {
            let mut deadlines = self.0.deadlines.borrow_mut();
            let old = deadlines
                .iter()
                .position(|v| v.key == key)
                .map(|i| deadlines.remove(i));
            deadlines.push(Deadline {
                key,
                at: Instant::now() + delay,
                action: Rc::new(action),
            });
            old
        };
        drop(old);
        self.redraw();
    }
    pub fn cancel_deadline(&self, key: &str) {
        self.0.deadlines.borrow_mut().retain(|d| d.key != key);
    }
    pub fn next_deadline(&self) -> Option<Duration> {
        self.0
            .deadlines
            .borrow()
            .iter()
            .map(|d| d.at.saturating_duration_since(Instant::now()))
            .chain(self.tooltip_delay())
            .chain(self.drag_delay())
            .min()
    }
    pub fn run_due(&self) {
        self.run_due_at(Instant::now());
    }
    pub fn run_due_at(&self, now: Instant) {
        self.run_drag_due(now);
        let tasks = {
            let mut deadlines = self.0.deadlines.borrow_mut();
            let mut tasks = Vec::new();
            let mut i = 0;
            while i < deadlines.len() {
                if deadlines[i].at <= now {
                    tasks.push(deadlines.remove(i).action);
                } else {
                    i += 1;
                }
            }
            tasks
        };
        for task in tasks {
            task();
        }
    }
}
