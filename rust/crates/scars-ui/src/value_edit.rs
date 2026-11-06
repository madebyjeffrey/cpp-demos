//! Host actions must not commit stale typed text while its editor owns preedit.
use crate::{Host, NodeId, View};
use std::rc::Rc;
impl View {
    pub(crate) fn value_editor(mut self) -> Self {
        Rc::make_mut(&mut self.0).value_editor = true;
        self
    }
    pub(crate) fn value_cancel(mut self) -> Self {
        Rc::make_mut(&mut self.0).value_cancel = true;
        self
    }
}
impl Host {
    pub(crate) fn value_owner(&self, id: NodeId) -> Option<NodeId> {
        let m = self.0.mounted.borrow();
        let (mut current, mut root) = (Some(id), None);
        while let Some(id) = current {
            if m.descriptions.get(&id).is_some_and(|d| d.value_editor) {
                root = Some(id);
            }
            current = m.nodes.iter().find(|n| n.id == id).and_then(|n| n.parent);
        }
        root
    }
    pub(crate) fn value_composing(&self, id: NodeId) -> bool {
        let Some(root) = self.value_owner(id) else {
            return false;
        };
        let m = self.0.mounted.borrow();
        self.0
            .editors
            .borrow()
            .iter()
            .any(|(id, e)| e.marked.is_some() && Self::within(&m.nodes, *id, root))
    }
    pub(crate) fn cancel_value_composition(&self, id: NodeId) {
        let Some(root) = self.value_owner(id) else {
            return;
        };
        let m = self.0.mounted.borrow();
        for (id, editor) in self.0.editors.borrow_mut().iter_mut() {
            if Self::within(&m.nodes, *id, root) {
                editor.unmark(false);
            }
        }
        drop(m);
        self.redraw();
    }
}
