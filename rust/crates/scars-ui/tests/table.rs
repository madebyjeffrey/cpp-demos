use scars_ui::*;
use std::rc::Rc;
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
fn columns() -> Vec<TableColumn> {
    vec![
        TableColumn::new("name", "Name")
            .editable(true)
            .width(180.)
            .limits(80., 320.),
        TableColumn::new("team", "Team")
            .width(140.)
            .limits(60., 240.),
        TableColumn::new("rank", "Rank")
            .width(100.)
            .alignment(Align::End),
    ]
}
fn rows() -> Vec<TableRow> {
    vec![
        TableRow::new("a")
            .cell("name", "Zed")
            .cell("team", "B")
            .cell("rank", "2"),
        TableRow::new("off").cell("name", "Off").enabled(false),
        TableRow::new("b")
            .cell("name", "Ana")
            .cell("team", "A")
            .cell("rank", "2"),
        TableRow::new("c")
            .cell("name", "Ana")
            .cell("team", "A")
            .cell("rank", "1"),
    ]
}
fn model() -> TableModel {
    TableModel::with_columns(columns(), rows()).unwrap()
}
fn layout(h: &Host) {
    h.layout(Size::new(360., 320.), &Metrics).unwrap();
}
fn host(t: &TableModel) -> Host {
    let t = t.clone();
    let h = Host::new(move || {
        View::column(vec![
            t.view().grow(1.),
            View::button("After", || {}).key("after"),
        ])
    });
    layout(&h);
    h
}
fn cell(h: &Host, row: &str, column: &str) -> Node {
    h.snapshot()
        .unwrap()
        .into_iter()
        .find(|n| n.table_cell.as_ref() == Some(&TableCell::new(row, column)))
        .unwrap()
}
fn header(h: &Host, column: &str, resize: bool) -> Node {
    h.snapshot()
        .unwrap()
        .into_iter()
        .find(|n| n.table_column.as_deref() == Some(column) && n.table_resize == resize)
        .unwrap()
}
fn key(h: &Host, key: Key, modifiers: Modifiers) {
    assert!(
        h.key(KeyEvent {
            key,
            down: true,
            repeat: false,
            modifiers
        })
        .unwrap()
    );
    layout(h);
}
fn pointer(h: &Host, kind: PointerKind, x: f64, y: f64) {
    h.pointer(Pointer {
        kind,
        position: Point { x, y },
        button: 0,
        modifiers: Default::default(),
    })
    .unwrap();
    layout(h);
}
#[test]
fn checked_columns_keep_data_and_identity_through_widths_order_and_invalid_replacement() {
    let t = model();
    let token = t.column_token("name");
    assert_eq!(t.text("a", "name").as_deref(), Some("Zed"));
    assert!(t.resize_column("name", 900.).unwrap());
    assert_eq!(t.columns()[0].width, 320.);
    assert!(t.resize_column("name", 10.).unwrap());
    assert_eq!(t.columns()[0].width, 80.);
    let before = t.columns();
    assert!(
        t.set_columns(vec![TableColumn::new("x", "X"), TableColumn::new("x", "Y")])
            .is_err()
    );
    assert_eq!(t.columns(), before);
    assert!(
        t.set_widths(&[
            TableColumnWidth {
                column: "team".into(),
                width: 190.
            },
            TableColumnWidth {
                column: "missing".into(),
                width: 10.
            }
        ])
        .is_err()
    );
    assert_eq!(t.columns(), before);
    assert!(t.move_column("name", 2).unwrap());
    assert_eq!(
        t.columns()
            .iter()
            .map(|c| c.id.as_str())
            .collect::<Vec<_>>(),
        ["team", "rank", "name"]
    );
    assert_eq!(t.column_token("name"), token);
    assert_eq!(t.text("a", "name").as_deref(), Some("Zed"));
    assert!(
        t.set_rows(vec![TableRow::new("a"), TableRow::new("a")])
            .is_err()
    );
    assert_eq!(t.row_count(), 4);
    let mut cols = t.columns();
    cols[0].resizable = false;
    t.set_columns(cols).unwrap();
    assert!(!t.resize_column("team", 200.).unwrap());
    assert!(t.resize_column("name", f64::NAN).is_err());
    assert!(
        TableModel::with_options(
            columns(),
            rows(),
            TableOptions {
                row_height: 0.,
                ..Default::default()
            }
        )
        .is_err()
    );
}
#[test]
fn stable_multi_sort_clear_and_callbacks_preserve_logical_selection() {
    let events = State::new(vec![]);
    let t = model().on_sort(events.handler(|s, e| s.update(|v| v.push(e))));
    t.select_row("a", ListSelection::Replace);
    t.set_sort(vec![
        SortDescriptor::new("name", SortOrder::Ascending),
        SortDescriptor::new("rank", SortOrder::Ascending),
    ])
    .unwrap();
    assert_eq!(t.row(0).unwrap().key, "c");
    assert_eq!(t.row(1).unwrap().key, "b");
    assert_eq!(t.selection().rows, ["a".to_owned()].into());
    let old = t.sort();
    assert!(
        t.set_sort(vec![SortDescriptor::new("absent", SortOrder::Ascending)])
            .is_err()
    );
    assert_eq!(t.sort(), old);
    t.set_sort(vec![]).unwrap();
    assert_eq!(t.row(0).unwrap().key, "a");
    assert_eq!(events.get().len(), 2);
    t.toggle_sort("team", false).unwrap();
    t.toggle_sort("rank", true).unwrap();
    assert_eq!(t.sort().len(), 2);
    t.toggle_sort("rank", true).unwrap();
    assert_eq!(t.sort()[1].order, SortOrder::Descending);
    t.toggle_sort("rank", true).unwrap();
    assert_eq!(t.sort().len(), 1);
}
#[test]
fn rectangular_cell_selection_skips_disabled_rows_and_follows_keys_after_changes() {
    let t = model();
    t.set_selection_mode(TableSelectionMode::Cells, true);
    assert!(t.select_cell(TableCell::new("a", "name"), ListSelection::Replace));
    assert!(t.select_cell(TableCell::new("c", "rank"), ListSelection::Range));
    assert_eq!(t.selection_count(), 9);
    assert!(!t.selection().cells.iter().any(|c| c.row == "off"));
    t.move_column("rank", 1).unwrap();
    t.set_sort(vec![SortDescriptor::new("name", SortOrder::Ascending)])
        .unwrap();
    assert_eq!(t.selection_count(), 9);
    assert_eq!(t.cursor(), Some(TableCell::new("c", "rank")));
    t.set_rows(vec![rows().remove(0)]).unwrap();
    assert_eq!(t.selection_count(), 3);
    assert_eq!(t.cursor(), Some(TableCell::new("a", "rank")));
    t.set_columns(vec![TableColumn::new("name", "Name")])
        .unwrap();
    assert_eq!(t.selection_count(), 1);
    assert_eq!(t.cursor(), Some(TableCell::new("a", "name")));
    t.set_selection_mode(TableSelectionMode::Rows, true);
    assert!(t.selection().cells.is_empty());
    t.select_row("a", ListSelection::Replace);
    assert_eq!(t.selection_count(), 1);
    t.set_rows(vec![]).unwrap();
    assert_eq!(t.cursor(), None);
    assert_eq!(t.selection_count(), 0);
}
#[test]
fn headers_and_cells_align_scroll_horizontally_and_keyboard_roves_one_cell_tab_stop() {
    let t = model();
    t.set_selection_mode(TableSelectionMode::Cells, true);
    let h = host(&t);
    let name = cell(&h, "a", "name");
    let heading = header(&h, "name", false);
    assert_eq!(heading.bounds.x, name.bounds.x);
    assert_eq!(name.bounds.width, 180.);
    assert_eq!(cell(&h, "a", "team").bounds.width, 140.);
    assert!(name.bounds.height >= 30.);
    h.focus(name.id).unwrap();
    key(&h, Key::Right, Default::default());
    assert_eq!(t.cursor(), Some(TableCell::new("a", "team")));
    assert_eq!(h.focused(), Some(cell(&h, "a", "team").id));
    key(
        &h,
        Key::Down,
        Modifiers {
            shift: true,
            ..Default::default()
        },
    );
    assert_eq!(t.cursor(), Some(TableCell::new("b", "team")));
    assert_eq!(t.selection_count(), 2);
    key(&h, Key::Right, Default::default());
    let rank = cell(&h, "b", "rank");
    assert_eq!(h.focused(), Some(rank.id));
    assert!(rank.clip.width > 0.);
    assert!(header(&h, "rank", false).clip.width > 0.);
    key(&h, Key::Tab, Default::default());
    assert_eq!(
        h.snapshot()
            .unwrap()
            .iter()
            .find(|n| Some(n.id) == h.focused())
            .unwrap()
            .key
            .as_deref(),
        Some("after")
    );
}
#[test]
fn resizing_drag_cancel_and_keyboard_reorder_sort_are_checked() {
    let t = model();
    let h = host(&t);
    let resize = header(&h, "name", true);
    let p = Point {
        x: resize.bounds.x + 4.,
        y: resize.bounds.y + 12.,
    };
    assert_eq!(
        h.pointer_cursor(p).unwrap(),
        PointerCursor::ResizeHorizontal
    );
    pointer(&h, PointerKind::Down, p.x, p.y);
    pointer(&h, PointerKind::Move, p.x + 60., p.y);
    assert_eq!(t.columns()[0].width, 240.);
    key(&h, Key::Escape, Default::default());
    assert_eq!(t.columns()[0].width, 180.);
    pointer(&h, PointerKind::Down, p.x, p.y);
    pointer(&h, PointerKind::Move, p.x - 300., p.y);
    assert_eq!(t.columns()[0].width, 80.);
    pointer(&h, PointerKind::Up, p.x - 300., p.y);
    let resize = header(&h, "name", true);
    h.focus(resize.id).unwrap();
    key(&h, Key::Right, Default::default());
    assert_eq!(t.columns()[0].width, 88.);
    let head = header(&h, "name", false);
    h.focus(head.id).unwrap();
    key(
        &h,
        Key::Right,
        Modifiers {
            alt: true,
            ..Default::default()
        },
    );
    assert_eq!(t.columns()[1].id, "name");
    key(&h, Key::Enter, Default::default());
    assert_eq!(
        t.sort()[0],
        SortDescriptor::new("name", SortOrder::Ascending)
    );
    let head = header(&h, "name", false);
    let target = header(&h, "team", false);
    pointer(
        &h,
        PointerKind::Down,
        head.bounds.x + 20.,
        head.bounds.y + 10.,
    );
    pointer(
        &h,
        PointerKind::Move,
        target.bounds.x + 20.,
        target.bounds.y + 10.,
    );
    pointer(
        &h,
        PointerKind::Up,
        target.bounds.x + 20.,
        target.bounds.y + 10.,
    );
    assert_eq!(t.columns()[0].id, "name");
}
#[test]
fn focused_cell_and_drafts_survive_order_but_removed_lifetimes_retire_even_with_same_text() {
    let t = model();
    t.set_selection_mode(TableSelectionMode::Cells, false);
    let h = host(&t);
    let original = cell(&h, "a", "name");
    h.focus(original.id).unwrap();
    let edit = t.edit_cell(&TableCell::new("a", "name")).unwrap();
    t.move_column("name", 2).unwrap();
    layout(&h);
    assert_eq!(cell(&h, "a", "name").id, original.id);
    assert_eq!(h.focused(), Some(original.id));
    let mut edit = edit;
    edit.text = "New".into();
    t.commit(edit).unwrap();
    assert_eq!(t.text("a", "name").as_deref(), Some("New"));
    let old = t.edit_cell(&TableCell::new("a", "name")).unwrap();
    let token = t.column_token("name");
    t.set_columns(vec![TableColumn::new("team", "Team")])
        .unwrap();
    t.set_columns(columns()).unwrap();
    layout(&h);
    assert_ne!(t.column_token("name"), token);
    assert_ne!(cell(&h, "a", "name").id, original.id);
    assert!(t.commit(old).unwrap_err().to_string().contains("removed"));
    assert_eq!(
        h.snapshot()
            .unwrap()
            .iter()
            .find(|n| Some(n.id) == h.focused())
            .unwrap()
            .table_cell,
        t.cursor()
    );
}
#[test]
fn large_table_keeps_realization_bounded_and_selection_through_sort_and_resize() {
    let t = TableModel::with_columns(
        columns(),
        (0..100_000)
            .map(|i| TableRow::new(i.to_string()).cell("name", format!("Row {i:06}")))
            .collect(),
    )
    .unwrap();
    t.set_selection_mode(TableSelectionMode::Cells, true);
    let h = host(&t);
    h.focus(cell(&h, "0", "name").id).unwrap();
    key(&h, Key::End, Default::default());
    assert_eq!(t.cursor(), Some(TableCell::new("99999", "name")));
    assert!(h.snapshot().unwrap().len() < 180);
    let id = cell(&h, "99999", "name").id;
    t.resize_column("name", 250.).unwrap();
    layout(&h);
    assert_eq!(cell(&h, "99999", "name").id, id);
    assert!(h.snapshot().unwrap().len() < 180);
    t.set_sort(vec![SortDescriptor::new("name", SortOrder::Descending)])
        .unwrap();
    layout(&h);
    assert_eq!(
        t.selection().cells,
        [TableCell::new("99999", "name")].into()
    );
    assert_eq!(h.focused(), Some(cell(&h, "99999", "name").id));
    assert!(h.snapshot().unwrap().len() < 180);
}

#[test]
fn mode_changes_and_equal_text_column_replacement_keep_logical_focus_and_cancel_stale_capture() {
    let t = TableModel::with_columns(
        vec![
            TableColumn::new("one", "One"),
            TableColumn::new("two", "Two"),
        ],
        vec![TableRow::new("r")],
    )
    .unwrap();
    let h = host(&t);
    let list = h
        .snapshot()
        .unwrap()
        .iter()
        .find(|n| n.table_model.is_some())
        .unwrap()
        .id;
    h.select_collection_item(list, 0, false).unwrap();
    layout(&h);
    assert_eq!(
        h.snapshot()
            .unwrap()
            .iter()
            .find(|n| Some(n.id) == h.focused())
            .unwrap()
            .kind,
        Kind::CollectionRow
    );
    t.set_selection_mode(TableSelectionMode::Cells, true);
    layout(&h);
    assert_eq!(
        h.snapshot()
            .unwrap()
            .iter()
            .find(|n| Some(n.id) == h.focused())
            .unwrap()
            .table_cell,
        Some(TableCell::new("r", "one"))
    );
    t.move_column("one", 1).unwrap();
    layout(&h);
    assert_eq!(h.focused(), Some(cell(&h, "r", "one").id));
    t.set_columns(vec![
        TableColumn::new("new", "New"),
        TableColumn::new("two", "Two"),
    ])
    .unwrap();
    layout(&h);
    assert_eq!(h.focused(), Some(cell(&h, "r", "new").id));
    t.set_selection_mode(TableSelectionMode::Rows, true);
    layout(&h);
    assert_eq!(
        h.snapshot()
            .unwrap()
            .iter()
            .find(|n| Some(n.id) == h.focused())
            .unwrap()
            .kind,
        Kind::CollectionRow
    );
    let r = header(&h, "new", true);
    let p = Point {
        x: r.bounds.x + 4.,
        y: r.bounds.y + 10.,
    };
    pointer(&h, PointerKind::Down, p.x, p.y);
    pointer(&h, PointerKind::Move, p.x + 20., p.y);
    t.resize_column("new", 220.).unwrap();
    layout(&h);
    pointer(&h, PointerKind::Cancel, p.x, p.y);
    assert_eq!(t.columns()[0].width, 220.);
    let r = header(&h, "new", true);
    pointer(&h, PointerKind::Down, r.bounds.x + 4., r.bounds.y + 10.);
    t.set_columns(vec![TableColumn::new("two", "Two")]).unwrap();
    layout(&h);
    pointer(&h, PointerKind::Move, 300., 10.);
    assert_eq!(t.columns().len(), 1);
}
