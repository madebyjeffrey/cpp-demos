//! Native-independent search/password behavior, scoped to mounted field identity.
use crate::{Error, Host, NodeId, View, view::Data};
use std::{
    rc::Rc,
    time::{Duration, Instant},
};
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TextFieldKind {
    #[default]
    Plain,
    Search,
    Password,
}
#[derive(Clone, Copy)]
pub(crate) enum FieldAction {
    Clear,
    Reveal,
}
#[derive(Clone)]
pub(crate) struct FieldOptions {
    pub kind: TextFieldKind,
    pub clear: bool,
    pub reveal: bool,
    pub delay: Duration,
    pub query: Option<Rc<dyn Fn(String)>>,
}
impl Default for FieldOptions {
    fn default() -> Self {
        Self {
            kind: TextFieldKind::Plain,
            clear: true,
            reveal: false,
            delay: Duration::ZERO,
            query: None,
        }
    }
}
#[derive(Default)]
pub(crate) struct FieldState {
    pub revealed: bool,
    pending: Option<(Instant, String)>,
}
impl View {
    pub fn clear_button(mut self, show: bool) -> Self {
        Rc::make_mut(&mut self.0).field.clear = show;
        self
    }
    pub fn allow_reveal(mut self, allow: bool) -> Self {
        Rc::make_mut(&mut self.0).field.reveal = allow;
        self
    }
    pub fn debounce(mut self, delay: Duration) -> Self {
        Rc::make_mut(&mut self.0).field.delay = delay;
        self
    }
    /// Committed user edits only. External model replacement cancels pending work.
    pub fn on_query(mut self, query: impl Fn(String) + 'static) -> Self {
        Rc::make_mut(&mut self.0).field.query = Some(Rc::new(query));
        self
    }
}
impl Host {
    pub(crate) fn reconcile_fields(&self) {
        let m = self.0.mounted.borrow();
        let active = self.0.interaction.borrow().active;
        self.0.fields.borrow_mut().retain(|id, state| {
            let Some(n) = m
                .nodes
                .iter()
                .find(|n| n.id == *id && n.field_kind != TextFieldKind::Plain)
            else {
                return false;
            };
            let d = &m.descriptions[id];
            if n.read_only {
                state.pending = None;
            }
            if !active || !n.enabled || n.hidden || n.layout_hidden || !self.in_scope(*id) {
                state.pending = None;
                state.revealed = false;
            }
            if !d.field.reveal {
                state.revealed = false;
            }
            if state.pending.as_ref().is_some_and(
                |(_, query)| !matches!(&n.value,crate::Value::Text(value) if value==query),
            ) || d.field.query.is_none()
            {
                state.pending = None;
            }
            true
        });
    }
    pub(crate) fn field_delay(&self) -> Option<Duration> {
        self.0
            .fields
            .borrow()
            .values()
            .filter_map(|s| {
                s.pending
                    .as_ref()
                    .map(|(at, _)| at.saturating_duration_since(Instant::now()))
            })
            .min()
    }
    pub(crate) fn search_changed(&self, id: NodeId, query: String, immediate: bool) {
        let d = self.0.mounted.borrow().descriptions.get(&id).cloned();
        let Some(d) =
            d.filter(|d| d.field.kind == TextFieldKind::Search && d.field.query.is_some())
        else {
            return;
        };
        self.0.fields.borrow_mut().entry(id).or_default().pending = Some((
            Instant::now()
                + if immediate {
                    Duration::ZERO
                } else {
                    d.field.delay
                },
            query,
        ));
        self.redraw();
    }
    pub(crate) fn run_search_due(&self, now: Instant) {
        if self.refresh().is_err() {
            self.cancel_field_work();
            return;
        }
        self.reconcile_fields();
        let due: Vec<_> = {
            let mut states = self.0.fields.borrow_mut();
            states
                .iter_mut()
                .filter_map(|(id, s)| {
                    if s.pending.as_ref().is_some_and(|(at, _)| *at <= now) {
                        Some((*id, s.pending.take().unwrap().1))
                    } else {
                        None
                    }
                })
                .collect()
        };
        for (id, query) in due {
            self.dispatch_search(id, query);
        }
    }
    fn dispatch_search(&self, id: NodeId, query: String) {
        if self.refresh().is_err() {
            return;
        }
        let callback = {
            let m = self.0.mounted.borrow();
            m.nodes
                .iter()
                .find(|n| {
                    n.id == id && n.enabled && !n.hidden && !n.layout_hidden && self.in_scope(id)
                })
                .filter(|n| matches!(&n.value,crate::Value::Text(v) if v==&query))
                .and_then(|_| m.descriptions.get(&id))
                .and_then(|d| d.field.query.clone())
        };
        if self.0.interaction.borrow().active
            && let Some(callback) = callback
        {
            callback(query);
        }
    }
    pub(crate) fn flush_search(&self, id: NodeId) {
        let pending = self
            .0
            .fields
            .borrow_mut()
            .get_mut(&id)
            .and_then(|s| s.pending.take());
        if let Some((_, query)) = pending {
            self.dispatch_search(id, query);
        }
    }
    pub(crate) fn cancel_field_work(&self) {
        for state in self.0.fields.borrow_mut().values_mut() {
            state.pending = None;
            state.revealed = false;
        }
    }
    pub(crate) fn field_owner(&self, id: NodeId) -> Option<NodeId> {
        let m = self.0.mounted.borrow();
        let n = m.nodes.iter().find(|n| n.id == id)?;
        if n.field_kind == TextFieldKind::Password {
            Some(id)
        } else if m.descriptions[&id].field_action.is_some() {
            n.parent
                .filter(|p| m.nodes.iter().any(|n| n.id == *p && n.secure))
        } else {
            None
        }
    }
    pub fn set_password_revealed(&self, id: NodeId, reveal: bool) -> Result<bool, Error> {
        self.refresh()?;
        if reveal && !self.0.interaction.borrow().active {
            return Ok(false);
        }
        if !self.0.mounted.borrow().nodes.iter().any(|n| {
            n.id == id
                && n.enabled
                && !n.hidden
                && !n.layout_hidden
                && n.field_kind == TextFieldKind::Password
                && self.in_scope(id)
                && self.0.mounted.borrow().descriptions[&id].field.reveal
        }) {
            return Ok(false);
        }
        self.0.fields.borrow_mut().entry(id).or_default().revealed = reveal;
        self.redraw();
        Ok(true)
    }
    pub fn clear_search(&self, id: NodeId) -> Result<bool, Error> {
        self.refresh()?;
        let binding = {
            let m = self.0.mounted.borrow();
            m.nodes
                .iter()
                .find(|n| {
                    n.id == id
                        && n.enabled
                        && !n.hidden
                        && !n.layout_hidden
                        && !n.read_only
                        && self.in_scope(id)
                        && n.field_kind == TextFieldKind::Search
                })
                .and_then(|_| match &m.descriptions[&id].data {
                    Data::Text(b) => Some(b.clone()),
                    _ => None,
                })
        };
        let Some(binding) = binding else {
            return Ok(false);
        };
        self.set_text(id, "")?;
        self.search_changed(id, binding.get(), true);
        self.focus(id)?;
        self.redraw();
        Ok(true)
    }
    pub(crate) fn field_action(&self, id: NodeId, action: FieldAction) -> Result<bool, Error> {
        let parent = self
            .0
            .mounted
            .borrow()
            .nodes
            .iter()
            .find(|n| n.id == id)
            .and_then(|n| n.parent);
        let Some(parent) = parent else {
            return Ok(false);
        };
        match action {
            FieldAction::Clear => self.clear_search(parent),
            FieldAction::Reveal => {
                let revealed = self
                    .0
                    .fields
                    .borrow()
                    .get(&parent)
                    .is_some_and(|s| s.revealed);
                self.set_password_revealed(parent, !revealed)?;
                self.focus(parent)?;
                Ok(true)
            }
        }
    }
}
pub(crate) fn children(d: &crate::view::Description, value: &crate::Value) -> Vec<View> {
    let mut children = d.children.clone();
    let (action, label, width) = match d.field.kind {
        TextFieldKind::Search
            if d.field.clear
                && !d.read_only
                && matches!(value,crate::Value::Text(v) if !v.is_empty()) =>
        {
            (FieldAction::Clear, "Clear", 44)
        }
        TextFieldKind::Password if d.field.reveal => (FieldAction::Reveal, "Show", 48),
        _ => return children,
    };
    let mut button = View::button(label, || {})
        .key("$field-action")
        .width(width)
        .height(24)
        .accessible_label(if matches!(action, FieldAction::Clear) {
            "Clear search"
        } else {
            "Reveal password"
        });
    Rc::make_mut(&mut button.0).field_action = Some(action);
    children.push(button);
    children
}
