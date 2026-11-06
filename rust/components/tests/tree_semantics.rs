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
    h.layout(Size::new(500., 260.), &Metrics).unwrap();
}
fn root(h: &Host) -> NodeId {
    h.snapshot()
        .unwrap()
        .iter()
        .find(|n| n.tree_model.is_some())
        .unwrap()
        .id
}
fn fixture() -> TreeModel {
    let t = TreeModel::from_nodes(vec![
        TreeNode::new("root", "Root")
            .accessible_label("Project workspace")
            .children(vec![
                TreeNode::new("branch", "Branch").children(vec![TreeNode::new("leaf", "Leaf")]),
                TreeNode::new("off", "Disabled").enabled(false),
                TreeNode::new("other", "Other"),
            ]),
    ])
    .unwrap()
    .multiple();
    t.expand("root").unwrap();
    t.expand("branch").unwrap();
    t
}
#[test]
fn logical_rows_express_flat_outline_order_and_direct_disclosure_links() {
    let t = fixture();
    let rows = t.rows(0, usize::MAX);
    assert_eq!(
        rows.iter().map(|r| r.item.key.as_str()).collect::<Vec<_>>(),
        ["root", "branch", "leaf", "off", "other"]
    );
    assert_eq!(rows[0].item.accessible_label, "Project workspace");
    assert_eq!(rows[2].item.parent.as_deref(), Some("branch"));
    assert_eq!(rows[2].item.depth, 2);
    assert_eq!(rows[2].index, 2);
    assert!(!rows[2].activatable);
    assert!(!rows[3].item.enabled);
    assert_eq!(t.disclosed_count("root"), 3);
    assert_eq!(t.disclosed_rows("root", 1, 1)[0].item.key, "off");
    assert_eq!(t.disclosed_rows("branch", 0, 9)[0].item.key, "leaf");
    assert!(t.rows(usize::MAX, 10).is_empty());
    let descendants = t.disclosed_snapshot("root").unwrap();
    t.collapse("root").unwrap();
    assert!(t.row("leaf").is_none());
    assert!(t.item("leaf").is_some());
    assert!(t.disclosed_snapshot("root").is_none());
    assert_eq!(descendants.len(), 3);
    assert_eq!(descendants.key(0), Some("branch"));
    assert!(descendants.token(0).is_some());
}
#[test]
fn shared_snapshots_preserve_order_and_tokens_through_reorder_and_reinsertion() {
    let list = ListModel::new(vec![ListItem::new("a", "A"), ListItem::new("b", "B")], 32.);
    let before = list.snapshot();
    let token = before.item_token("a");
    list.set_items(vec![ListItem::new("b", "B"), ListItem::new("a", "Updated")])
        .unwrap();
    assert_eq!(before.item(0).unwrap().key, "a");
    assert_eq!(before.item(0).unwrap().label(), "A");
    assert_eq!(list.snapshot().item(0).unwrap().key, "b");
    assert_eq!(list.item_token("a"), token);
    list.set_items(vec![]).unwrap();
    list.set_items(vec![ListItem::new("a", "A")]).unwrap();
    assert_ne!(list.item_token("a"), token);
    assert_eq!(before.item_token("a"), token);
}
#[test]
fn selection_is_atomic_respects_visibility_and_does_not_steal_outside_focus() {
    let t = fixture();
    let h = Host::new({
        let t = t.clone();
        move || View::column(vec![t.view(), View::button("After", || {}).key("after")])
    });
    layout(&h);
    let id = root(&h);
    let after = h
        .snapshot()
        .unwrap()
        .iter()
        .find(|n| n.key.as_deref() == Some("after"))
        .unwrap()
        .id;
    h.focus(after).unwrap();
    h.set_tree_selection(id, &["leaf".into(), "other".into()])
        .unwrap();
    layout(&h);
    assert_eq!(h.focused(), Some(after));
    assert_eq!(t.selection_count(), 2);
    let before = t.selected();
    assert!(
        h.set_tree_selection(id, &["root".into(), "off".into()])
            .is_err()
    );
    assert_eq!(t.selected(), before);
    t.collapse("root").unwrap();
    assert!(t.set_selection(&["leaf".into()]).is_err());
    assert_eq!(t.selected(), ["root".to_owned()].into());
    let scoped = Host::new({
        let t = t.clone();
        move || {
            View::layer(vec![
                t.view(),
                View::modal(vec![View::button("Modal", || {})], || {}),
            ])
        }
    });
    layout(&scoped);
    let token = t.item_token("root").unwrap();
    assert!(
        !scoped
            .set_tree_disclosed(root(&scoped), "root", token, true)
            .unwrap()
    );
    assert!(!t.row("root").unwrap().item.expanded);
    let single =
        TreeModel::from_nodes(vec![TreeNode::new("a", "A"), TreeNode::new("b", "B")]).unwrap();
    assert!(single.set_selection(&["a".into(), "b".into()]).is_err());
    assert!(single.selected().is_empty());
}
#[test]
fn offscreen_disclosure_loading_failure_and_stale_actions_use_logical_lifetimes() {
    let source = State::new(false);
    let t = TreeModel::from_nodes(
        (0..100_000)
            .map(|i| {
                if i == 99_999 {
                    TreeNode::new(i.to_string(), "Remote").lazy({
                        let source = source.clone();
                        move || {
                            if source.get() {
                                Ok(vec![TreeNode::new("fetched", "Fetched")])
                            } else {
                                Err(Error("Offline".into()))
                            }
                        }
                    })
                } else {
                    TreeNode::new(i.to_string(), "Row")
                }
            })
            .collect(),
    )
    .unwrap();
    let h = Host::new({
        let t = t.clone();
        move || t.view()
    });
    layout(&h);
    let id = root(&h);
    let row = t.row("99999").unwrap();
    assert!(
        !h.snapshot()
            .unwrap()
            .iter()
            .any(|n| n.tree_item.as_ref().is_some_and(|i| i.key == "99999"))
    );
    assert!(h.set_tree_disclosed(id, "99999", row.token, true).unwrap());
    assert_eq!(
        t.row("99999").unwrap().item.error.as_deref(),
        Some("Offline")
    );
    source.set(true);
    assert!(h.activate_tree_item(id, "99999", row.token).unwrap());
    assert_eq!(t.disclosed_count("99999"), 1);
    layout(&h);
    assert!(h.snapshot().unwrap().len() < 100);
    t.set_nodes(vec![TreeNode::new("99999", "New").deferred()])
        .unwrap();
    assert!(!h.set_tree_disclosed(id, "99999", row.token, true).unwrap());
    let fresh = t.row("99999").unwrap();
    assert_ne!(fresh.token, row.token);
    h.set_tree_disclosed(id, "99999", fresh.token, true)
        .unwrap();
    assert!(t.row("99999").unwrap().item.loading);
    let load = t.pending_load("99999").unwrap();
    load.complete(Ok(vec![])).unwrap();
    assert!(!t.row("99999").unwrap().item.loading);
}
#[test]
fn immutable_child_membership_does_not_keep_loader_resources_alive() {
    struct Resource(Rc<Cell<usize>>);
    impl Drop for Resource {
        fn drop(&mut self) {
            self.0.set(self.0.get() + 1)
        }
    }
    let drops = Rc::new(Cell::new(0));
    let resource = Resource(drops.clone());
    let t = TreeModel::from_nodes(vec![TreeNode::new("root", "Root").children(vec![
        TreeNode::new("lazy", "Lazy").lazy(move || {
            let _ = &resource;
            Ok(vec![])
        }),
    ])])
    .unwrap();
    t.expand("root").unwrap();
    let children = t.disclosed_snapshot("root").unwrap();
    let rows = t.row_snapshot();
    t.set_nodes(vec![]).unwrap();
    assert_eq!(drops.get(), 1);
    assert_eq!(children.key(0), Some("lazy"));
    assert_eq!(children.len(), 1);
    assert_eq!(rows.len(), 2);
    assert_eq!(t.visible_count(), 0);
}
