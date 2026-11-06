use scars_ui::*;
#[test]
fn list_versions_distinguish_same_size_selection_content_and_focus() {
    let list =
        ListModel::new(vec![ListItem::new("a", "A"), ListItem::new("b", "B")], 32.).multiple();
    let original = list.version();
    list.set_selection(&["a".into()]).unwrap();
    let a = list.version();
    assert_ne!(a.selection, original.selection);
    assert_eq!(a.content, original.content);
    list.set_selection(&["b".into()]).unwrap();
    let b = list.version();
    assert_ne!(a.selection, b.selection);
    assert_eq!(list.selection_count(), 1);
    list.select_with("a", ListSelection::Focus);
    assert_eq!(b, list.version());
    list.set_selection(&["b".into()]).unwrap();
    assert_eq!(b, list.version());
    list.set_items(vec![ListItem::new("a", "Changed"), ListItem::new("b", "B")])
        .unwrap();
    assert_ne!(b.content, list.version().content);
    assert_eq!(b.identity, list.version().identity);
    let replacement = ListModel::new(vec![], 32.);
    assert_ne!(list.version().identity, replacement.version().identity);
}
#[test]
fn table_selection_versions_ignore_cursor_only_changes_and_track_mode_data() {
    let table = TableModel::with_options(
        vec![
            TableColumn::new("name", "Name"),
            TableColumn::new("rank", "Rank"),
        ],
        vec![
            TableRow::new("a").cell("name", "A"),
            TableRow::new("b").cell("name", "B"),
        ],
        TableOptions {
            selection_mode: TableSelectionMode::Cells,
            multiple_selection: true,
            ..Default::default()
        },
    )
    .unwrap();
    table.select_cell(TableCell::new("a", "name"), ListSelection::Replace);
    let before = table.version();
    table.select_cell(TableCell::new("b", "rank"), ListSelection::Focus);
    assert_eq!(before, table.version());
    table.select_cell(TableCell::new("b", "rank"), ListSelection::Replace);
    assert_ne!(before.selection, table.version().selection);
    assert_eq!(before.content, table.version().content);
    let before = table.version();
    table.move_column("rank", 0).unwrap();
    assert_ne!(before.content, table.version().content);
    assert_eq!(before.selection, table.version().selection);
    let before = table.version();
    table.set_text("a", "name", "Changed").unwrap();
    assert_ne!(before.content, table.version().content);
}
#[test]
fn tree_metadata_changes_advance_content_without_realizing_rows() {
    let tree = TreeModel::from_nodes(vec![TreeNode::new("remote", "Remote").deferred()]).unwrap();
    let before = tree.version();
    tree.expand("remote").unwrap();
    assert_eq!(tree.visible_count(), 1);
    assert_ne!(before.content, tree.version().content);
    assert_eq!(before.selection, tree.version().selection);
    let before = tree.version();
    tree.pending_load("remote")
        .unwrap()
        .complete(Ok(vec![TreeNode::new("child", "Child")]))
        .unwrap();
    assert_ne!(before.content, tree.version().content);
    assert_eq!(tree.visible_count(), 2);
}
