use scars_ui::*;
use std::{cell::Cell, rc::Rc};
fn key_list(tree: &TreeModel) -> Vec<String> {
    tree.flatten().into_iter().map(|i| i.key).collect()
}
fn node(key: &str) -> TreeNode {
    TreeNode::new(key, key)
}
#[test]
fn eager_metadata_and_global_validation_include_collapsed_branches_atomically() {
    let tree = TreeModel::from_nodes(vec![
        node("root").children(vec![node("a").children(vec![node("hidden")]), node("b")]),
    ])
    .unwrap();
    assert_eq!(tree.known_count(), 4);
    assert_eq!(key_list(&tree), ["root"]);
    let hidden = tree.item("hidden").unwrap();
    assert_eq!(hidden.parent.as_deref(), Some("a"));
    assert_eq!(hidden.depth, 2);
    assert!(!hidden.expanded);
    for bad in [
        vec![
            node("x").children(vec![node("duplicate")]),
            node("y").children(vec![node("duplicate")]),
        ],
        vec![node(" ")],
        vec![node("both").loading(true).failed("bad")],
    ] {
        assert!(tree.set_nodes(bad).is_err());
        assert_eq!(tree.known_count(), 4);
        assert!(tree.item("hidden").is_some());
    }
    assert!(tree.expand("root").unwrap());
    assert_eq!(key_list(&tree), ["root", "a", "b"]);
    assert!(tree.expand("a").unwrap());
    assert_eq!(key_list(&tree), ["root", "a", "hidden", "b"]);
    tree.collapse("root").unwrap();
    assert_eq!(tree.known_count(), 4);
    assert_eq!(tree.visible_count(), 1);
    assert!(tree.item("a").unwrap().expanded);
    tree.expand("root").unwrap();
    assert_eq!(tree.visible_count(), 4);
}
#[test]
fn lazy_snapshots_cache_once_and_empty_results_become_leaves() {
    let calls = Rc::new(Cell::new(0));
    let tree = TreeModel::from_nodes(vec![
        node("lazy").lazy({
            let calls = calls.clone();
            move || {
                calls.set(calls.get() + 1);
                Ok(vec![node("child")])
            }
        }),
        node("empty").lazy(|| Ok(vec![])),
    ])
    .unwrap();
    assert_eq!(calls.get(), 0);
    assert_eq!(
        tree.item("lazy").unwrap().load_state,
        TreeLoadState::Unloaded
    );
    assert!(tree.item("lazy").unwrap().expandable);
    tree.expand("lazy").unwrap();
    assert_eq!(calls.get(), 1);
    assert_eq!(tree.known_count(), 3);
    assert_eq!(tree.item("lazy").unwrap().child_count, 1);
    tree.collapse("lazy").unwrap();
    tree.expand("lazy").unwrap();
    assert_eq!(calls.get(), 1);
    tree.expand("empty").unwrap();
    let empty = tree.item("empty").unwrap();
    assert!(!empty.expandable);
    assert!(!empty.expanded);
    assert_eq!(empty.load_state, TreeLoadState::Loaded);
    assert!(!tree.expand("empty").unwrap());
}
#[test]
fn loading_requests_complete_after_collapse_and_stale_duplicate_results_are_inert() {
    let pending = State::new(None::<TreeLoad>);
    let tree = TreeModel::from_nodes(vec![node("remote").deferred()])
        .unwrap()
        .on_load(pending.handler(|s, r| s.set(Some(r))));
    tree.expand("remote").unwrap();
    let first = pending.get().unwrap();
    assert!(first.is_current());
    assert!(tree.item("remote").unwrap().loading);
    tree.collapse("remote").unwrap();
    assert!(first.complete(Ok(vec![node("child")])).unwrap());
    assert_eq!(tree.visible_count(), 1);
    assert_eq!(tree.known_count(), 2);
    assert!(!first.is_current());
    assert!(!first.complete(Ok(vec![node("late")])).unwrap());
    tree.expand("remote").unwrap();
    assert_eq!(key_list(&tree), ["remote", "child"]);
    let retired = tree.begin_load("remote").unwrap();
    let current = tree.begin_load("remote").unwrap();
    assert!(!retired.complete(Err(Error("old failure".into()))).unwrap());
    assert!(tree.last_failure().is_none());
    current.complete(Ok(vec![node("replacement")])).unwrap();
    assert!(tree.item("child").is_none());
    let old = tree.begin_load("remote").unwrap();
    tree.set_nodes(vec![node("remote").deferred()]).unwrap();
    assert!(!old.complete(Ok(vec![node("wrong-generation")])).unwrap());
    assert_eq!(tree.known_count(), 1);
    assert!(!tree.item("remote").unwrap().expanded);
}
#[test]
fn load_errors_are_returned_visible_and_cached_until_explicit_retry() {
    let online = State::new(false);
    let calls = Rc::new(Cell::new(0));
    let tree = TreeModel::from_nodes(vec![node("remote").lazy({
        let online = online.clone();
        let calls = calls.clone();
        move || {
            calls.set(calls.get() + 1);
            if online.get() {
                Ok(vec![node("ready")])
            } else {
                Err(Error("Offline".into()))
            }
        }
    })])
    .unwrap();
    assert_eq!(tree.expand("remote").unwrap_err().0, "Offline");
    let failed = tree.item("remote").unwrap();
    assert!(failed.expanded);
    assert_eq!(failed.error.as_deref(), Some("Offline"));
    assert_eq!(tree.last_failure().unwrap().key, "remote");
    assert_eq!(calls.get(), 1);
    tree.collapse("remote").unwrap();
    assert!(tree.expand("remote").is_err());
    assert_eq!(calls.get(), 1);
    online.set(true);
    tree.retry("remote").unwrap();
    assert_eq!(calls.get(), 2);
    assert!(tree.last_failure().is_none());
    assert_eq!(key_list(&tree), ["remote", "ready"]);
}
#[test]
fn lazy_child_validation_checks_hidden_siblings_and_preserves_the_old_subtree() {
    let tree = TreeModel::from_nodes(vec![
        node("left").children(vec![node("taken")]),
        node("right").children(vec![node("old")]).deferred(),
    ])
    .unwrap();
    let request = tree.begin_load("right").unwrap();
    assert!(
        request
            .complete(Ok(vec![
                node("new"),
                node("nested").children(vec![node("taken")])
            ]))
            .is_err()
    );
    assert_eq!(tree.known_count(), 4);
    assert!(tree.item("old").is_some());
    assert!(tree.item("new").is_none());
    assert!(
        tree.item("right")
            .unwrap()
            .error
            .unwrap()
            .contains("globally unique")
    );
    let next = tree.begin_load("right").unwrap();
    next.complete(Ok(vec![node("old").children(vec![node("fresh")])]))
        .unwrap();
    assert_eq!(tree.known_count(), 5);
    assert_eq!(tree.item("fresh").unwrap().depth, 2);
    assert!(tree.item("right").unwrap().error.is_none());
}
#[test]
fn disabled_loading_and_accessible_metadata_are_integrated_and_loaders_are_read_only() {
    let tree = TreeModel::from_nodes(vec![
        node("off")
            .enabled(false)
            .children(vec![node("child").deferred()]),
        node("loading")
            .loading(true)
            .accessible_label("Network documents"),
    ])
    .unwrap();
    assert!(!tree.item("child").unwrap().enabled);
    assert!(!tree.expand("off").unwrap());
    assert!(tree.begin_load("child").is_err());
    let loading = tree.item("loading").unwrap();
    assert!(loading.loading);
    assert_eq!(loading.accessible_label, "Network documents");
    tree.pending_load("loading")
        .unwrap()
        .complete(Ok(vec![node("done")]))
        .unwrap();
    assert!(!tree.item("loading").unwrap().loading);
    let source = State::new(0);
    let tree = TreeModel::from_nodes(vec![node("bad").lazy({
        let source = source.clone();
        move || {
            source.set(1);
            Ok(vec![])
        }
    })])
    .unwrap();
    assert!(tree.expand("bad").is_err());
    assert_eq!(source.get(), 0);
    assert_eq!(
        tree.item("bad").unwrap().error.as_deref(),
        Some("Tree loader panicked")
    );
    let host = Host::new({
        let tree = tree.clone();
        move || {
            tree.retry("bad").unwrap();
            View::label("Bad")
        }
    });
    assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| host.snapshot())).is_err());
    assert_eq!(source.get(), 0);
}
#[test]
fn deferred_handlers_can_complete_synchronously_replace_the_model_and_do_not_retain_it() {
    let tree = TreeModel::from_nodes(vec![node("remote").deferred()])
        .unwrap()
        .on_load(|request| {
            request.complete(Ok(vec![node("ready")])).unwrap();
        });
    tree.expand("remote").unwrap();
    assert_eq!(tree.known_count(), 2);
    let holder = Rc::new(std::cell::RefCell::new(None::<TreeModel>));
    let tree = TreeModel::from_nodes(vec![node("remote").deferred()])
        .unwrap()
        .on_load({
            let holder = holder.clone();
            move |request| {
                holder
                    .borrow()
                    .as_ref()
                    .unwrap()
                    .set_nodes(vec![node("replacement")])
                    .unwrap();
                assert!(!request.complete(Ok(vec![node("stale")])).unwrap());
            }
        });
    holder.replace(Some(tree.clone()));
    tree.expand("remote").unwrap();
    assert_eq!(key_list(&tree), ["replacement"]);
    holder.borrow_mut().take();
    let request = tree.begin_load("replacement").unwrap();
    drop(tree);
    assert!(!request.is_current());
    assert!(!request.complete(Ok(vec![])).unwrap());
}
#[test]
fn schedulers_observe_consistent_metadata_and_visible_rows_and_replacement_retires_actions() {
    let tree = TreeModel::from_nodes(vec![node("root").children(vec![node("child")])]).unwrap();
    let host = Host::new({
        let tree = tree.clone();
        move || tree.view()
    });
    host.snapshot().unwrap();
    let old = host
        .snapshot()
        .unwrap()
        .iter()
        .find(|n| n.kind == Kind::Button)
        .unwrap()
        .id;
    let wakes = Rc::new(Cell::new(0));
    host.on_invalidate({
        let tree = tree.clone();
        let wakes = wakes.clone();
        move || {
            assert_eq!(tree.visible_count(), tree.flatten().len());
            wakes.set(wakes.get() + 1);
        }
    });
    tree.expand("root").unwrap();
    assert!(wakes.get() > 0);
    host.snapshot().unwrap();
    tree.set_nodes(vec![node("root").children(vec![node("new")])])
        .unwrap();
    assert!(!host.activate(old).unwrap());
    assert!(!tree.item("root").unwrap().expanded);
}

#[test]
fn deep_collapsed_metadata_and_replaced_nested_requests_keep_distinct_lifetimes() {
    let mut root = node("2500");
    for depth in (0..2500).rev() {
        root = node(&depth.to_string()).children(vec![root]);
    }
    let tree = TreeModel::from_nodes(vec![root]).unwrap();
    assert_eq!(tree.known_count(), 2501);
    assert_eq!(tree.visible_count(), 1);
    assert_eq!(tree.item("2500").unwrap().depth, 2500);
    let tree =
        TreeModel::from_nodes(vec![node("root").children(vec![node("child").deferred()])]).unwrap();
    let child = tree.begin_load("child").unwrap();
    tree.set_children("root", vec![node("child").deferred()])
        .unwrap();
    assert!(!child.complete(Ok(vec![node("stale")])).unwrap());
    assert_eq!(tree.known_count(), 2);
    assert_eq!(
        tree.item("child").unwrap().load_state,
        TreeLoadState::Unloaded
    );
}

#[test]
fn disclosure_actions_render_load_errors_and_explicitly_retry_without_loading_during_build() {
    let online = State::new(false);
    let calls = Rc::new(Cell::new(0));
    let tree = TreeModel::from_nodes(vec![node("remote").lazy({
        let online = online.clone();
        let calls = calls.clone();
        move || {
            calls.set(calls.get() + 1);
            if online.get() {
                Ok(vec![node("ready")])
            } else {
                Err(Error("Offline".into()))
            }
        }
    })])
    .unwrap();
    let host = Host::new({
        let tree = tree.clone();
        move || tree.view()
    });
    let action = host
        .snapshot()
        .unwrap()
        .iter()
        .find(|n| n.key.as_deref() == Some("$disclosure"))
        .unwrap()
        .id;
    assert_eq!(calls.get(), 0);
    assert!(host.activate(action).unwrap());
    assert_eq!(calls.get(), 1);
    assert!(
        host.snapshot()
            .unwrap()
            .iter()
            .any(|n| n.kind == Kind::Label && n.text.contains("Offline (retry)"))
    );
    assert_eq!(tree.last_failure().unwrap().message, "Offline");
    online.set(true);
    assert!(host.activate(action).unwrap());
    assert_eq!(calls.get(), 2);
    assert!(tree.item("ready").is_some());
    assert!(tree.last_failure().is_none());
}

#[test]
fn replacing_tree_state_drops_loader_resources_after_releasing_the_state_borrow() {
    use std::cell::RefCell;
    struct CheckDrop {
        source: Rc<RefCell<Option<State<TreeModel>>>>,
        seen: Rc<Cell<usize>>,
    }
    impl Drop for CheckDrop {
        fn drop(&mut self) {
            self.seen
                .set(self.source.borrow().as_ref().unwrap().get().known_count());
        }
    }
    let source = Rc::new(RefCell::new(None::<State<TreeModel>>));
    let seen = Rc::new(Cell::new(0));
    let guard = CheckDrop {
        source: source.clone(),
        seen: seen.clone(),
    };
    let old = TreeModel::from_nodes(vec![node("old").lazy(move || {
        let _ = &guard;
        Ok(vec![])
    })])
    .unwrap();
    let state = State::new(old);
    source.replace(Some(state.clone()));
    state.set(TreeModel::from_nodes(vec![node("new"), node("second")]).unwrap());
    assert_eq!(seen.get(), 2);
    source.borrow_mut().take();
}
