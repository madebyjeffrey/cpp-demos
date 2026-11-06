//! Captured button repeats use one-shot deadlines, never an idle animation loop.
use crate::{Error, Host, NodeId, View};
use std::{
    rc::Rc,
    time::{Duration, Instant},
};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Repeat {
    pub delay: Duration,
    pub interval: Duration,
}
impl Default for Repeat {
    fn default() -> Self {
        Self {
            delay: Duration::from_millis(400),
            interval: Duration::from_millis(80),
        }
    }
}
impl View {
    pub fn repeat(mut self, repeat: Repeat) -> Self {
        Rc::make_mut(&mut self.0).repeat = Some(repeat);
        self
    }
    pub fn on_blur(mut self, callback: impl Fn() + 'static) -> Self {
        Rc::make_mut(&mut self.0).blur = Some(Rc::new(callback));
        self
    }
}
impl Host {
    pub(crate) fn blur_action(&self, id: Option<NodeId>) {
        let callback = id.and_then(|id| {
            let owner = self.value_owner(id);
            if let Some(owner) = owner
                && self.0.interaction.borrow().active
                && self
                    .focused()
                    .is_some_and(|next| Self::within(&self.0.mounted.borrow().nodes, next, owner))
            {
                return None;
            }
            self.0
                .mounted
                .borrow()
                .descriptions
                .get(&owner.unwrap_or(id))
                .and_then(|d| d.blur.clone())
        });
        if let Some(callback) = callback {
            callback();
        }
    }
    pub(crate) fn repeating(&self, id: NodeId) -> Option<Repeat> {
        self.0
            .mounted
            .borrow()
            .descriptions
            .get(&id)
            .and_then(|d| d.repeat)
    }
    fn repeat_owner(&self) -> Option<(NodeId, Repeat)> {
        let i = self.0.interaction.borrow();
        if !i.active {
            return None;
        }
        let id = i.pressed?;
        let m = self.0.mounted.borrow();
        m.nodes.iter().find(|n| {
            n.id == id
                && n.enabled
                && !n.hidden
                && !n.layout_hidden
                && n.clip.contains(i.pointer)
                && n.bounds.contains(i.pointer)
                && self.in_scope(id)
        })?;
        Some((id, m.descriptions.get(&id)?.repeat?))
    }
    pub(crate) fn start_repeat(&self, id: NodeId) -> Result<(), Error> {
        if let Some(repeat) = self.repeating(id) {
            if !self.activate(id)? {
                return Ok(());
            }
            self.0.interaction.borrow_mut().repeat = Some((id, Instant::now() + repeat.delay));
        }
        Ok(())
    }
    pub(crate) fn update_repeat(&self) {
        let owner = self.repeat_owner();
        let mut i = self.0.interaction.borrow_mut();
        if let Some((id, repeat)) = owner {
            if i.repeat.is_none() {
                i.repeat = Some((id, Instant::now() + repeat.delay));
            }
        } else {
            i.repeat = None;
        }
    }
    pub(crate) fn repeat_delay(&self) -> Option<Duration> {
        self.0
            .interaction
            .borrow()
            .repeat
            .map(|(_, at)| at.saturating_duration_since(Instant::now()))
    }
    pub(crate) fn run_repeat_due(&self, now: Instant) {
        if self.refresh().is_err() {
            self.0.interaction.borrow_mut().repeat = None;
            return;
        }
        self.update_repeat();
        if !self
            .0
            .interaction
            .borrow()
            .repeat
            .is_some_and(|(_, at)| at <= now)
        {
            return;
        }
        let Some((id, repeat)) = self.repeat_owner() else {
            return;
        };
        self.0.interaction.borrow_mut().repeat = None;
        let _ = self.activate(id);
        let _ = self.refresh();
        if self
            .repeat_owner()
            .is_some_and(|(current, _)| current == id)
        {
            self.0.interaction.borrow_mut().repeat = Some((id, now + repeat.interval));
        }
    }
}
