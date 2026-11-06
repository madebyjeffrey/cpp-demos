//! Mounted list input; logical selection/navigation lives in ListModel.
use crate::{Error, Host, Key, KeyEvent, Kind, ListModel, ListSelection, Modifiers, Node, NodeId};
use std::time::Instant;
pub(crate) fn selection(modifiers: Modifiers) -> ListSelection {
    match (modifiers.shift, modifiers.command || modifiers.control) {
        (true, true) => ListSelection::AddRange,
        (true, false) => ListSelection::Range,
        (false, true) => ListSelection::Toggle,
        _ => ListSelection::Replace,
    }
}
impl Host {
    pub(crate) fn list(&self, id: NodeId) -> Option<ListModel> {
        self.0
            .mounted
            .borrow()
            .nodes
            .iter()
            .find(|n| n.id == id && n.enabled && !n.hidden && !n.layout_hidden && self.in_scope(id))
            .and_then(|n| n.collection.clone())
    }
    pub(crate) fn focused_list(&self) -> Option<(NodeId, ListModel)> {
        let id = self.focused()?;
        let m = self.0.mounted.borrow();
        let n = m.nodes.iter().find(|n| n.id == id)?;
        let parent = if n.kind == Kind::CollectionRow {
            n.parent?
        } else if n.collection.is_some() {
            n.id
        } else {
            return None;
        };
        drop(m);
        self.list(parent).map(|model| (parent, model))
    }
    pub fn select_collection_item_with(
        &self,
        id: NodeId,
        index: usize,
        operation: ListSelection,
    ) -> Result<bool, Error> {
        self.refresh()?;
        let Some(model) = self.list(id) else {
            return Ok(false);
        };
        let Some(item) = model.item(index) else {
            return Ok(false);
        };
        if !model.select_with(&item.key, operation) {
            return Ok(false);
        }
        model.reveal(index);
        self.0.interaction.borrow_mut().pending_row = Some((id, item.key));
        self.redraw();
        Ok(true)
    }
    pub(crate) fn remember_collection_focus(&self, id: NodeId) {
        let data = {
            let m = self.0.mounted.borrow();
            let mut current = Some(id);
            let mut result = None;
            while let Some(id) = current {
                if let Some(data) = m
                    .descriptions
                    .get(&id)
                    .and_then(|d| d.collection_row.clone())
                {
                    result = Some(data);
                    break;
                }
                current = m.nodes.iter().find(|n| n.id == id).and_then(|n| n.parent);
            }
            result
        };
        if let Some((model, key, token, _)) = data
            && model.item_token(&key) == Some(token)
        {
            model.focus_key(&key);
        }
    }
    pub(crate) fn collection_tab_stop(&self, node: &Node, nodes: &[Node]) -> bool {
        let Some(parent) = node.parent else {
            return false;
        };
        let Some(model) = nodes
            .iter()
            .find(|n| n.id == parent)
            .and_then(|n| n.collection.as_ref())
        else {
            return false;
        };
        let rows: Vec<_> = nodes
            .iter()
            .filter(|n| {
                n.parent == Some(parent)
                    && n.kind == Kind::CollectionRow
                    && n.enabled
                    && !n.hidden
                    && !n.layout_hidden
                    && n.bounds.intersection(n.clip).height > 0.
            })
            .collect();
        let target = model
            .cursor()
            .and_then(|key| rows.iter().find(|n| n.key.as_ref() == Some(&key)))
            .or_else(|| rows.iter().find(|n| n.value.selected()))
            .or_else(|| rows.first());
        target.is_some_and(|n| n.id == node.id)
    }
    pub(crate) fn restore_collection_focus(
        &self,
        prior: Option<NodeId>,
        old: &crate::host::Mounted,
    ) {
        let Some(prior) = prior else {
            return;
        };
        let mut row = Some(prior);
        let mut data = None;
        while let Some(id) = row {
            if let Some(value) = old
                .descriptions
                .get(&id)
                .and_then(|d| d.collection_row.clone())
            {
                data = Some((id, value));
                break;
            }
            row = old.nodes.iter().find(|n| n.id == id).and_then(|n| n.parent);
        }
        let Some((row, (model, key, token, revision))) = data else {
            return;
        };
        let Some(parent) = old
            .nodes
            .iter()
            .find(|n| n.id == row)
            .and_then(|n| n.parent)
        else {
            return;
        };
        if self.list(parent).as_ref() != Some(&model) {
            return;
        }
        if row != prior {
            if self.focused().is_none() {
                self.0.interaction.borrow_mut().focus = Some(parent);
            } else if model.revision() != revision
                && model.item_token(&key) == Some(token)
                && let Some(editor) = self.0.editors.borrow_mut().get_mut(&prior)
            {
                editor.reveal = true;
            }
            return;
        }
        let tree = self
            .0
            .mounted
            .borrow()
            .descriptions
            .get(&parent)
            .and_then(|d| d.tree.clone());
        if tree.is_some() && model.revision() != revision {
            if let Some(key) = model.cursor()
                && let Some(index) = model.index_of(&key)
            {
                model.reveal(index);
                self.0.interaction.borrow_mut().pending_row = Some((parent, key));
            }
            return;
        }
        if model.revision() != revision {
            if let Some(key) = model
                .cursor()
                .filter(|k| k != &key || model.item_token(k) == Some(token))
                && let Some(index) = model.index_of(&key)
            {
                model.reveal(index);
                self.0.interaction.borrow_mut().pending_row = Some((parent, key));
            }
        } else if self.focused().is_none() {
            // Wheel scrolling may unrealize the row. Keep logical navigation in
            // the list without snapping the viewport back to the old row.
            self.0.interaction.borrow_mut().focus = Some(parent);
        }
    }
    pub(crate) fn collection_key(&self, event: KeyEvent) -> Result<Option<bool>, Error> {
        let Some((parent, model)) = self.focused_list() else {
            return Ok(None);
        };
        if let Some(handled) = self.tree_key(parent, event)? {
            return Ok(Some(handled));
        }
        if event.key == Key::A && (event.modifiers.command || event.modifiers.control) {
            return Ok(Some(model.select_all()));
        }
        let current = model.cursor().and_then(|k| model.index_of(&k));
        if event.key == Key::Enter {
            if !event.repeat
                && let Some(key) = model.cursor()
            {
                self.activate_collection_row(parent, &key)?;
            }
            return Ok(Some(true));
        }
        if event.key == Key::Space {
            if event.repeat {
                return Ok(Some(true));
            }
            if let Some(index) = current {
                return self
                    .select_collection_item_with(parent, index, selection(event.modifiers))
                    .map(Some);
            }
            return Ok(Some(true));
        }
        let page = (model.viewport_height() / model.row_height())
            .floor()
            .max(1.) as usize;
        let had_cursor = current.is_some();
        let current = current.unwrap_or(0);
        let target = match event.key {
            Key::Home => model.enabled_from(0, true),
            Key::End => model.enabled_from(model.len().saturating_sub(1), false),
            Key::Up => current
                .checked_sub(1)
                .and_then(|i| model.enabled_from(i, false)),
            Key::Down => model.enabled_from(
                if had_cursor {
                    current.saturating_add(1)
                } else {
                    0
                },
                true,
            ),
            Key::PageUp => model
                .enabled_from(current.saturating_sub(page), false)
                .or_else(|| model.enabled_from(0, true)),
            Key::PageDown => model
                .enabled_from(
                    current
                        .saturating_add(page)
                        .min(model.len().saturating_sub(1)),
                    true,
                )
                .or_else(|| model.enabled_from(model.len().saturating_sub(1), false)),
            _ => return Ok(None),
        };
        if let Some(target) = target {
            let operation =
                if !event.modifiers.shift && (event.modifiers.command || event.modifiers.control) {
                    ListSelection::Focus
                } else {
                    selection(event.modifiers)
                };
            self.select_collection_item_with(parent, target, operation)?;
        }
        Ok(Some(true))
    }
    pub fn collection_text(&self, text: &str) -> Result<bool, Error> {
        self.collection_text_at(text, Instant::now())
    }
    pub fn collection_text_at(&self, text: &str, now: Instant) -> Result<bool, Error> {
        self.refresh()?;
        let Some((parent, model)) = self.focused_list() else {
            return Ok(false);
        };
        if text.is_empty() || text.chars().any(char::is_control) {
            return Ok(false);
        }
        if let Some(index) = model.typeahead(text, now) {
            self.select_collection_item_with(parent, index, ListSelection::Replace)?;
        }
        Ok(true)
    }
}
