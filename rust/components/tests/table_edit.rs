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
fn columns() -> Vec<TableColumn> {
    vec![
        TableColumn::new("name", "Name")
            .editable(true)
            .validate(|_, s| {
                if s.trim().is_empty() {
                    Err(Error("A name is required".into()))
                } else {
                    Ok(())
                }
            })
            .width(200.),
        TableColumn::new("state", "Status").content(|_, s| View::label(format!("● {s}"))),
    ]
}
fn rows(n: usize) -> Vec<TableRow> {
    (0..n)
        .map(|i| {
            TableRow::new(i.to_string())
                .cell("name", format!("Name {i:06}"))
                .cell("state", "Ready")
        })
        .collect()
}
fn model(n: usize) -> TableModel {
    TableModel::with_options(
        columns(),
        rows(n),
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
    let h = Host::new(move || {
        View::column(vec![
            t.view().grow(1.),
            View::button("After", || {}).key("after"),
        ])
    });
    layout(&h);
    h
}
fn layout(h: &Host) {
    h.layout(Size::new(420., 300.), &Metrics).unwrap();
}
fn node(h: &Host, row: &str, col: &str) -> Node {
    h.snapshot()
        .unwrap()
        .into_iter()
        .find(|n| n.table_cell == Some(TableCell::new(row, col)))
        .unwrap()
}
fn editor(h: &Host) -> Node {
    h.snapshot()
        .unwrap()
        .into_iter()
        .find(|n| n.table_editor.is_some())
        .unwrap()
}
fn key(h: &Host, key: Key, modifiers: Modifiers) {
    assert!(
        h.key(KeyEvent {
            key,
            modifiers,
            down: true,
            repeat: false
        })
        .unwrap()
    );
    layout(h);
}
fn begin(t: &TableModel, h: &Host, row: &str) -> TableEdit {
    h.focus(node(h, row, "name").id).unwrap();
    key(h, Key::Enter, Default::default());
    t.active_edit().unwrap()
}
#[test]
fn keyboard_edit_validation_commit_cancel_and_readonly_custom_content() {
    let commits = State::new(vec![]);
    let t = model(5).on_commit(commits.handler(|s, e| s.update(|v| v.push(e))));
    let h = host(&t);
    assert!(
        h.snapshot()
            .unwrap()
            .iter()
            .any(|n| n.text == "● Ready" && n.clip.width > 0. && n.clip.height > 0.)
    );
    assert!(t.start_edit(TableCell::new("0", "state")).is_err());
    let edit = begin(&t, &h, "0");
    let field = editor(&h).id;
    assert_eq!(h.focused(), Some(field));
    assert_eq!(h.text_input().unwrap().unwrap().text, "Name 000000");
    h.set_text(field, "").unwrap();
    key(&h, Key::Enter, Default::default());
    assert!(edit.is_current());
    assert!(t.edit_error().unwrap().contains("required"));
    assert!(
        h.snapshot()
            .unwrap()
            .iter()
            .any(|n| n.text == "A name is required")
    );
    assert_eq!(t.text("0", "name").as_deref(), Some("Name 000000"));
    let target = node(&h, "1", "name");
    for kind in [PointerKind::Down, PointerKind::Up] {
        h.pointer_click(
            Pointer {
                kind,
                position: Point {
                    x: target.bounds.x + 30.,
                    y: target.bounds.y + 12.,
                },
                button: 0,
                modifiers: Default::default(),
            },
            2,
        )
        .unwrap();
        layout(&h);
    }
    layout(&h);
    assert!(edit.is_current());
    assert_eq!(h.focused(), Some(field));
    assert_eq!(t.selection().cells, [TableCell::new("0", "name")].into());
    h.set_text(field, "Saved").unwrap();
    key(&h, Key::Enter, Default::default());
    assert!(!edit.is_current());
    assert_eq!(t.text("0", "name").as_deref(), Some("Saved"));
    assert_eq!(
        commits.get(),
        [TableCommit {
            cell: TableCell::new("0", "name"),
            text: "Saved".into()
        }]
    );
    assert_eq!(h.focused(), Some(node(&h, "0", "name").id));
    assert!(!edit.commit().unwrap());
    let next = begin(&t, &h, "0");
    assert_ne!(editor(&h).id, field);
    h.set_text(editor(&h).id, "Unsaved").unwrap();
    key(&h, Key::Escape, Default::default());
    assert!(!next.is_current());
    assert_eq!(t.text("0", "name").as_deref(), Some("Saved"));
    assert_eq!(commits.get().len(), 1);
}
#[test]
fn value_versions_reject_aba_and_noop_duplicate_commits_but_ignore_other_cells() {
    let t = model(2);
    let draft = t.edit_cell(&TableCell::new("0", "name")).unwrap();
    let mut changed = rows(2);
    changed[0].cells.insert("name".into(), "Other".into());
    t.set_rows(changed).unwrap();
    t.set_rows(rows(2)).unwrap();
    assert!(t.commit(draft).unwrap_err().to_string().contains("changed"));
    let draft = t.edit_cell(&TableCell::new("0", "name")).unwrap();
    let mut changed = rows(2);
    changed[0].cells.insert("state".into(), "Busy".into());
    t.set_rows(changed).unwrap();
    t.commit(draft.clone()).unwrap();
    assert!(t.commit(draft).is_err());
    let h = host(&t);
    let edit = begin(&t, &h, "0");
    edit.set_text("My draft");
    let mut changed = rows(2);
    changed[0].cells.insert("name".into(), "Remote".into());
    t.set_rows(changed).unwrap();
    layout(&h);
    assert_eq!(edit.info().unwrap().text, "My draft");
    assert!(edit.info().unwrap().error.unwrap().contains("changed"));
    assert!(edit.commit().is_err());
    assert_eq!(t.text("0", "name").as_deref(), Some("Remote"));
    assert!(edit.cancel());
}
#[test]
fn pinned_editor_keeps_selection_history_and_composition_through_scroll_sort_and_columns() {
    let t = model(100_000);
    let h = host(&t);
    let edit = begin(&t, &h, "0");
    let field = editor(&h).id;
    h.set_text(field, "Changed").unwrap();
    h.select_text(
        field,
        Selection {
            anchor: 2,
            caret: 4,
        },
    )
    .unwrap();
    h.mark_text("日本", 6..6, None).unwrap();
    layout(&h);
    let before = h.text_input().unwrap().unwrap();
    let root = h
        .snapshot()
        .unwrap()
        .iter()
        .find(|n| n.table_model.is_some())
        .unwrap()
        .id;
    h.scroll_to(root, Point { x: 0., y: 380_000. }).unwrap();
    layout(&h);
    let after = h.text_input().unwrap().unwrap();
    assert_eq!(after.id, field);
    assert_eq!(after.marked, before.marked);
    assert_eq!(after.selection, before.selection);
    assert_eq!(editor(&h).clip.height, 0.);
    assert!(h.snapshot().unwrap().len() < 180);
    t.move_column("name", 1).unwrap();
    t.set_sort(vec![SortDescriptor::new("name", SortOrder::Descending)])
        .unwrap();
    layout(&h);
    assert_eq!(editor(&h).id, field);
    assert_eq!(h.text_input().unwrap().unwrap().marked, before.marked);
    assert!(edit.is_current());
    h.unmark_text(false).unwrap();
    h.text_command(TextCommand::Undo, false).unwrap();
    assert_eq!(edit.info().unwrap().text, "Name 000000");
    assert!(h.text_input().unwrap().unwrap().can_redo);
    for i in 0..20 {
        h.scroll_to(
            root,
            Point {
                x: 0.,
                y: (i * 150_000) as f64,
            },
        )
        .unwrap();
        layout(&h);
        assert_eq!(editor(&h).id, field);
        assert!(h.snapshot().unwrap().len() < 180);
    }
    assert!(edit.cancel());
    layout(&h);
    assert!(
        !h.snapshot()
            .unwrap()
            .iter()
            .any(|n| n.table_editor.is_some())
    );
}
#[test]
fn retired_row_column_policy_and_mount_lifetimes_cannot_commit_a_new_cell() {
    let t = model(2);
    let h = host(&t);
    let old = begin(&t, &h, "0");
    let old_id = editor(&h).id;
    t.set_rows(vec![rows(2).remove(1)]).unwrap();
    t.set_rows(rows(2)).unwrap();
    layout(&h);
    assert!(!old.is_current());
    assert!(!old.commit().unwrap());
    assert!(!h.set_text(old_id, "stale").unwrap());
    let edit = t.start_edit(TableCell::new("0", "name")).unwrap();
    let mut cols = t.columns();
    cols[0] = cols[0].clone().editable(false);
    t.set_columns(cols).unwrap();
    layout(&h);
    assert!(!edit.is_current());
    assert!(!edit.commit().unwrap());
    t.set_columns(columns()).unwrap();
    let edit = t.start_edit(TableCell::new("0", "name")).unwrap();
    t.set_columns(vec![TableColumn::new("state", "Status")])
        .unwrap();
    t.set_columns(columns()).unwrap();
    layout(&h);
    assert!(!edit.is_current());
    let edit = t.start_edit(TableCell::new("0", "name")).unwrap();
    let mut disabled = rows(2);
    disabled[0].enabled = false;
    t.set_rows(disabled).unwrap();
    layout(&h);
    assert!(!edit.is_current());
    assert!(!edit.commit().unwrap());
    t.set_rows(rows(2)).unwrap();
    let edit = t.start_edit(TableCell::new("0", "name")).unwrap();
    layout(&h);
    drop(h);
    assert!(!edit.is_current());
    assert!(t.active_edit().is_none());
    let replacement = model(2);
    let chosen = State::new(t.clone());
    let h = Host::new({
        let chosen = chosen.clone();
        move || chosen.get().view()
    });
    let old = t.start_edit(TableCell::new("0", "name")).unwrap();
    layout(&h);
    let id = editor(&h).id;
    chosen.set(replacement.clone());
    layout(&h);
    assert!(!old.is_current());
    assert!(!h.set_text(id, "stale").unwrap());
    assert_eq!(
        replacement.text("0", "name").as_deref(),
        Some("Name 000000")
    );
}
#[test]
fn commit_callback_can_start_the_next_pinned_edit_and_unmount_releases_it() {
    let t = model(3).on_commit_with_model(|t, _| {
        t.start_edit(TableCell::new("1", "name")).unwrap();
    });
    let h = host(&t);
    let edit = begin(&t, &h, "0");
    edit.set_text("Done");
    assert!(edit.commit().unwrap());
    layout(&h);
    assert_eq!(
        t.active_edit().unwrap().info().unwrap().cell,
        TableCell::new("1", "name")
    );
    assert_eq!(h.focused(), Some(editor(&h).id));
    let visible = State::new(true);
    drop(h);
    let h = Host::new({
        let t = t.clone();
        let visible = visible.clone();
        move || {
            if visible.get() {
                t.view()
            } else {
                View::label("Removed")
            }
        }
    });
    let edit = t.start_edit(TableCell::new("0", "name")).unwrap();
    layout(&h);
    visible.set(false);
    layout(&h);
    assert!(!edit.is_current());
    assert!(t.active_edit().is_none());
}
#[test]
fn multiline_tab_marked_text_and_grapheme_length_follow_editor_policy() {
    let t = TableModel::with_columns(
        vec![
            TableColumn::new("name", "Notes")
                .editable(true)
                .multiline(true)
                .max_length(12),
        ],
        rows(1),
    )
    .unwrap();
    let h = host(&t);
    let edit = begin(&t, &h, "0");
    let id = editor(&h).id;
    h.set_text(id, "One").unwrap();
    key(&h, Key::Enter, Default::default());
    assert_eq!(edit.info().unwrap().text, "One\n");
    assert!(edit.is_current());
    h.mark_text("日", 3..3, None).unwrap();
    key(
        &h,
        Key::Enter,
        Modifiers {
            command: true,
            ..Default::default()
        },
    );
    assert!(edit.is_current());
    assert!(h.text_input().unwrap().unwrap().marked.is_none());
    key(
        &h,
        Key::Enter,
        Modifiers {
            command: true,
            ..Default::default()
        },
    );
    assert!(!edit.is_current());
    let edit = begin(&t, &h, "0");
    h.set_text(editor(&h).id, "Draft").unwrap();
    key(&h, Key::Tab, Default::default());
    assert!(!edit.is_current());
    assert_eq!(t.text("0", "name").as_deref(), Some("Draft"));
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
    let mut cols = t.columns();
    cols[0] = cols[0].clone().max_length(2);
    t.set_columns(cols).unwrap();
    let edit = t.start_edit(TableCell::new("0", "name")).unwrap();
    layout(&h);
    h.set_text(editor(&h).id, "👨‍👩‍👧‍👦éX").unwrap();
    assert_eq!(edit.info().unwrap().text, "👨‍👩‍👧‍👦é");
    edit.set_text("too long");
    assert!(edit.commit().is_err());
}
#[test]
fn validation_is_pure_and_double_click_activates_without_changing_readonly_content() {
    let changed = State::new(false);
    let commits = Rc::new(Cell::new(0));
    let t = TableModel::with_columns(
        vec![TableColumn::new("name", "Name").editable(true).validate({
            let changed = changed.clone();
            move |_, _| {
                changed.set(true);
                Ok(())
            }
        })],
        rows(2),
    )
    .unwrap()
    .on_commit({
        let n = commits.clone();
        move |_| n.set(n.get() + 1)
    });
    let h = host(&t);
    let cell = node(&h, "0", "name");
    for kind in [PointerKind::Down, PointerKind::Up] {
        h.pointer_click(
            Pointer {
                kind,
                position: Point {
                    x: cell.bounds.x + 30.,
                    y: cell.bounds.y + 12.,
                },
                button: 0,
                modifiers: Default::default(),
            },
            2,
        )
        .unwrap();
        layout(&h);
    }
    layout(&h);
    let edit = t.active_edit().unwrap();
    edit.set_text("X");
    assert!(edit.commit().unwrap_err().to_string().contains("validator"));
    assert!(!changed.get());
    assert_eq!(commits.get(), 0);
}
