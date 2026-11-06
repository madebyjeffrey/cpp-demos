//! Exact accessibility value assignments share the host's scope and binding guards.
use crate::{CheckState, Error, Host, Kind, NodeId, view::Data};
impl Host {
    pub fn set_check_state(&self, id: NodeId, value: CheckState) -> Result<bool, Error> {
        self.refresh()?;
        let data = {
            let m = self.0.mounted.borrow();
            m.nodes
                .iter()
                .find(|n| {
                    n.id == id
                        && n.kind == Kind::CheckBox
                        && n.enabled
                        && !n.hidden
                        && !n.layout_hidden
                        && !n.read_only
                        && self.in_scope(id)
                })
                .and_then(|_| m.descriptions.get(&id))
                .map(|d| d.data.clone())
        };
        let Some(Data::Check(binding)) = data else {
            return Ok(false);
        };
        binding.set(value);
        self.redraw();
        Ok(binding.get() == value)
    }
    /// Assign a boolean control's value without synthesizing a click or toggling twice.
    pub fn set_selected(&self, id: NodeId, value: bool) -> Result<bool, Error> {
        self.refresh()?;
        let data = {
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
                        && n.menu_item.is_none()
                        && n.table_cell.is_none()
                        && matches!(
                            n.kind,
                            Kind::Toggle
                                | Kind::CheckBox
                                | Kind::Switch
                                | Kind::Radio
                                | Kind::Segment
                                | Kind::Tab
                        )
                })
                .and_then(|n| m.descriptions.get(&id).map(|d| (n.kind, d.data.clone())))
        };
        let Some((kind, data)) = data else {
            return Ok(false);
        };
        if kind == Kind::Tab {
            return if value { self.activate(id) } else { Ok(false) };
        }
        match data {
            Data::Bool(binding) => {
                binding.set(value);
                self.redraw();
                Ok(binding.get() == value)
            }
            Data::Check(_) => self.set_check_state(
                id,
                if value {
                    CheckState::Checked
                } else {
                    CheckState::Unchecked
                },
            ),
            _ => Ok(false),
        }
    }
}
