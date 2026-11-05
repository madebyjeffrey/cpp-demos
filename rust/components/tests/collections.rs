use scars_ui::*;
use std::rc::Rc;
struct Metrics;
impl Measure for Metrics {
    fn text(&self, text: &str, _: TextStyle, _: f64) -> TextLayout {
        TextLayout {
            size: Size::new(text.len() as f64 * 8., 18.),
            baseline: 14.,
            hit_test: Rc::new(|_| 0),
            range_rects: Rc::new(|_| vec![]),
        }
    }
    fn image(&self, _: &Bitmap) -> Result<Size, Error> {
        Ok(Size::default())
    }
}
#[test]
fn large_list_mounts_only_viewport_and_reveals_offscreen_selection() {
    let model = ListModel::new(
        (0..100_000)
            .map(|i| ListItem::new(format!("id-{i}"), format!("Item {i}")))
            .collect(),
        32.,
    );
    let host = Host::new({
        let model = model.clone();
        move || model.view()
    });
    host.layout(Size::new(400., 240.), &Metrics).unwrap();
    assert!(host.snapshot().unwrap().len() < 50);
    let list = host.snapshot().unwrap()[0].id;
    assert!(host.select_collection_item(list, 99_999, false).unwrap());
    host.layout(Size::new(400., 240.), &Metrics).unwrap();
    assert!(model.selected().contains("id-99999"));
    assert!(host.snapshot().unwrap().len() < 50);
    let focused = host.focused().unwrap();
    assert_eq!(
        host.snapshot()
            .unwrap()
            .iter()
            .find(|n| n.id == focused)
            .unwrap()
            .key
            .as_deref(),
        Some("id-99999")
    );
}
#[test]
fn selection_tracks_keys_sorting_and_edit_conflicts() {
    let mut a = ListItem::new("a", "Zebra");
    a.cells.push("1".into());
    let b = ListItem::new("b", "Apple");
    let table = TableModel::new(vec!["Name".into()], vec![a, b]);
    table.rows.select("a", false);
    table.sort_by(0, false);
    assert_eq!(table.rows.index_of("a"), Some(1));
    assert!(table.rows.selected().contains("a"));
    let mut first = table.begin_edit("a", 0).unwrap();
    let mut second = first.clone();
    first.text = "First".into();
    table.commit(first).unwrap();
    second.text = "Second".into();
    assert!(table.commit(second).unwrap_err().0.contains("changed"));
    assert!(
        table
            .rows
            .set_items(vec![ListItem::new("x", "a"), ListItem::new("x", "b")])
            .is_err()
    );
    assert!(table.rows.selected().contains("a"));
}
#[test]
fn tree_only_flattens_expanded_branches() {
    let tree = TreeModel::new();
    tree.set_tree(&[TreeItem {
        key: "root".into(),
        label: "Root".into(),
        children: vec![TreeItem {
            key: "child".into(),
            label: "Child".into(),
            children: vec![],
        }],
    }])
    .unwrap();
    assert_eq!(tree.list.len(), 1);
    tree.toggle("root");
    assert_eq!(tree.list.len(), 2);
    tree.toggle("root");
    assert_eq!(tree.list.len(), 1);
}
