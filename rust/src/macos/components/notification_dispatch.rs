//! Coalesced main-runloop posts with bounded semantic diffs and inert retired targets.
use super::{
    notification_state::{Key, Snapshot, Target},
    *,
};
use objc2::Message;
use std::collections::{HashSet, VecDeque};
thread_local! {
 static POSTS:Cell<u64>=const{Cell::new(0)};
 // No objects, text, model values or callback resources are retained by diagnostics.
 static EVENTS:RefCell<VecDeque<String>>=const{RefCell::new(VecDeque::new())};
}
pub(crate) fn post_count() -> u64 {
    POSTS.with(Cell::get)
}
pub(crate) fn post_events() -> Retained<NSArray<NSString>> {
    EVENTS.with(|q| {
        NSArray::from_retained_slice(&q.borrow().iter().map(|s| ns(s)).collect::<Vec<_>>())
    })
}
pub(crate) fn clear_events() {
    EVENTS.with(|q| q.borrow_mut().clear());
}
fn record(target: &AnyObject, name: &NSString, payload: usize) {
    POSTS.with(|n| n.set(n.get().wrapping_add(1)));
    EVENTS.with(|q| {
        let mut q = q.borrow_mut();
        if q.len() == 64 {
            q.pop_front();
        }
        q.push_back(format!(
            "{}@{:x}:{payload}",
            name, target as *const _ as usize
        ));
    });
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum Change {
    Value,
    Title,
    Rows,
    SelectedRows,
    SelectedCells,
    SelectedChildren,
    Expanded,
    Collapsed,
    Destroyed,
    Moved,
    Resized,
}
impl Change {
    fn name(self) -> &'static NSString {
        unsafe {
            match self {
                Self::Value => NSAccessibilityValueChangedNotification,
                Self::Title => NSAccessibilityTitleChangedNotification,
                Self::Rows => NSAccessibilityRowCountChangedNotification,
                Self::SelectedRows => NSAccessibilitySelectedRowsChangedNotification,
                Self::SelectedCells => NSAccessibilitySelectedCellsChangedNotification,
                Self::SelectedChildren => NSAccessibilitySelectedChildrenChangedNotification,
                Self::Expanded => NSAccessibilityRowExpandedNotification,
                Self::Collapsed => NSAccessibilityRowCollapsedNotification,
                Self::Destroyed => NSAccessibilityUIElementDestroyedNotification,
                Self::Moved => NSAccessibilityMovedNotification,
                Self::Resized => NSAccessibilityResizedNotification,
            }
        }
    }
}
#[derive(Default)]
struct Changes {
    posts: Vec<(Target, Change)>,
    layout: bool,
    affected: Vec<Target>,
}
fn diff(old: &Snapshot, new: &Snapshot) -> Changes {
    let mut out = Changes {
        layout: !old.initialized || old.context != new.context,
        ..Default::default()
    };
    let mut seen = HashSet::new();
    let mut add = |target: &Target, kind: Change| {
        if seen.insert((target.key(), kind)) {
            out.posts.push((target.clone(), kind));
        }
    };
    for (key, item) in &new.items {
        let previous = old.items.get(key);
        let before = previous.and_then(|p| p.attrs.as_ref());
        let after = item.attrs.as_ref();
        if item.retired && previous.is_some_and(|p| !p.retired) {
            add(&item.target, Change::Destroyed);
            out.layout = true;
            continue;
        }
        if let (Some(a), Some(b)) = (before, after) {
            if a.value != b.value {
                add(&item.target, Change::Value)
            }
            if a.title != b.title {
                add(&item.target, Change::Title)
            }
            if a.rows != b.rows && b.rows.is_some() {
                add(&item.target, Change::Rows)
            }
            if a.selection != b.selection {
                for s in [a.selection, b.selection].into_iter().flatten() {
                    add(
                        &item.target,
                        match s.kind {
                            0 => Change::SelectedRows,
                            1 => Change::SelectedCells,
                            _ => Change::SelectedChildren,
                        },
                    );
                }
                add(&item.target, Change::SelectedChildren);
            }
            if a.expanded != b.expanded {
                if matches!(item.target, Target::Row(..)) {
                    add(
                        &item.target,
                        if b.expanded == Some(true) {
                            Change::Expanded
                        } else {
                            Change::Collapsed
                        },
                    );
                } else {
                    add(&item.target, Change::Value);
                }
                out.layout = true;
            }
            if a.bounds != b.bounds {
                if let (Some(a), Some(b)) = (a.bounds, b.bounds) {
                    if a[..2] != b[..2] {
                        add(&item.target, Change::Moved)
                    }
                    if a[2..] != b[2..] {
                        add(&item.target, Change::Resized)
                    }
                }
                out.layout = true;
            }
            if a.meta != b.meta || a.content != b.content {
                out.layout = true;
            }
            if a != b && out.affected.len() < 32 {
                out.affected.push(item.target.clone());
            }
        } else if before.is_some() != after.is_some() {
            out.layout = true;
            if after.is_some() && out.affected.len() < 32 {
                out.affected.push(item.target.clone())
            }
        }
    }
    if old.items.keys().any(|key| !new.items.contains_key(key)) {
        out.layout = true;
    }
    out
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
    pub(super) fn prime_notification(&self, target: Target) {
        if !self.alive.get()
            || self
                .notifications
                .borrow()
                .snapshot
                .items
                .contains_key(&target.key())
        {
            return;
        }
        let capture = self.notifications.borrow().fork();
        let Ok(nodes) = self.host.snapshot() else {
            return;
        };
        let item = capture.capture(target.clone(), self, &nodes);
        if self.alive.get() {
            self.notifications
                .borrow_mut()
                .snapshot
                .items
                .entry(target.key())
                .or_insert(item);
        }
    }
    pub(super) fn forget_notification(&self, key: Key) {
        if let Ok(mut n) = self.notifications.try_borrow_mut() {
            n.snapshot.items.remove(&key);
        }
    }
    fn watched(&self) -> Vec<(Target, Retained<AnyObject>)> {
        let mut watched = vec![];
        for weak in self.row_cache.borrow().values() {
            if let Some(row) = weak.load() {
                let target = row.target();
                watched.push((target, unsafe { Retained::cast_unchecked(row) }));
            }
        }
        for weak in self.table_cache.borrow().values() {
            if let Some(entry) = weak.load() {
                let target = entry.target();
                watched.push((target, unsafe { Retained::cast_unchecked(entry) }));
            }
        }
        watched
    }
    fn notify(&self, surface: &Surface) {
        if !self.alive.get() || !surface.is_open() {
            return;
        }
        let focus = accessibility::focused_element(self);
        let focused = focus
            .as_deref()
            .and_then(notification_state::target)
            .map(|t| t.key());
        let watched = self.watched();
        let capture = self.notifications.borrow().fork();
        let next = match capture.next(self, surface, &watched, focused) {
            Ok(n) => n,
            Err(e) => {
                if let Some(app) = surface.state.app.upgrade() {
                    report(&app, error(&e.to_string()));
                }
                return;
            }
        };
        if !self.alive.get() || !surface.is_open() {
            return;
        }
        let old = std::mem::replace(&mut self.notifications.borrow_mut().snapshot, next);
        let (changes, text, focus_changed) = {
            let n = self.notifications.borrow();
            (
                diff(&old, &n.snapshot),
                n.snapshot.text.clone(),
                old.focus != n.snapshot.focus,
            )
        };
        // Publish the complete baseline before AppKit can reenter our getters.
        let retired = std::mem::take(&mut *self.retired.borrow_mut());
        for element in retired {
            self.post_notification(&element, Change::Destroyed.name());
        }
        for (target, kind) in changes.posts {
            if !self.alive.get() || !surface.is_open() {
                return;
            }
            if let Some(object) = target.object(self) {
                self.post_notification(&object, kind.name());
            }
        }
        if !self.alive.get() || !surface.is_open() {
            return;
        }
        unsafe {
            if let Some(current) = &text {
                // Focus announces the field and its initial value/selection. Only
                // subsequent changes within that same field need a selection post.
                if old.text.as_ref().is_some_and(|old| {
                    old.id == current.id
                        && (old.selection != current.selection || old.marked != current.marked)
                }) {
                    let object = accessibility::element(self, current.id);
                    self.post_notification(&object, NSAccessibilitySelectedTextChangedNotification);
                }
                if self.alive.get()
                    && surface.is_open()
                    && current.marked.is_some()
                    && old
                        .text
                        .as_ref()
                        .is_none_or(|old| old.id != current.id || old.marked.is_none())
                {
                    let object = accessibility::element(self, current.id);
                    self.post_notification(
                        &object,
                        NSAccessibilityTextInputMarkingSessionBeganNotification,
                    );
                }
            }
            if let Some(previous) = old.text.as_ref().filter(|p| p.marked.is_some())
                && text
                    .as_ref()
                    .is_none_or(|t| t.id != previous.id || t.marked.is_none())
                && self.alive.get()
                && surface.is_open()
            {
                // A hidden/blurred field still ends its session; retired fields have
                // already left the cache and receive destruction instead.
                let object = self.cache.borrow().get(&previous.id).cloned();
                if let Some(object) = object {
                    self.post_notification(
                        &object,
                        NSAccessibilityTextInputMarkingSessionEndedNotification,
                    );
                }
            }
        }
        if !self.alive.get() || !surface.is_open() {
            return;
        }
        if changes.layout {
            let mut objects = changes
                .affected
                .iter()
                .filter_map(|t| t.object(self))
                .collect::<Vec<_>>();
            if objects.is_empty() || objects.len() == 32 {
                objects.clear();
                objects.push(unsafe { Retained::cast_unchecked(surface.native_view().retain()) });
            }
            let elements = NSArray::from_retained_slice(&objects);
            // SAFETY: Standard AppKit layout payload; keys and objects live through the post.
            unsafe {
                let info = NSDictionary::<NSString, AnyObject>::from_slices(
                    &[NSAccessibilityUIElementsKey],
                    &[&*elements as &AnyObject],
                );
                record(
                    surface.native_view(),
                    NSAccessibilityLayoutChangedNotification,
                    objects.len(),
                );
                NSAccessibilityPostNotificationWithUserInfo(
                    surface.native_view(),
                    NSAccessibilityLayoutChangedNotification,
                    Some(&info),
                );
            }
        }
        if focus_changed && self.alive.get() && surface.is_open() {
            unsafe {
                self.post_notification(
                    focus.as_deref().unwrap_or(surface.native_view()),
                    NSAccessibilityFocusedUIElementChangedNotification,
                );
            }
        }
    }
    pub(super) fn notify_destroyed(&self) {
        let mut objects = self
            .cache
            .borrow_mut()
            .drain()
            .map(|(_, e)| unsafe { Retained::cast_unchecked(e) })
            .collect::<Vec<Retained<AnyObject>>>();
        objects.extend(self.watched().into_iter().filter_map(|(target, object)| {
            let already_retired = self
                .notifications
                .borrow()
                .snapshot
                .items
                .get(&target.key())
                .is_some_and(|item| item.retired);
            (!already_retired).then_some(object)
        }));
        objects.extend(
            std::mem::take(&mut *self.retired.borrow_mut())
                .into_iter()
                .map(|e| unsafe { Retained::cast_unchecked(e) }),
        );
        self.notifications.borrow_mut().snapshot = Snapshot::default();
        let mut seen = HashSet::new();
        for object in objects {
            if seen.insert(&*object as *const _ as usize) {
                self.post_notification(&object, Change::Destroyed.name());
            }
        }
    }
    pub(super) fn post_notification(&self, target: &AnyObject, name: &NSString) {
        record(target, name, 0);
        unsafe { NSAccessibilityPostNotification(target, name) };
    }
}
