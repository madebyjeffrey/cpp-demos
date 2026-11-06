use crate::{ComboBox, Error, Host, Key, KeyEvent, NodeId, TextCommand};
impl Host {
    pub(crate) fn combo_field(&self, model: &ComboBox) -> Option<NodeId> {
        self.0
            .mounted
            .borrow()
            .nodes
            .iter()
            .find(|n| n.key.as_deref() == Some(model.field_key().as_str()))
            .map(|n| n.id)
    }
    pub(crate) fn combo_focus_owner(&self, id: NodeId) -> Option<NodeId> {
        let model = {
            let m = self.0.mounted.borrow();
            let d = m.descriptions.get(&id)?;
            d.combo_action.clone().or_else(|| {
                m.nodes
                    .iter()
                    .find(|n| n.id == id)
                    .and_then(|n| n.parent)
                    .and_then(|p| m.descriptions.get(&p))
                    .and_then(|d| d.combo_popup.clone())
            })?
        };
        self.combo_field(&model)
    }
    pub(crate) fn toggle_combo(&self, _id: NodeId, model: &ComboBox) -> Result<bool, Error> {
        if model.info().read_only {
            return Ok(false);
        }
        if let Some(field) = self.combo_field(model) {
            self.focus(field)?;
        }
        if model.is_open() {
            model.cancel();
        } else {
            model.show();
        }
        self.redraw();
        Ok(true)
    }
    pub(crate) fn accept_combo_row(&self, id: NodeId, key: &str) -> Result<Option<bool>, Error> {
        let model = self
            .0
            .mounted
            .borrow()
            .descriptions
            .get(&id)
            .and_then(|d| d.combo_popup.clone());
        let Some(model) = model else { return Ok(None) };
        if self.unmark_text(true)? {
            return Ok(Some(true));
        }
        if let Some(field) = self.combo_field(&model) {
            self.focus(field)?;
        }
        let accepted = model.accept_key(key);
        if accepted {
            self.refresh()?;
        }
        self.redraw();
        Ok(Some(accepted))
    }
    pub(crate) fn combo_key(&self, event: KeyEvent) -> Result<Option<bool>, Error> {
        let model = self.focused().and_then(|id| {
            self.0
                .mounted
                .borrow()
                .descriptions
                .get(&id)
                .and_then(|d| d.combo.clone())
        });
        let Some(model) = model else { return Ok(None) };
        let open = model.is_open();
        let handled = match event.key {
            Key::Up | Key::Down => {
                if self.text_input()?.is_some_and(|i| i.marked.is_some()) {
                    return Ok(None);
                }
                model.navigate(event.key)
            }
            Key::Home | Key::End | Key::PageUp | Key::PageDown if open => {
                if self.text_input()?.is_some_and(|i| i.marked.is_some()) {
                    return Ok(None);
                }
                model.navigate(event.key)
            }
            Key::Enter if open => {
                if !event.repeat && !self.unmark_text(true)? {
                    model.accept();
                }
                true
            }
            Key::Escape if open => model.cancel(),
            Key::Tab if open && !event.modifiers.control => {
                self.unmark_text(true)?;
                model.cancel();
                // Retire the popup scope before normal Tab traversal.
                self.refresh()?;
                self.reconcile_overlays()?;
                return self.focus_next(event.modifiers.shift).map(Some);
            }
            _ => return Ok(None),
        };
        self.redraw();
        Ok(Some(handled))
    }
    pub(crate) fn combo_command(
        &self,
        command: TextCommand,
        extend: bool,
    ) -> Result<Option<bool>, Error> {
        let key = match command {
            TextCommand::Up => Key::Up,
            TextCommand::Down => Key::Down,
            TextCommand::Start => Key::Home,
            TextCommand::End => Key::End,
            TextCommand::PageUp => Key::PageUp,
            TextCommand::PageDown => Key::PageDown,
            TextCommand::Enter => Key::Enter,
            TextCommand::Escape => Key::Escape,
            TextCommand::Tab | TextCommand::BackTab => Key::Tab,
            _ => return Ok(None),
        };
        if command == TextCommand::Escape && self.text_input()?.is_some_and(|i| i.marked.is_some())
        {
            return Ok(None);
        }
        self.combo_key(KeyEvent {
            key,
            down: true,
            repeat: false,
            modifiers: crate::Modifiers {
                shift: extend || command == TextCommand::BackTab,
                ..Default::default()
            },
        })
    }
}
