//! Explicit, bounded retention of active virtual row edits.
use super::{ListModel, list::Model};
use crate::Error;
use std::rc::{Rc, Weak};
pub(super) struct Lease {
    owner: Weak<Model>,
    key: String,
    token: u64,
    id: u64,
}
impl Drop for Lease {
    fn drop(&mut self) {
        let owner = self.owner.clone();
        let key = self.key.clone();
        let id = self.id;
        crate::state::cleanup(move || {
            let Some(owner) = owner.upgrade() else {
                return;
            };
            if owner.pins.with(|pins| pins.get(&key).copied()) != Some(id) {
                return;
            }
            owner.leases.borrow_mut().remove(&key);
            owner.pins.update(|pins| {
                pins.remove(&key);
            });
        });
    }
}
/// Keep this handle while editing a virtual row. Clones share one retention slot.
/// Drop the last clone to release it. A removed/disabled row invalidates the pin.
#[derive(Clone)]
pub struct RowPin(Rc<Lease>);
impl PartialEq for RowPin {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}
impl Eq for RowPin {}
impl RowPin {
    pub fn key(&self) -> &str {
        &self.0.key
    }
    pub fn token(&self) -> u64 {
        self.0.token
    }
    pub fn is_valid(&self) -> bool {
        self.0.owner.upgrade().is_some_and(|owner| {
            owner.pins.with(|pins| pins.get(&self.0.key).copied()) == Some(self.0.id)
        })
    }
}
impl ListModel {
    /// Pin an enabled logical item. Configure ListOptions::max_pinned_rows for
    /// simultaneous edits. Create pins outside view construction; last-handle
    /// cleanup defers safely if it occurs during description replacement.
    pub fn pin(&self, key: &str) -> Result<RowPin, Error> {
        crate::state::check_write();
        let item = self
            .index_of(key)
            .and_then(|i| self.item(i))
            .filter(|i| i.enabled)
            .ok_or_else(|| Error("Cannot pin a missing or disabled row".into()))?;
        let token = self.item_token(&item.key).unwrap();
        if let Some(lease) = self
            .0
            .leases
            .borrow()
            .get(key)
            .and_then(Weak::upgrade)
            .filter(|lease| {
                lease.token == token
                    && self.0.pins.with(|pins| pins.get(key).copied()) == Some(lease.id)
            })
        {
            return Ok(RowPin(lease));
        }
        if self.0.pins.with(|pins| pins.len()) >= self.list_options().max_pinned_rows {
            return Err(Error(
                "Pinned row limit reached; release an edit or increase max_pinned_rows".into(),
            ));
        }
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
        let id = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let lease = Rc::new(Lease {
            id,
            owner: Rc::downgrade(&self.0),
            key: key.into(),
            token,
        });
        self.0
            .leases
            .borrow_mut()
            .insert(key.into(), Rc::downgrade(&lease));
        self.0.pins.update(|pins| {
            pins.insert(key.into(), id);
        });
        Ok(RowPin(lease))
    }
    pub fn pinned_keys(&self) -> Vec<String> {
        self.0.pins.with(|pins| pins.keys().cloned().collect())
    }
    /// Realized viewport/overscan indices plus explicitly pinned rows, in order.
    pub fn realized_indices(&self) -> Vec<usize> {
        let mut indices = self
            .visible_range()
            .collect::<std::collections::BTreeSet<_>>();
        self.0.pins.with(|pins| {
            indices.extend(pins.keys().filter_map(|key| self.index_of(key)));
        });
        indices.into_iter().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Binding, Host, Kind, View};
    #[test]
    fn long_scroll_sessions_bound_host_editor_storage_to_realized_rows() {
        let model = ListModel::new(
            (0..100_000)
                .map(|i| crate::ListItem::new(i.to_string(), "Row"))
                .collect(),
            40.,
        );
        let pin = model.pin("0").unwrap();
        let host = Host::new({
            let model = model.clone();
            move || {
                model.view_with(|item| {
                    let key = item.key.clone();
                    View::text_field("Value", Binding::new(move || key.clone(), |_| {}))
                        .key("editor")
                })
            }
        });
        host.snapshot().unwrap();
        let field = host
            .snapshot()
            .unwrap()
            .iter()
            .find(|n| n.kind == Kind::TextField)
            .unwrap()
            .id;
        for step in 1..=200 {
            model.set_viewport(step as f64 * 400., 240.);
            host.snapshot().unwrap();
            assert!(host.0.editors.borrow().contains_key(&field));
            assert_eq!(
                host.0.editors.borrow().len(),
                model.realized_indices().len()
            );
            assert!(host.0.editors.borrow().len() <= 11);
        }
        drop(pin);
        host.snapshot().unwrap();
        assert!(!host.0.editors.borrow().contains_key(&field));
        assert_eq!(host.0.editors.borrow().len(), model.visible_range().len());
    }
}
