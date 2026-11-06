//! Single-use alert transactions, shared by component overlays and native sheets.
use crate::{DismissReason, Error, Host, Key, KeyEvent, NodeId, State, View};
use std::{
    cell::{Cell, RefCell},
    rc::{Rc, Weak},
    sync::atomic::{AtomicU64, Ordering},
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AlertActionRole {
    #[default]
    Normal,
    Default,
    Cancel,
    Destructive,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AlertAction {
    pub id: String,
    pub label: String,
    pub role: AlertActionRole,
    pub enabled: bool,
}
impl AlertAction {
    pub fn new(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            role: Default::default(),
            enabled: true,
        }
    }
    pub fn role(mut self, role: AlertActionRole) -> Self {
        self.role = role;
        self
    }
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AlertReason {
    Action,
    Cancel,
    Escape,
    Dismissed,
    Closed,
    OwnerRemoved,
    Unmounted,
    Destroyed,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AlertResult {
    pub action: Option<String>,
    pub reason: AlertReason,
}
type Completion = Rc<dyn Fn(AlertResult)>;
struct Inner {
    id: u64,
    title: String,
    actions: Vec<AlertAction>,
    result: State<Option<AlertResult>>,
    completed: Cell<bool>,
    completion: Completion,
    observers: RefCell<Vec<(u64, Completion)>>,
    next_observer: Cell<u64>,
}
impl Drop for Inner {
    fn drop(&mut self) {
        if !self.completed.replace(true) {
            let result = AlertResult {
                action: None,
                reason: AlertReason::Destroyed,
            };
            for (_, observer) in self.observers.get_mut().drain(..) {
                observer(result.clone());
            }
            (self.completion)(result);
        }
    }
}
/// Retain this handle across rebuilds. Create a new transaction to present again.
#[derive(Clone)]
pub struct Alert(Rc<Inner>);
impl PartialEq for Alert {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}
impl Eq for Alert {}
/// Keeps a completion observer registered without extending the alert's lifetime.
pub struct AlertSubscription {
    owner: Weak<Inner>,
    id: u64,
}
impl Drop for AlertSubscription {
    fn drop(&mut self) {
        if let Some(owner) = self.owner.upgrade() {
            let old = {
                let mut observers = owner.observers.borrow_mut();
                observers
                    .iter()
                    .position(|(id, _)| *id == self.id)
                    .map(|i| observers.remove(i))
            };
            drop(old);
        }
    }
}
impl Alert {
    pub fn new(
        title: impl Into<String>,
        actions: Vec<AlertAction>,
        completion: impl Fn(AlertResult) + 'static,
    ) -> Result<Self, Error> {
        let mut ids = std::collections::HashSet::new();
        for action in &actions {
            if action.id.trim().is_empty()
                || action.label.trim().is_empty()
                || !ids.insert(&action.id)
            {
                return Err(Error(
                    "Alert actions need unique nonempty IDs and nonempty labels".into(),
                ));
            }
        }
        for role in [AlertActionRole::Default, AlertActionRole::Cancel] {
            if actions.iter().filter(|a| a.role == role).count() > 1 {
                return Err(Error(
                    "An alert may have at most one default and one cancel action".into(),
                ));
            }
        }
        static NEXT: AtomicU64 = AtomicU64::new(1);
        Ok(Self(Rc::new(Inner {
            id: NEXT.fetch_add(1, Ordering::Relaxed),
            title: title.into(),
            actions,
            result: State::new(None),
            completed: Cell::new(false),
            completion: Rc::new(completion),
            observers: RefCell::new(vec![]),
            next_observer: Cell::new(0),
        })))
    }
    pub fn title(&self) -> &str {
        &self.0.title
    }
    pub fn actions(&self) -> &[AlertAction] {
        &self.0.actions
    }
    pub fn result(&self) -> Option<AlertResult> {
        self.0.result.get()
    }
    pub fn completed(&self) -> bool {
        self.result().is_some()
    }
    fn resolve(&self, result: AlertResult) -> bool {
        crate::state::check_write();
        if self.0.completed.replace(true) {
            return false;
        }
        // Publish before any callback. Reentrant close/choose is inert.
        let observers = std::mem::take(&mut *self.0.observers.borrow_mut());
        self.0.result.set(Some(result.clone()));
        // Backend observers release modal ownership before application completion.
        for (_, observer) in observers {
            observer(result.clone());
        }
        (self.0.completion)(result);
        true
    }
    pub fn choose(&self, id: &str) -> bool {
        if !self.0.actions.iter().any(|a| a.id == id && a.enabled) {
            return false;
        }
        self.resolve(AlertResult {
            action: Some(id.into()),
            reason: AlertReason::Action,
        })
    }
    fn cancel_for(&self, reason: AlertReason) -> bool {
        let action = self
            .0
            .actions
            .iter()
            .find(|a| a.role == AlertActionRole::Cancel && a.enabled)
            .map(|a| a.id.clone());
        self.resolve(AlertResult { action, reason })
    }
    /// Select the enabled cancel action, or complete without an action.
    pub fn cancel(&self) -> bool {
        self.cancel_for(AlertReason::Cancel)
    }
    pub fn dismiss(&self) -> bool {
        self.dismiss_with(AlertReason::Dismissed)
    }
    /// Lifecycle adapters use this to distinguish close, removal and unmount.
    pub fn dismiss_with(&self, reason: AlertReason) -> bool {
        assert!(
            !matches!(
                reason,
                AlertReason::Action | AlertReason::Cancel | AlertReason::Escape
            ),
            "Use choose or cancel for action results"
        );
        self.resolve(AlertResult {
            action: None,
            reason,
        })
    }
    /// Mainly for presentation adapters. Already-completed alerts notify immediately.
    pub fn observe_completion(
        &self,
        callback: impl Fn(AlertResult) + 'static,
    ) -> AlertSubscription {
        crate::state::check_write();
        let id = self.0.next_observer.get().wrapping_add(1);
        self.0.next_observer.set(id);
        let subscription = AlertSubscription {
            owner: Rc::downgrade(&self.0),
            id,
        };
        if let Some(result) = self.result() {
            callback(result);
        } else {
            self.0.observers.borrow_mut().push((id, Rc::new(callback)));
        }
        subscription
    }
    fn dismissed(&self, reason: DismissReason) {
        match reason {
            DismissReason::Escape => {
                self.cancel_for(AlertReason::Escape);
            }
            DismissReason::OwnerRemoved => {
                self.dismiss_with(AlertReason::OwnerRemoved);
            }
            DismissReason::Unmounted => {
                self.dismiss_with(AlertReason::Unmounted);
            }
            _ => {
                self.dismiss();
            }
        }
    }
    /// Modal component overlay; the surrounding window stays open.
    pub fn view(&self, body: View) -> View {
        self.build(body, false)
    }
    /// Full-size sheet content. Normally installed by `Surface::show_sheet`.
    pub fn sheet_view(&self, body: View) -> View {
        self.build(body, true)
    }
    fn build(&self, body: View, sheet: bool) -> View {
        if self.completed() {
            return View::fragment(vec![]);
        }
        let buttons = self
            .0
            .actions
            .iter()
            .map(|action| {
                let model = self.clone();
                let id = action.id.clone();
                let mut button = View::button(&action.label, move || {
                    model.choose(&id);
                })
                .key(format!("$action-{}", action.id))
                .enabled(action.enabled)
                .autofocus(action.role == AlertActionRole::Default && action.enabled);
                let d = Rc::make_mut(&mut button.0);
                d.alert_action = Some((self.clone(), action.id.clone()));
                d.alert_role = Some(action.role);
                button
            })
            .collect();
        let panel = View::panel(vec![
            View::label(self.title())
                .font_size(22.)
                .bold(true)
                .wrap(true),
            body,
            View::row(buttons)
                .wrap(true)
                .justify(crate::Justify::End)
                .key("$actions"),
        ])
        .padding(24.)
        .gap(16.);
        let panel = if sheet {
            panel.width(crate::percent(100.)).grow(1.)
        } else {
            panel.width(crate::percent(100.)).max_width(440.)
        };
        let close = self.clone();
        let presented = self.clone();
        let mut root = View::modal(vec![panel], || {})
            .key(format!("$alert-{}", self.0.id))
            .accessible_label(self.title());
        if sheet {
            root = root
                .background(0x00000000)
                .align(crate::Align::Stretch)
                .justify(crate::Justify::Start);
        }
        let d = Rc::make_mut(&mut root.0);
        d.alert = Some(self.clone());
        d.overlay = Some(crate::overlay::OverlayData {
            options: crate::OverlayOptions {
                dismiss_on_outside: false,
                dismiss_on_deactivate: false,
                ..Default::default()
            },
            point: None,
            close: Rc::new(move |reason| close.dismissed(reason)),
            presented: Some(Rc::new(move || !presented.completed())),
        });
        root
    }
}
impl Host {
    pub(crate) fn alert_action(
        &self,
        id: NodeId,
        alert: &Alert,
        action: &str,
    ) -> Result<bool, Error> {
        if alert.completed() {
            return Ok(false);
        }
        let cancel = alert
            .actions()
            .iter()
            .any(|a| a.id == action && a.role == AlertActionRole::Cancel);
        self.unmark_text(!cancel)?;
        self.refresh()?;
        if !self.snapshot()?.iter().any(|n| {
            n.id == id && n.enabled && !n.hidden && !n.layout_hidden && self.in_scope(n.id)
        }) {
            return Ok(false);
        }
        Ok(alert.choose(action))
    }
    pub(crate) fn alert_key(&self, event: KeyEvent) -> Result<Option<bool>, Error> {
        if !matches!(event.key, Key::Enter | Key::Escape)
            || event.modifiers.command
            || event.modifiers.control
            || event.modifiers.alt
        {
            return Ok(None);
        }
        let alert = self.active_modal().and_then(|id| {
            self.0
                .mounted
                .borrow()
                .descriptions
                .get(&id)
                .and_then(|d| d.alert.clone())
        });
        let Some(alert) = alert else {
            return Ok(None);
        };
        if event.repeat {
            return Ok(Some(true));
        }
        if event.key == Key::Escape {
            return Ok(Some(alert.cancel_for(AlertReason::Escape)));
        }
        if let Some(input) = self.text_input()? {
            if input.marked.is_some() {
                return self.unmark_text(true).map(Some);
            }
            if input.multiline {
                return Ok(None);
            }
        }
        let action = alert
            .actions()
            .iter()
            .find(|a| a.role == AlertActionRole::Default && a.enabled);
        if let Some(action) = action {
            let id = self
                .snapshot()?
                .iter()
                .find(|n| {
                    n.key.as_deref() == Some(&format!("$action-{}", action.id))
                        && self.in_scope(n.id)
                })
                .map(|n| n.id);
            if let Some(id) = id {
                return self.alert_action(id, &alert, &action.id).map(Some);
            }
        }
        // Return must never fall through to an arbitrary destructive/cancel button.
        Ok(Some(true))
    }
}
