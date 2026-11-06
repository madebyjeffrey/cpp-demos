use scars_ui::*;
use std::{cell::Cell, rc::Rc};
struct Metrics;
impl Measure for Metrics {
    fn text(&self, s: &str, _: TextStyle, _: f64) -> TextLayout {
        TextLayout {
            size: Size::new(s.len() as f64 * 7., 18.),
            baseline: 14.,
            lines: vec![],
            placeholders: vec![],
            hit_test: Rc::new(|_| 0),
            range_rects: Rc::new(|_| vec![]),
        }
    }
    fn image(&self, _: &Bitmap) -> Result<Size, Error> {
        Ok(Size::default())
    }
}
fn layout(h: &Host) {
    h.layout(Size::new(520., 320.), &Metrics).unwrap();
}
fn table(rows: usize) -> TableModel {
    TableModel::with_options(
        vec![
            TableColumn::new("name", "Name")
                .editable(true)
                .validate(|_, s| {
                    if s.is_empty() {
                        Err(Error("Name is required".into()))
                    } else {
                        Ok(())
                    }
                }),
            TableColumn::new("rank", "Rank").sortable(false),
        ],
        (0..rows)
            .map(|i| {
                TableRow::new(i.to_string())
                    .cell("name", format!("Name {i:06}"))
                    .cell("rank", i.to_string())
                    .enabled(i != 1)
            })
            .collect(),
        TableOptions {
            selection_mode: TableSelectionMode::Cells,
            multiple_selection: true,
            ..Default::default()
        },
    )
    .unwrap()
}
fn host(t: &TableModel) -> Host {
    let t = t.clone();
    let h =
        Host::new(move || View::column(vec![t.view(), View::button("After", || {}).key("after")]));
    layout(&h);
    h
}
fn root(h: &Host) -> NodeId {
    h.snapshot()
        .unwrap()
        .iter()
        .find(|n| n.table_model.is_some())
        .unwrap()
        .id
}
fn selection(cells: &[TableCell]) -> TableSelection {
    TableSelection {
        cells: cells.iter().cloned().collect(),
        ..Default::default()
    }
}
#[test]
fn snapshots_preserve_display_order_data_tokens_and_sort_without_retaining_callbacks() {
    struct Resource(Rc<Cell<bool>>);
    impl Drop for Resource {
        fn drop(&mut self) {
            self.0.set(true);
        }
    }
    let dropped = Rc::new(Cell::new(false));
    let resource = Resource(dropped.clone());
    let t = TableModel::with_columns(
        vec![
            TableColumn::new("name", "Name")
                .editable(true)
                .validate(move |_, _| {
                    let _ = &resource;
                    Ok(())
                }),
        ],
        vec![
            TableRow::new("a").cell("name", "Zulu"),
            TableRow::new("b").cell("name", "Alpha"),
        ],
    )
    .unwrap();
    let initial = t.snapshot();
    let a = t.cell_info(&TableCell::new("a", "name")).unwrap();
    t.set_sort(vec![SortDescriptor::new("name", SortOrder::Ascending)])
        .unwrap();
    let sorted = t.snapshot();
    assert_eq!(sorted.cell(0, 0), Some(TableCell::new("b", "name")));
    assert_eq!(sorted.column(0).unwrap().sort, Some(SortOrder::Ascending));
    assert_eq!(sorted.column(0).unwrap().sort_priority, Some(0));
    assert_eq!(initial.text(0, 0), Some("Zulu"));
    assert_eq!(initial.column(0).unwrap().sort, None);
    assert_eq!(t.cell_info(&a.cell).unwrap().row_token, a.row_token);
    let mut columns = t.columns();
    columns.push(TableColumn::new("rank", "Rank"));
    t.set_columns(columns).unwrap();
    t.set_sort(vec![
        SortDescriptor::new("name", SortOrder::Descending),
        SortDescriptor::new("rank", SortOrder::Ascending),
    ])
    .unwrap();
    assert_eq!(t.snapshot().column(1).unwrap().sort_priority, Some(1));
    assert_eq!(
        t.snapshot().column(1).unwrap().sort,
        Some(SortOrder::Ascending)
    );
    t.set_columns(vec![]).unwrap();
    assert!(
        dropped.get(),
        "snapshots must not retain validator captures"
    );
    t.set_columns(vec![TableColumn::new("name", "Renamed")])
        .unwrap();
    assert_ne!(t.column_token("name"), Some(a.column_token));
    t.set_rows(vec![]).unwrap();
    t.set_rows(vec![TableRow::new("a").cell("name", "New lifetime")])
        .unwrap();
    assert_ne!(t.row_token("a"), Some(a.row_token));
    assert_eq!(initial.rows().item_token("a"), Some(a.row_token));
    assert_eq!(initial.column(0).unwrap().token, a.column_token);
    assert!(initial.cell(usize::MAX, 0).is_none());
    assert!(initial.cell(0, usize::MAX).is_none());
}
#[test]
fn replacement_selection_is_atomic_and_respects_mode_and_enabled_targets() {
    let t = table(4);
    let a = TableCell::new("0", "name");
    let b = TableCell::new("2", "rank");
    t.set_selection(selection(&[a.clone(), b.clone()])).unwrap();
    assert_eq!(t.selection_count(), 2);
    assert_eq!(t.selected_cells(1, 1), vec![b.clone()]);
    assert!(t.selected_cells(usize::MAX, 2).is_empty());
    for invalid in [
        TableCell::new("1", "name"),
        TableCell::new("missing", "name"),
        TableCell::new("0", "missing"),
    ] {
        assert!(t.set_selection(selection(&[a.clone(), invalid])).is_err());
        assert_eq!(t.selection_count(), 2);
    }
    let wrong_mode = TableSelection {
        rows: ["0".into()].into(),
        ..Default::default()
    };
    assert!(t.set_selection(wrong_mode.clone()).is_err());
    t.set_selection_mode(TableSelectionMode::Rows, false);
    assert!(t.set_selection(selection(&[a])).is_err());
    assert!(
        t.set_selection(TableSelection {
            rows: ["0".into(), "2".into()].into(),
            ..Default::default()
        })
        .is_err()
    );
    t.set_selection(wrong_mode).unwrap();
    assert_eq!(t.selected_rows(0, 1), vec!["0"]);
    assert!(t.cell_info(&b).is_some_and(|i| !i.selected && !i.editable));
    assert!(t.cell_info(&TableCell::new("0", "rank")).unwrap().selected);
    t.set_selection(Default::default()).unwrap();
    assert_eq!(t.selection_count(), 0);
}
#[test]
fn logical_host_actions_are_bounded_validated_and_preserve_outside_focus() {
    let t = table(100_000);
    let h = host(&t);
    let id = root(&h);
    let target = TableCell::new("99999", "name");
    let info = t.cell_info(&target).unwrap();
    assert_eq!(info.row_index, 99999);
    assert!(!info.editing);
    let after = h
        .snapshot()
        .unwrap()
        .iter()
        .find(|n| n.key.as_deref() == Some("after"))
        .unwrap()
        .id;
    h.focus(after).unwrap();
    assert!(
        h.set_table_selection(id, selection(std::slice::from_ref(&target)))
            .unwrap()
    );
    assert_eq!(h.focused(), Some(after));
    assert!(t.realized_row_count() < 30);
    assert!(
        h.edit_table_cell(id, target.clone(), info.row_token, info.column_token)
            .unwrap()
    );
    layout(&h);
    assert!(t.cell_info(&target).unwrap().editing);
    assert!(
        h.set_table_cell_text(id, target.clone(), info.row_token, info.column_token, "")
            .is_err()
    );
    let invalid = t.cell_info(&target).unwrap();
    assert_eq!(invalid.text, "");
    assert_eq!(invalid.error.as_deref(), Some("Name is required"));
    assert_eq!(t.text("99999", "name").as_deref(), Some("Name 099999"));
    assert!(
        h.set_table_cell_text(
            id,
            target.clone(),
            info.row_token,
            info.column_token,
            "Updated"
        )
        .unwrap()
    );
    assert_eq!(t.text("99999", "name").as_deref(), Some("Updated"));
    assert!(!t.cell_info(&target).unwrap().editing);
    layout(&h);
    assert!(h.snapshot().unwrap().len() < 160);
    assert!(
        !h.sort_table_column(id, "rank", t.column_token("rank").unwrap(), false)
            .unwrap()
    );
    assert!(
        h.sort_table_column(id, "name", info.column_token, false)
            .unwrap()
    );
    t.set_rows(vec![]).unwrap();
    t.set_rows(vec![TableRow::new("99999").cell("name", "Replacement")])
        .unwrap();
    assert!(
        !h.select_table_cell(
            id,
            target.clone(),
            info.row_token,
            info.column_token,
            ListSelection::Replace
        )
        .unwrap()
    );
    assert!(
        !h.set_table_cell_text(id, target, info.row_token, info.column_token, "Stale")
            .unwrap()
    );
    assert_eq!(t.text("99999", "name").as_deref(), Some("Replacement"));
}
#[test]
fn host_actions_reject_disabled_ancestors_modal_isolation_and_unmounted_tables() {
    let t = table(3);
    let enabled = State::new(true);
    let modal = State::new(false);
    let visible = State::new(true);
    let h = Host::new({
        let t = t.clone();
        let enabled = enabled.clone();
        let modal = modal.clone();
        let visible = visible.clone();
        move || {
            if !visible.get() {
                return View::label("Replacement");
            }
            View::column(vec![
                t.view().enabled(enabled.get()),
                View::modal(vec![View::button("Modal", || {})], || {}).hidden(!modal.get()),
            ])
        }
    });
    layout(&h);
    let id = root(&h);
    let cell = TableCell::new("0", "name");
    let info = t.cell_info(&cell).unwrap();
    for suppressed in [0, 1] {
        enabled.set(suppressed != 0);
        modal.set(suppressed == 1);
        layout(&h);
        assert!(
            !h.set_table_selection(id, selection(std::slice::from_ref(&cell)))
                .unwrap()
        );
        assert!(
            !h.edit_table_cell(id, cell.clone(), info.row_token, info.column_token)
                .unwrap()
        );
        assert!(
            !h.sort_table_column(id, "name", info.column_token, false)
                .unwrap()
        );
        assert!(
            !h.select_table_cell(
                id,
                cell.clone(),
                info.row_token,
                info.column_token,
                ListSelection::Replace
            )
            .unwrap()
        );
    }
    visible.set(false);
    layout(&h);
    assert!(
        !h.set_table_cell_text(id, cell, info.row_token, info.column_token, "Stale")
            .unwrap()
    );
    assert_eq!(t.text("0", "name").as_deref(), Some("Name 000000"));
}
#[test]
fn switching_editors_rechecks_lifetimes_after_reentrant_commit_callbacks() {
    let t = table(3).on_commit_with_model(|t, _| {
        t.set_rows(vec![]).unwrap();
        t.set_rows(vec![TableRow::new("2").cell("name", "Replacement")])
            .unwrap();
    });
    let h = host(&t);
    let id = root(&h);
    let next = TableCell::new("2", "name");
    let info = t.cell_info(&next).unwrap();
    t.start_edit(TableCell::new("0", "name")).unwrap();
    t.active_edit().unwrap().set_text("Committed");
    assert!(
        !h.set_table_cell_text(id, next, info.row_token, info.column_token, "Stale")
            .unwrap()
    );
    assert!(t.active_edit().is_none());
    assert_eq!(t.text("2", "name").as_deref(), Some("Replacement"));
}
