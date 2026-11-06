use crate::{Error, Host, NodeId, TableCell, TextCommand};
impl Host {
    pub(crate) fn start_table_cell_edit(&self, id: NodeId, cell: TableCell) -> Result<bool, Error> {
        self.unmark_text(true)?;
        self.refresh()?;
        let Some(parent) = self.table_container(id) else {
            return Ok(false);
        };
        let Some(model) = self.table(parent) else {
            return Ok(false);
        };
        if !model.is_editable(&cell) {
            return Ok(false);
        }
        if let Some(edit) = model.active_edit() {
            if edit.info().is_some_and(|i| i.cell == cell) {
                edit.focus();
                self.redraw();
                return Ok(true);
            }
            if model.commit_edit().is_err() {
                if let Some(info) = edit.info() {
                    if model.options().selection_mode == crate::TableSelectionMode::Cells {
                        model.select_cell(info.cell, crate::ListSelection::Replace);
                    } else {
                        model.select_row(&info.cell.row, crate::ListSelection::Replace);
                    }
                }
                edit.focus();
                self.redraw();
                return Ok(true);
            }
        }
        if model.start_edit(cell).is_err() {
            return Ok(false);
        }
        self.redraw();
        Ok(true)
    }
    pub(crate) fn focus_requested_table_editor(&self) -> Result<(), Error> {
        let target = self
            .0
            .mounted
            .borrow()
            .nodes
            .iter()
            .filter(|n| n.enabled && !n.hidden && !n.layout_hidden)
            .filter_map(|n| {
                n.table_editor
                    .as_ref()
                    .filter(|e| e.wants_focus())
                    .map(|e| (n.id, e.clone()))
            })
            .next();
        if let Some((id, edit)) = target
            && self.in_scope(id)
            && edit.take_focus()
        {
            self.focus(id)?;
        }
        Ok(())
    }
    pub(crate) fn table_edit_command(
        &self,
        command: TextCommand,
        extend: bool,
    ) -> Result<Option<bool>, Error> {
        let id = self.focused();
        let editor = id.and_then(|id| {
            self.0
                .mounted
                .borrow()
                .descriptions
                .get(&id)
                .and_then(|d| d.table_editor.clone())
        });
        let Some(edit) = editor else { return Ok(None) };
        if !matches!(
            command,
            TextCommand::Enter | TextCommand::Escape | TextCommand::Tab | TextCommand::BackTab
        ) {
            return Ok(None);
        }
        if self.text_input()?.is_some_and(|i| i.marked.is_some()) {
            if command == TextCommand::Enter {
                return self.unmark_text(true).map(Some);
            }
            if command == TextCommand::Escape {
                return self.unmark_text(false).map(Some);
            }
        }
        if command == TextCommand::Enter && edit.multiline() {
            return Ok(None);
        }
        self.finish_table_edit(
            id.unwrap(),
            command == TextCommand::Escape,
            match command {
                TextCommand::Tab => Some(extend),
                TextCommand::BackTab => Some(true),
                _ => None,
            },
        )
        .map(Some)
    }
    pub(crate) fn finish_table_edit(
        &self,
        id: NodeId,
        cancel: bool,
        tab: Option<bool>,
    ) -> Result<bool, Error> {
        let action = self
            .0
            .mounted
            .borrow()
            .descriptions
            .get(&id)
            .and_then(|d| d.table_cell.clone());
        let Some(action) = action else {
            return Ok(false);
        };
        let parent = self.table_container(id);
        self.unmark_text(!cancel)?;
        let accepted = if cancel {
            action.model.cancel_edit()
        } else {
            match action.model.commit_edit() {
                Ok(value) => value,
                Err(_) => {
                    if let Some(edit) = action.model.active_edit() {
                        edit.focus();
                    }
                    self.redraw();
                    return Ok(true);
                }
            }
        };
        if accepted
            && (self.focused() == Some(id) || self.focused().is_none())
            && let Some(parent) = parent
        {
            let mut i = self.0.interaction.borrow_mut();
            i.pending_row = Some((parent, action.cell.row));
            i.pending_table_tab = tab;
        }
        self.redraw();
        Ok(accepted)
    }
    /// Retire scoped table edits when a native mount is detached, even if its host is retained.
    pub fn cancel_table_edits(&self) {
        let models = self
            .0
            .mounted
            .borrow()
            .nodes
            .iter()
            .filter_map(|n| n.table_model.clone())
            .collect::<Vec<_>>();
        for model in models {
            model.cancel_edit();
        }
    }
    pub(crate) fn retire_table_edits(&self, old: &crate::host::Mounted) {
        let models = old
            .nodes
            .iter()
            .filter_map(|n| n.table_model.clone())
            .collect::<Vec<_>>();
        for model in models {
            if !self
                .0
                .mounted
                .borrow()
                .nodes
                .iter()
                .any(|n| n.table_model.as_ref() == Some(&model))
            {
                model.cancel_edit();
            }
        }
    }
}
