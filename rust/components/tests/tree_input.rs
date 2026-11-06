use scars_ui::*;
use std::{cell::Cell, rc::Rc};
struct Metrics;
impl Measure for Metrics {
    fn text(&self, text: &str, _: TextStyle, _: f64) -> TextLayout {
        TextLayout {
            size: Size::new(text.len() as f64 * 7., 18.),
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
fn node(key: &str) -> TreeNode {
    TreeNode::new(key, key)
}
fn fixture() -> TreeModel {
    TreeModel::from_nodes(vec![
        node("root").children(vec![
            node("disabled").enabled(false),
            node("branch").children(vec![
                node("off").enabled(false),
                node("file"),
                node("other"),
            ]),
        ]),
        node("sibling"),
    ])
    .unwrap()
}
fn host(tree: &TreeModel) -> Host {
    Host::new({
        let tree = tree.clone();
        move || {
            View::column(vec![
                View::button("Before", || {}).key("before"),
                tree.view().height(160).key("tree"),
                View::button("After", || {}).key("after"),
            ])
        }
    })
}
fn layout(h: &Host) {
    h.layout(Size::new(500., 280.), &Metrics).unwrap();
}
fn row(h: &Host, key: &str) -> Node {
    h.snapshot()
        .unwrap()
        .into_iter()
        .find(|n| n.kind == Kind::CollectionRow && n.key.as_deref() == Some(key))
        .unwrap()
}
fn cursor(h: &Host) -> String {
    h.snapshot()
        .unwrap()
        .iter()
        .find(|n| Some(n.id) == h.focused())
        .unwrap()
        .key
        .clone()
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
fn select(h: &Host, tree: &TreeModel, key: &str) {
    let root = h
        .snapshot()
        .unwrap()
        .iter()
        .find(|n| n.tree_model.is_some())
        .unwrap()
        .id;
    let index = tree.flatten().iter().position(|i| i.key == key).unwrap();
    h.select_collection_item(root, index, false).unwrap();
    layout(h);
}
fn click(h: &Host, key: &str, count: u32) {
    let row = row(h, key);
    for kind in [PointerKind::Down, PointerKind::Up] {
        h.pointer_click(
            Pointer {
                kind,
                position: Point {
                    x: row.bounds.x + 160.,
                    y: row.bounds.y + 12.,
                },
                button: 0,
                modifiers: Default::default(),
            },
            count,
        )
        .unwrap();
    }
    layout(h);
}
#[test]
fn keyboard_roves_one_tab_stop_and_right_left_expand_enter_child_parent_and_collapse() {
    let tree = fixture();
    let h = host(&tree);
    layout(&h);
    key(&h, Key::Tab, Default::default());
    assert_eq!(cursor(&h), "before");
    key(&h, Key::Tab, Default::default());
    assert_eq!(cursor(&h), "root");
    key(&h, Key::Right, Default::default());
    assert!(tree.item("root").unwrap().expanded);
    key(&h, Key::Right, Default::default());
    assert_eq!(cursor(&h), "branch");
    key(&h, Key::Right, Default::default());
    key(&h, Key::Right, Default::default());
    assert_eq!(cursor(&h), "file");
    assert_eq!(row(&h, "file").tree_item.unwrap().depth, 2);
    key(&h, Key::Left, Default::default());
    assert_eq!(cursor(&h), "branch");
    key(&h, Key::Left, Default::default());
    assert!(!tree.item("branch").unwrap().expanded);
    key(&h, Key::Left, Default::default());
    assert_eq!(cursor(&h), "root");
    key(&h, Key::Left, Default::default());
    assert!(!tree.item("root").unwrap().expanded);
    key(&h, Key::Tab, Default::default());
    assert_eq!(cursor(&h), "after");
    key(
        &h,
        Key::Tab,
        Modifiers {
            shift: true,
            ..Default::default()
        },
    );
    assert_eq!(cursor(&h), "root");
}
#[test]
fn collapse_projects_selected_descendants_and_focus_to_parent_without_losing_other_selections() {
    let tree = fixture().multiple();
    tree.expand("root").unwrap();
    tree.expand("branch").unwrap();
    let h = host(&tree);
    layout(&h);
    tree.select("file", false);
    tree.select("other", true);
    tree.select("sibling", true);
    h.focus(row(&h, "other").id).unwrap();
    layout(&h);
    tree.collapse("branch").unwrap();
    layout(&h);
    assert_eq!(cursor(&h), "branch");
    assert_eq!(tree.cursor().as_deref(), Some("branch"));
    assert_eq!(
        tree.selected(),
        ["branch".into(), "sibling".into()].into_iter().collect()
    );
    tree.expand("branch").unwrap();
    layout(&h);
    assert_eq!(cursor(&h), "branch");
    assert!(!tree.selected().contains("file"));
    key(
        &h,
        Key::Down,
        Modifiers {
            shift: true,
            ..Default::default()
        },
    );
    assert_eq!(cursor(&h), "file");
    assert!(!tree.selected().contains("off"));
}
#[test]
fn removed_rows_and_fresh_snapshots_restore_logical_context_with_new_host_lifetimes() {
    let tree = fixture();
    tree.expand("root").unwrap();
    tree.expand("branch").unwrap();
    let h = host(&tree);
    layout(&h);
    select(&h, &tree, "file");
    let old = row(&h, "root").id;
    tree.set_children("branch", vec![node("replacement")])
        .unwrap();
    layout(&h);
    assert_eq!(cursor(&h), "branch");
    assert_eq!(tree.selected(), ["branch".into()].into_iter().collect());
    tree.set_nodes(vec![
        node("root").children(vec![node("branch")]),
        node("sibling"),
    ])
    .unwrap();
    layout(&h);
    assert_eq!(cursor(&h), "root");
    assert_ne!(row(&h, "root").id, old);
    assert!(!h.activate(old).unwrap());
    assert_eq!(tree.selected(), ["root".into()].into_iter().collect());
    tree.set_nodes(vec![node("fresh")]).unwrap();
    layout(&h);
    assert_eq!(cursor(&h), "fresh");
    assert!(tree.selected().is_empty());
    tree.set_nodes(vec![]).unwrap();
    layout(&h);
    assert!(h.focused().is_none());
}
#[test]
fn disclosure_actions_focus_the_row_and_outside_focus_cancels_pending_reveal() {
    let tree = fixture();
    let h = host(&tree);
    layout(&h);
    let root = row(&h, "root");
    let disclosure = h
        .snapshot()
        .unwrap()
        .iter()
        .find(|n| n.key.as_deref() == Some("$disclosure"))
        .unwrap()
        .id;
    assert!(h.activate(disclosure).unwrap());
    layout(&h);
    assert_eq!(h.focused(), Some(root.id));
    assert!(tree.item("root").unwrap().expanded);
    tree.expand("branch").unwrap();
    layout(&h);
    select(&h, &tree, "file");
    let after = h
        .snapshot()
        .unwrap()
        .iter()
        .find(|n| n.key.as_deref() == Some("after"))
        .unwrap()
        .id;
    h.focus(after).unwrap();
    tree.collapse("root").unwrap();
    layout(&h);
    assert_eq!(h.focused(), Some(after));
    let list = h
        .snapshot()
        .unwrap()
        .iter()
        .find(|n| n.tree_model.is_some())
        .unwrap()
        .id;
    h.select_collection_item(list, 0, false).unwrap();
    h.focus(after).unwrap();
    layout(&h);
    assert_eq!(h.focused(), Some(after));
}
#[test]
fn enter_double_click_and_expansion_callbacks_have_defined_default_and_custom_behavior() {
    let activations = State::new(vec![]);
    let expansions = State::new(vec![]);
    let tree = fixture()
        .on_activate(activations.handler(|s, key| s.update(|v| v.push(key))))
        .on_expand(expansions.handler(|s, event: TreeExpansion| s.update(|v| v.push(event))));
    tree.expand("root").unwrap();
    tree.expand("root").unwrap();
    assert_eq!(expansions.get().len(), 1);
    tree.expand("branch").unwrap();
    let h = host(&tree);
    layout(&h);
    select(&h, &tree, "file");
    key(&h, Key::Enter, Default::default());
    h.key(KeyEvent {
        key: Key::Enter,
        down: true,
        repeat: true,
        modifiers: Default::default(),
    })
    .unwrap();
    assert_eq!(activations.get(), ["file"]);
    click(&h, "file", 2);
    assert_eq!(activations.get(), ["file", "file"]);
    click(&h, "branch", 2);
    assert_eq!(activations.get(), ["file", "file", "branch"]);
    assert!(tree.item("branch").unwrap().expanded);
    let default = TreeModel::from_nodes(vec![node("folder").children(vec![node("leaf")])]).unwrap();
    let h = host(&default);
    layout(&h);
    select(&h, &default, "folder");
    key(&h, Key::Enter, Default::default());
    assert!(default.item("folder").unwrap().expanded);
    click(&h, "folder", 2);
    assert!(!default.item("folder").unwrap().expanded);
}
#[test]
fn loading_children_preserves_focus_and_failed_branches_can_collapse_without_retrying() {
    let attempts = Rc::new(Cell::new(0));
    let online = State::new(false);
    let tree = TreeModel::from_nodes(vec![
        node("remote").deferred(),
        node("bad").lazy({
            let attempts = attempts.clone();
            let online = online.clone();
            move || {
                attempts.set(attempts.get() + 1);
                if online.get() {
                    Ok(vec![node("ready")])
                } else {
                    Err(Error("Offline".into()))
                }
            }
        }),
    ])
    .unwrap();
    let h = host(&tree);
    layout(&h);
    select(&h, &tree, "remote");
    key(&h, Key::Right, Default::default());
    assert!(tree.item("remote").unwrap().loading);
    key(&h, Key::Right, Default::default());
    assert_eq!(cursor(&h), "remote");
    tree.pending_load("remote")
        .unwrap()
        .complete(Ok(vec![node("off").enabled(false), node("child")]))
        .unwrap();
    layout(&h);
    assert_eq!(cursor(&h), "remote");
    key(&h, Key::Right, Default::default());
    assert_eq!(cursor(&h), "child");
    select(&h, &tree, "bad");
    key(&h, Key::Right, Default::default());
    assert_eq!(attempts.get(), 1);
    key(&h, Key::Left, Default::default());
    assert!(!tree.item("bad").unwrap().expanded);
    key(&h, Key::Right, Default::default());
    assert_eq!(attempts.get(), 1);
    online.set(true);
    key(&h, Key::Enter, Default::default());
    assert_eq!(attempts.get(), 2);
    assert!(tree.item("bad").unwrap().error.is_none());
}
#[test]
fn options_reveal_typeahead_and_selection_use_known_keys_and_configured_geometry() {
    for options in [
        TreeOptions {
            row_height: 0.,
            ..Default::default()
        },
        TreeOptions {
            indentation: f64::NAN,
            ..Default::default()
        },
        TreeOptions {
            visible_rows: 0,
            ..Default::default()
        },
    ] {
        assert!(TreeModel::with_options(vec![], options).is_err());
    }
    let tree = TreeModel::with_options(
        vec![node("root").children(vec![
                node("branch").children(
                    (0..20)
                        .map(|i| TreeNode::new(i.to_string(), format!("Leaf {i:02}")))
                        .collect(),
                ),
            ])],
        TreeOptions {
            row_height: 40.,
            visible_rows: 4,
            overscan_rows: 0,
            indentation: 24.,
            selection: ListSelectionMode::Multiple,
        },
    )
    .unwrap();
    let h = host(&tree);
    layout(&h);
    assert!(tree.reveal("19").unwrap());
    layout(&h);
    select(&h, &tree, "19");
    assert_eq!(cursor(&h), "19");
    assert_eq!(tree.tree_options().indentation, 24.);
    assert_eq!(
        row(&h, "19").tree_item.unwrap().parent.as_deref(),
        Some("branch")
    );
    assert!(h.collection_text("Leaf 05").unwrap());
    layout(&h);
    assert_eq!(cursor(&h), "5");
    key(
        &h,
        Key::Down,
        Modifiers {
            shift: true,
            ..Default::default()
        },
    );
    assert!(tree.selected().contains("5"));
    assert!(tree.selected().contains("6"));
    key(
        &h,
        Key::A,
        Modifiers {
            command: true,
            ..Default::default()
        },
    );
    assert_eq!(tree.selection_count(), 22);
    tree.clear_selection();
    assert_eq!(tree.selection_count(), 0);
    assert!(!tree.reveal("missing").unwrap());
}
#[test]
fn callback_replacement_and_reentrant_expansion_are_safe_and_large_trees_stay_virtual() {
    let holder = Rc::new(std::cell::RefCell::new(None::<TreeModel>));
    let events = Rc::new(Cell::new(0));
    let tree = fixture().on_expand({
        let holder = holder.clone();
        let events = events.clone();
        move |event| {
            events.set(events.get() + 1);
            if event.expanded {
                holder
                    .borrow()
                    .as_ref()
                    .unwrap()
                    .collapse(&event.key)
                    .unwrap();
            }
        }
    });
    holder.replace(Some(tree.clone()));
    tree.expand("root").unwrap();
    assert_eq!(events.get(), 1);
    assert!(!tree.item("root").unwrap().expanded);
    holder.borrow_mut().take();
    let tree = fixture()
        .on_activate_with_model(|tree, _| tree.set_nodes(vec![node("replacement")]).unwrap());
    let h = host(&tree);
    layout(&h);
    select(&h, &tree, "root");
    key(&h, Key::Enter, Default::default());
    assert_eq!(cursor(&h), "replacement");
    let tree = TreeModel::from_nodes(vec![
        node("root").children(
            (0..100_000)
                .map(|i| TreeNode::new(i.to_string(), format!("File {i}")))
                .collect(),
        ),
    ])
    .unwrap();
    tree.expand("root").unwrap();
    let h = host(&tree);
    layout(&h);
    select(&h, &tree, "root");
    key(&h, Key::End, Default::default());
    assert_eq!(cursor(&h), "99999");
    assert!(h.snapshot().unwrap().len() < 80);
    key(&h, Key::Left, Default::default());
    assert_eq!(cursor(&h), "root");
    key(&h, Key::Left, Default::default());
    assert_eq!(tree.visible_count(), 1);
    assert!(h.snapshot().unwrap().len() < 20);
}
