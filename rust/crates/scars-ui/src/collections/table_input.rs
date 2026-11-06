use crate::{
    Error, Host, Key, KeyEvent, Kind, ListSelection, Node, NodeId, TableCell, TableModel,
    TableSelectionMode,
};
#[derive(Clone)]
pub(crate) struct TableDrag {
    pub id: NodeId,
    pub model: TableModel,
    pub column: String,
    pub token: u64,
    pub resize: bool,
    pub start: f64,
    pub original: f64,
    pub last: f64,
    pub moved: bool,
    pub order: Vec<String>,
}
impl Host {
    pub(crate) fn table(&self, parent: NodeId) -> Option<TableModel> {
        self.0
            .mounted
            .borrow()
            .descriptions
            .get(&parent)
            .and_then(|d| d.table.clone())
    }
    pub(crate) fn table_container(&self, id: NodeId) -> Option<NodeId> {
        let m = self.0.mounted.borrow();
        let mut p = Some(id);
        while let Some(id) = p {
            if m.descriptions.get(&id).is_some_and(|d| d.table.is_some()) {
                return Some(id);
            }
            p = m.nodes.iter().find(|n| n.id == id)?.parent;
        }
        None
    }
    pub(crate) fn table_focus_owner(&self, id: NodeId) -> Option<NodeId> {
        let m = self.0.mounted.borrow();
        let n = m.nodes.iter().find(|n| n.id == id)?;
        if m.descriptions.get(&id)?.table_editor.is_some() {
            return None;
        }
        if let Some(action) = m.descriptions.get(&id)?.table_cell.as_ref()
            && action.model.options().selection_mode == TableSelectionMode::Rows
        {
            let mut p = n.parent;
            while let Some(id) = p {
                let n = m.nodes.iter().find(|n| n.id == id)?;
                if n.kind == Kind::CollectionRow {
                    return Some(id);
                }
                p = n.parent;
            }
        }
        if n.kind == Kind::CollectionRow
            && let Some(model) = n
                .parent
                .and_then(|p| m.descriptions.get(&p))
                .and_then(|d| d.table.as_ref())
            && (model.options().selection_mode == TableSelectionMode::Cells
                || model
                    .active_edit()
                    .and_then(|e| e.info())
                    .is_some_and(|i| Some(&i.cell.row) == n.key.as_ref()))
        {
            let column = model
                .active_edit()
                .and_then(|e| e.info())
                .filter(|i| Some(&i.cell.row) == n.key.as_ref())
                .map(|i| i.cell)
                .or_else(|| model.cursor())
                .map(|c| c.column)
                .or_else(|| model.columns().first().map(|c| c.id.clone()))?;
            return m
                .nodes
                .iter()
                .find(|cell| {
                    cell.table_cell
                        .as_ref()
                        .is_some_and(|c| Some(&c.row) == n.key.as_ref() && c.column == column)
                        && Host::within(&m.nodes, cell.id, id)
                })
                .map(|n| n.id);
        }
        None
    }
    pub(crate) fn remember_table_focus(&self, id: NodeId) {
        let action = self
            .0
            .mounted
            .borrow()
            .descriptions
            .get(&id)
            .and_then(|d| d.table_cell.clone());
        if let Some(a) = action
            && a.model.options().selection_mode == TableSelectionMode::Cells
        {
            a.model.select_cell(a.cell, ListSelection::Focus);
        }
    }
    pub(crate) fn table_tab_stop(&self, n: &Node, nodes: &[Node]) -> Option<bool> {
        if n.table_editor.is_some() {
            return Some(true);
        }
        if n.table_cell.is_some() {
            let m = self.0.mounted.borrow();
            let action = m.descriptions[&n.id].table_cell.as_ref()?;
            if action.model.active_edit().is_some() {
                return Some(false);
            }
            if action.model.options().selection_mode == TableSelectionMode::Rows {
                return Some(false);
            }
            let root = self.table_container(n.id)?;
            let cursor = action.model.cursor();
            let target = nodes
                .iter()
                .filter(|p| {
                    p.enabled
                        && !p.hidden
                        && !p.layout_hidden
                        && p.table_cell.is_some()
                        && Host::within(nodes, p.id, root)
                })
                .find(|p| p.table_cell == cursor)
                .or_else(|| {
                    nodes.iter().find(|p| {
                        p.enabled
                            && !p.hidden
                            && !p.layout_hidden
                            && p.table_cell.is_some()
                            && Host::within(nodes, p.id, root)
                    })
                });
            return Some(target.is_some_and(|p| p.id == n.id));
        }
        if n.kind == Kind::CollectionRow
            && n.parent
                .and_then(|p| self.table(p))
                .is_some_and(|m| m.options().selection_mode == TableSelectionMode::Cells)
        {
            return Some(false);
        }
        None
    }
    pub(crate) fn table_select_cell(
        &self,
        id: NodeId,
        action: &super::TableCellAction,
        operation: ListSelection,
    ) -> Result<bool, Error> {
        if action.model.row_token(&action.cell.row) != Some(action.row_token)
            || action.model.column_token(&action.cell.column) != Some(action.column_token)
        {
            return Ok(false);
        }
        let Some(parent) = self.table_container(id) else {
            return Ok(false);
        };
        if action.model.options().selection_mode == TableSelectionMode::Rows {
            return self.select_collection_item_with(
                parent,
                action.model.row_index(&action.cell.row).unwrap(),
                operation,
            );
        }
        if !action.model.select_cell(action.cell.clone(), operation) {
            return Ok(false);
        }
        let index = action.model.row_index(&action.cell.row).unwrap();
        action.model.0.list.reveal(index);
        self.0.interaction.borrow_mut().pending_row = Some((parent, action.cell.row.clone()));
        self.redraw();
        Ok(true)
    }
    pub(crate) fn table_select_row(
        &self,
        parent: NodeId,
        key: &str,
        operation: ListSelection,
    ) -> Option<bool> {
        let model = self.table(parent)?;
        if model.options().selection_mode != TableSelectionMode::Cells {
            return None;
        }
        let column = model
            .cursor()
            .map(|c| c.column)
            .or_else(|| model.columns().first().map(|c| c.id.clone()));
        Some(column.is_some_and(|column| model.select_cell(TableCell::new(key, column), operation)))
    }
    pub(crate) fn table_key(&self, parent: NodeId, event: KeyEvent) -> Result<Option<bool>, Error> {
        let Some(model) = self.table(parent) else {
            return Ok(None);
        };
        if event.key == Key::A && (event.modifiers.command || event.modifiers.control) {
            return Ok(Some(model.select_all()));
        }
        if model.options().selection_mode != TableSelectionMode::Cells {
            return Ok(None);
        }
        if matches!(event.key, Key::Left | Key::Right) {
            let cursor = model.cursor().or_else(|| {
                model
                    .row_cursor()
                    .and_then(|r| model.columns().first().map(|c| TableCell::new(r, &c.id)))
            });
            if let Some(mut cell) = cursor {
                let columns = model.columns();
                if let Some(i) = model.column_index(&cell.column) {
                    let target = if event.key == Key::Left {
                        i.saturating_sub(1)
                    } else {
                        (i + 1).min(columns.len() - 1)
                    };
                    cell.column = columns[target].id.clone();
                    let operation = if !event.modifiers.shift
                        && (event.modifiers.command || event.modifiers.control)
                    {
                        ListSelection::Focus
                    } else {
                        super::input::selection(event.modifiers)
                    };
                    model.select_cell(cell.clone(), operation);
                    self.0.interaction.borrow_mut().pending_row = Some((parent, cell.row));
                    self.redraw();
                }
            }
            return Ok(Some(true));
        }
        Ok(None)
    }
    pub(crate) fn table_header_key(
        &self,
        id: NodeId,
        event: KeyEvent,
    ) -> Result<Option<bool>, Error> {
        let action = self
            .0
            .mounted
            .borrow()
            .descriptions
            .get(&id)
            .and_then(|d| d.table_header.clone());
        let Some(a) = action else { return Ok(None) };
        if a.model.column_token(&a.column) != Some(a.token) {
            return Ok(Some(false));
        }
        if event.key == Key::Escape {
            let drag = self.0.interaction.borrow_mut().table_drag.take();
            if let Some(drag) = drag {
                if drag.resize {
                    drag.model.resize_column(&drag.column, drag.original)?;
                }
                self.redraw();
                return Ok(Some(true));
            }
            return Ok(None);
        }
        if a.resize {
            let c = a
                .model
                .columns()
                .into_iter()
                .find(|c| c.id == a.column)
                .unwrap();
            let width = match event.key {
                Key::Left => c.width - 8.,
                Key::Right => c.width + 8.,
                Key::Home => c.min_width,
                Key::End => c.max_width,
                _ => return Ok(None),
            };
            a.model.resize_column(&a.column, width)?;
        } else if event.modifiers.alt && matches!(event.key, Key::Left | Key::Right) {
            let index = a.model.column_index(&a.column).unwrap();
            let target = if event.key == Key::Left {
                index.saturating_sub(1)
            } else {
                (index + 1).min(a.model.columns().len() - 1)
            };
            a.model.move_column(&a.column, target)?;
        } else if matches!(event.key, Key::Enter | Key::Space) {
            if !event.repeat
                && a.model
                    .columns()
                    .iter()
                    .any(|c| c.id == a.column && c.sortable)
            {
                a.model.toggle_sort(&a.column, event.modifiers.shift)?;
            }
        } else {
            return Ok(None);
        }
        self.redraw();
        Ok(Some(true))
    }
    pub(crate) fn reconcile_table_capture(&self) {
        let invalid = self
            .0
            .interaction
            .borrow()
            .table_drag
            .as_ref()
            .is_some_and(|d| !self.table_drag_valid(d));
        if invalid {
            let mut i = self.0.interaction.borrow_mut();
            i.table_drag = None;
            i.pressed = None;
        }
    }
    fn table_drag_valid(&self, d: &TableDrag) -> bool {
        d.model.column_token(&d.column) == Some(d.token)
            && d.model
                .columns()
                .iter()
                .map(|c| c.id.clone())
                .collect::<Vec<_>>()
                == d.order
            && (!d.resize
                || d.model
                    .columns()
                    .iter()
                    .any(|c| c.id == d.column && c.width == d.last && c.resizable))
            && self.0.mounted.borrow().nodes.iter().any(|n| {
                n.id == d.id && n.enabled && !n.hidden && !n.layout_hidden && self.in_scope(n.id)
            })
    }
    pub(crate) fn table_pointer(&self, event: crate::Pointer, clicks: u32) -> Result<bool, Error> {
        use crate::PointerKind::*;
        let drag = self.0.interaction.borrow_mut().table_drag.take();
        if let Some(mut d) = drag {
            if !self.table_drag_valid(&d) {
                self.0.interaction.borrow_mut().pressed = None;
                return Ok(true);
            }
            match event.kind {
                Move => {
                    if (event.position.x - d.start).abs() > 4. {
                        d.moved = true;
                    }
                    if d.resize {
                        d.model
                            .resize_column(&d.column, d.original + event.position.x - d.start)?;
                        d.last = d
                            .model
                            .columns()
                            .iter()
                            .find(|c| c.id == d.column)
                            .unwrap()
                            .width;
                    }
                    self.0.interaction.borrow_mut().table_drag = Some(d);
                }
                Up => {
                    self.0.interaction.borrow_mut().pressed = None;
                    if !d.resize {
                        if d.moved {
                            let target = self
                                .0
                                .mounted
                                .borrow()
                                .nodes
                                .iter()
                                .filter(|n| {
                                    n.table_column.is_some()
                                        && !n.table_resize
                                        && n.clip.contains(event.position)
                                        && n.bounds.contains(event.position)
                                })
                                .filter_map(|n| {
                                    self.0
                                        .mounted
                                        .borrow()
                                        .descriptions
                                        .get(&n.id)
                                        .and_then(|v| v.table_header.as_ref())
                                        .filter(|a| a.model == d.model)
                                        .map(|a| a.column.clone())
                                })
                                .next();
                            if let Some(target) = target {
                                let index = d.model.column_index(&target).unwrap();
                                d.model.move_column(&d.column, index)?;
                            }
                        } else if d
                            .model
                            .columns()
                            .iter()
                            .any(|c| c.id == d.column && c.sortable)
                        {
                            d.model.toggle_sort(&d.column, event.modifiers.shift)?;
                        }
                    }
                }
                Cancel => {
                    self.0.interaction.borrow_mut().pressed = None;
                    if d.resize {
                        d.model.resize_column(&d.column, d.original)?;
                    }
                }
                _ => self.0.interaction.borrow_mut().table_drag = Some(d),
            }
            self.redraw();
            return Ok(true);
        }
        if !self.0.interaction.borrow().active {
            return Ok(false);
        }
        let hit = self.hit_test(event.position)?;
        if event.kind == Up
            && let Some(id) = hit
            && self.0.interaction.borrow().pressed == Some(id)
        {
            let action = self
                .0
                .mounted
                .borrow()
                .descriptions
                .get(&id)
                .filter(|d| d.table_editor.is_none())
                .and_then(|d| d.table_cell.clone());
            if let Some(action) = action {
                self.0.interaction.borrow_mut().pressed = None;
                let selected =
                    self.table_select_cell(id, &action, super::input::selection(event.modifiers))?;
                if selected && clicks >= 2 {
                    return self.start_table_cell_edit(id, action.cell);
                }
                return Ok(selected);
            }
        }
        if event.kind != Down || event.button != 0 {
            return Ok(false);
        }
        let Some(id) = hit else { return Ok(false) };
        let action = self
            .0
            .mounted
            .borrow()
            .descriptions
            .get(&id)
            .and_then(|d| d.table_header.clone());
        let Some(a) = action else { return Ok(false) };
        let Some(c) = a.model.columns().into_iter().find(|c| c.id == a.column) else {
            return Ok(false);
        };
        self.focus(id)?;
        let drag = TableDrag {
            id,
            model: a.model.clone(),
            column: a.column,
            token: a.token,
            resize: a.resize,
            start: event.position.x,
            original: c.width,
            last: c.width,
            moved: false,
            order: a.model.columns().into_iter().map(|c| c.id).collect(),
        };
        let mut i = self.0.interaction.borrow_mut();
        i.pressed = Some(id);
        i.table_drag = Some(drag);
        drop(i);
        self.redraw();
        Ok(true)
    }
}
