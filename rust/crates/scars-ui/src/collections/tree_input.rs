//! Tree disclosure and parent/child navigation reuse keyed list selection/reveal.
use crate::{Error, Host, Key, KeyEvent, ListSelection, NodeId, TreeModel};
impl Host {
    pub(crate) fn tree(&self, parent: NodeId) -> Option<TreeModel> {
        self.0
            .mounted
            .borrow()
            .descriptions
            .get(&parent)
            .and_then(|d| d.tree.clone())
    }
    pub fn set_tree_selection(&self, parent: NodeId, keys: &[String]) -> Result<bool, Error> {
        self.refresh()?;
        let Some(model) = self.tree(parent).filter(|_| self.list(parent).is_some()) else {
            return Ok(false);
        };
        let inside = self
            .focused()
            .is_some_and(|id| Self::within(&self.0.mounted.borrow().nodes, id, parent));
        model.set_selection(keys)?;
        if inside
            && let Some(key) = model.cursor()
            && let Some(index) = self.list(parent).and_then(|m| m.index_of(&key))
        {
            self.list(parent).unwrap().reveal(index);
            self.0.interaction.borrow_mut().pending_row = Some((parent, key));
        }
        self.redraw();
        Ok(true)
    }
    pub fn set_tree_disclosed(
        &self,
        parent: NodeId,
        key: &str,
        token: u64,
        expanded: bool,
    ) -> Result<bool, Error> {
        self.refresh()?;
        let Some(model) = self.tree(parent).filter(|_| self.list(parent).is_some()) else {
            return Ok(false);
        };
        if model
            .row(key)
            .is_none_or(|r| r.token != token || !r.item.enabled || !r.item.expandable)
        {
            return Ok(false);
        }
        let handled = model.ui_expansion(key, expanded);
        self.redraw();
        Ok(handled)
    }
    pub fn activate_tree_item(&self, parent: NodeId, key: &str, token: u64) -> Result<bool, Error> {
        self.refresh()?;
        let Some(model) = self.tree(parent).filter(|_| self.list(parent).is_some()) else {
            return Ok(false);
        };
        if model
            .row(key)
            .is_none_or(|r| r.token != token || !r.item.enabled)
        {
            return Ok(false);
        }
        let handled = model.ui_activate(key);
        self.redraw();
        Ok(handled)
    }
    pub(crate) fn tree_row_owner(&self, id: NodeId) -> Option<NodeId> {
        let m = self.0.mounted.borrow();
        m.descriptions.get(&id)?.tree_action.as_ref()?;
        let mut parent = m.nodes.iter().find(|n| n.id == id)?.parent;
        while let Some(id) = parent {
            if m.descriptions
                .get(&id)
                .is_some_and(|d| d.collection_row.is_some())
            {
                return Some(id);
            }
            parent = m.nodes.iter().find(|n| n.id == id)?.parent;
        }
        None
    }
    pub(crate) fn tree_disclosure(
        &self,
        id: NodeId,
        action: &super::TreeAction,
    ) -> Result<bool, Error> {
        self.unmark_text(true)?;
        self.refresh()?;
        let Some(row) = self.tree_row_owner(id) else {
            return Ok(false);
        };
        let parent = self
            .0
            .mounted
            .borrow()
            .nodes
            .iter()
            .find(|n| n.id == row)
            .and_then(|n| n.parent);
        let Some(parent) = parent.filter(|p| self.list(*p).is_some()) else {
            return Ok(false);
        };
        if !self.in_scope(id) || action.model.item_token(&action.key) != Some(action.token) {
            return Ok(false);
        }
        self.focus(row)?;
        let handled = action
            .model
            .ui_disclose(&action.key, action.token, action.retry);
        self.refresh()?;
        if let Some(key) = action.model.cursor() {
            self.0.interaction.borrow_mut().pending_row = Some((parent, key));
        }
        self.redraw();
        Ok(handled)
    }
    pub(crate) fn activate_collection_row(&self, parent: NodeId, key: &str) -> Result<bool, Error> {
        self.refresh()?;
        let Some(model) = self.list(parent) else {
            return Ok(false);
        };
        if let Some(table) = self.table(parent) {
            let column = if table.options().selection_mode == crate::TableSelectionMode::Cells {
                table.cursor().map(|c| c.column)
            } else {
                table
                    .columns()
                    .iter()
                    .find(|c| c.policy.editable)
                    .map(|c| c.id.clone())
            };
            if let Some(column) = column {
                return self.start_table_cell_edit(parent, crate::TableCell::new(key, column));
            }
            return Ok(false);
        }
        Ok(if let Some(tree) = self.tree(parent) {
            tree.ui_activate(key)
        } else {
            model.activate(key)
        })
    }
    pub(crate) fn tree_key(&self, parent: NodeId, event: KeyEvent) -> Result<Option<bool>, Error> {
        if !matches!(event.key, Key::Left | Key::Right) {
            return Ok(None);
        }
        let Some(tree) = self.tree(parent) else {
            return Ok(None);
        };
        let Some(key) = tree.cursor() else {
            return Ok(Some(true));
        };
        let Some(info) = tree.item(&key) else {
            return Ok(Some(true));
        };
        let target = match event.key {
            Key::Right if info.expandable && !info.expanded => {
                tree.ui_expansion(&key, true);
                None
            }
            Key::Right => tree.child(&key),
            Key::Left if info.expandable && info.expanded => {
                tree.ui_expansion(&key, false);
                None
            }
            _ => tree.parent(&key),
        };
        self.refresh()?;
        if let Some(key) = target
            && let Some(index) = tree.index_of(&key)
        {
            let operation =
                if !event.modifiers.shift && (event.modifiers.command || event.modifiers.control) {
                    ListSelection::Focus
                } else {
                    super::input::selection(event.modifiers)
                };
            self.select_collection_item_with(parent, index, operation)?;
        }
        self.redraw();
        Ok(Some(true))
    }
}
