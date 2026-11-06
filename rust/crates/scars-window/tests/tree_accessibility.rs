//! Opt-in, main-thread AppKit outline contracts and lazy-array resource checks.
#[cfg(target_os = "macos")]
fn main() {
    if std::env::var_os("SCARS_WINDOW_DESKTOP_TESTS").is_none() {
        println!("Native tree accessibility tests skipped");
        return;
    }
    use foundation::{NSArray, NSPoint, NSRect, NSSize, NSString};
    use objc2::{msg_send, rc::Retained, runtime::AnyObject};
    use scars_window::{ui::*, *};
    use std::{cell::Cell, rc::Rc};
    let app = Application::named("Tree accessibility tests").unwrap();
    let online = State::new(false);
    let disabled_loads = Rc::new(Cell::new(0));
    let tree = TreeModel::from_nodes(vec![
        TreeNode::new("root", "Root")
            .accessible_label("Project workspace")
            .children(vec![
                TreeNode::new("branch", "Folder").children(vec![
                    TreeNode::new("grand", "Notes").accessible_label("Notes document"),
                ]),
                TreeNode::new("deferred", "Remote").deferred(),
                TreeNode::new("retry", "Retryable").lazy({
                    let online = online.clone();
                    move || {
                        if online.get() {
                            Ok(vec![TreeNode::new("roadmap", "Roadmap")])
                        } else {
                            Err(scars_window::ui::Error("Offline".into()))
                        }
                    }
                }),
                TreeNode::new("off", "Disabled").enabled(false).lazy({
                    let calls = disabled_loads.clone();
                    move || {
                        calls.set(calls.get() + 1);
                        Ok(vec![])
                    }
                }),
                TreeNode::new("last", "Last"),
            ]),
    ])
    .unwrap()
    .multiple();
    tree.expand("root").unwrap();
    tree.expand("branch").unwrap();
    assert!(tree.expand("retry").is_err());
    let root_enabled = State::new(true);
    let window = Window::builder("Outline contracts")
        .size(620., 500.)
        .view({
            let tree = tree.clone();
            let enabled = root_enabled.clone();
            move || {
                tree.view()
                    .accessible_label("Projects")
                    .enabled(enabled.get())
            }
        })
        .build(&app)
        .unwrap();
    let host = window.view_host().unwrap();
    let native = window.native_view();
    unsafe {
        let roots: Retained<NSArray<AnyObject>> = msg_send![native, accessibilityChildren];
        let outline = roots.objectAtIndex(0);
        let role: Retained<NSString> = msg_send![&*outline, accessibilityRole];
        assert_eq!(role.to_string(), "AXOutline");
        let rows: Retained<NSArray<AnyObject>> = msg_send![&*outline, accessibilityRows];
        assert_eq!(rows.len(), 7);
        let cached: usize = msg_send![&*rows, scarsRealizedRowCount];
        assert_eq!(cached, 0);
        let copied: Retained<NSArray<AnyObject>> = msg_send![&*rows, copy];
        assert!(std::ptr::eq(&*rows, &*copied));
        let cached: usize = msg_send![&*rows, scarsRealizedRowCount];
        assert_eq!(cached, 0);
        let root = rows.objectAtIndex(0);
        let branch = rows.objectAtIndex(1);
        let grand = rows.objectAtIndex(2);
        let remote = rows.objectAtIndex(3);
        let retry = rows.objectAtIndex(4);
        let off = rows.objectAtIndex(5);
        let last = rows.objectAtIndex(6);
        let label: Retained<NSString> = msg_send![&*root, accessibilityLabel];
        assert_eq!(label.to_string(), "Project workspace");
        let level: isize = msg_send![&*grand, accessibilityDisclosureLevel];
        assert_eq!(level, 2);
        let parent: Retained<AnyObject> = msg_send![&*grand, accessibilityParent];
        assert!(std::ptr::eq(&*parent, &*outline));
        let disclosed_by: Retained<AnyObject> = msg_send![&*grand, accessibilityDisclosedByRow];
        assert!(std::ptr::eq(&*disclosed_by, &*branch));
        let direct: Retained<NSArray<AnyObject>> = msg_send![&*root, accessibilityDisclosedRows];
        assert_eq!(direct.len(), 5);
        assert!(std::ptr::eq(&*direct.objectAtIndex(0), &*branch));
        let children: Retained<NSArray<AnyObject>> =
            msg_send![&*branch, accessibilityDisclosedRows];
        assert_eq!(children.len(), 1);
        assert!(std::ptr::eq(&*children.objectAtIndex(0), &*grand));
        let leaf_allowed: bool = msg_send![&*grand,isAccessibilitySelectorAllowed:objc2::sel!(setAccessibilityDisclosed:)];
        assert!(!leaf_allowed);
        let enabled: bool = msg_send![&*off, isAccessibilityEnabled];
        assert!(!enabled);
        let _: () = msg_send![&*off,setAccessibilityDisclosed:true];
        assert_eq!(disabled_loads.get(), 0);
        let selected = NSArray::from_slice(&[&*grand, &*last]);
        let _: () = msg_send![&*outline,setAccessibilitySelectedRows:&*selected];
        assert_eq!(tree.selection_count(), 2);
        let _: () = msg_send![&*grand,setAccessibilityFocused:true];
        let focus: Retained<AnyObject> = msg_send![native, accessibilityFocusedUIElement];
        assert!(std::ptr::eq(&*focus, &*grand));
        assert_eq!(tree.selection_count(), 2);
        let focused: bool = msg_send![&*grand, isAccessibilityFocused];
        assert!(focused);
        let selected_rows: Retained<NSArray<AnyObject>> =
            msg_send![&*outline, accessibilitySelectedRows];
        assert_eq!(selected_rows.len(), 2);
        let foreign = NSString::from_str("foreign");
        let wrong = NSArray::from_slice(&[&*foreign]);
        let _: () = msg_send![&*outline,setAccessibilitySelectedRows:&*wrong];
        assert_eq!(tree.selection_count(), 2);
        let row = host
            .snapshot()
            .unwrap()
            .into_iter()
            .find(|n| n.tree_item.as_ref().is_some_and(|i| i.key == "grand"))
            .unwrap();
        let rect = native.convertRect_toView(
            NSRect::new(
                NSPoint::new(row.bounds.x + 100., row.bounds.y + 12.),
                NSSize::ZERO,
            ),
            None,
        );
        let point = window.native_window().convertRectToScreen(rect).origin;
        let hit: Retained<AnyObject> = msg_send![native,accessibilityHitTest:point];
        assert!(std::ptr::eq(&*hit, &*grand));
        let _: () = msg_send![&*root,setAccessibilityDisclosed:false];
        let focus: Retained<AnyObject> = msg_send![native, accessibilityFocusedUIElement];
        assert!(std::ptr::eq(&*focus, &*root));
        assert_eq!(tree.selected(), ["root".to_owned()].into());
        let live: bool = msg_send![&*grand, isAccessibilityElement];
        assert!(!live);
        assert_eq!(rows.len(), 7);
        let _: () = msg_send![&*root,setAccessibilityExpanded:true];
        let fresh: Retained<NSArray<AnyObject>> = msg_send![&*outline, accessibilityRows];
        assert!(std::ptr::eq(&*fresh.objectAtIndex(2), &*grand));
        let _: () = msg_send![&*remote,setAccessibilityDisclosed:true];
        let description: Retained<NSString> = msg_send![&*remote, accessibilityValueDescription];
        assert_eq!(description.to_string(), "Loading");
        tree.pending_load("deferred")
            .unwrap()
            .complete(Ok(vec![TreeNode::new("fetched", "Fetched")]))
            .unwrap();
        let description: Option<Retained<NSString>> =
            msg_send![&*remote, accessibilityValueDescription];
        assert!(description.is_none());
        let children: Retained<NSArray<AnyObject>> =
            msg_send![&*remote, accessibilityDisclosedRows];
        assert_eq!(children.len(), 1);
        let help: Retained<NSString> = msg_send![&*retry, accessibilityHelp];
        assert_eq!(help.to_string(), "Offline");
        let attempted: bool = msg_send![&*retry, accessibilityPerformConfirm];
        assert!(attempted);
        online.set(true);
        let retry_children: Retained<NSArray<AnyObject>> =
            msg_send![&*retry, accessibilityChildren];
        assert_eq!(retry_children.len(), 1);
        let retry_button = retry_children.objectAtIndex(0);
        let parent: Retained<AnyObject> = msg_send![&*retry_button, accessibilityParent];
        assert!(std::ptr::eq(&*parent, &*retry));
        let accepted: bool = msg_send![&*retry_button, accessibilityPerformPress];
        assert!(accepted);
        let help: Option<Retained<NSString>> = msg_send![&*retry, accessibilityHelp];
        assert!(help.is_none());
        let stale: bool = msg_send![&*retry_button, isAccessibilityElement];
        assert!(!stale);
        root_enabled.set(false);
        let live: bool = msg_send![&*grand, isAccessibilityElement];
        assert!(live);
        let available: bool = msg_send![&*grand, isAccessibilityEnabled];
        assert!(!available);
        let was_expanded = tree.row("root").unwrap().item.expanded;
        let _: () = msg_send![&*root,setAccessibilityDisclosed:false];
        assert_eq!(tree.row("root").unwrap().item.expanded, was_expanded);
        root_enabled.set(true);
        tree.set_nodes(vec![TreeNode::new("root", "Replacement")])
            .unwrap();
        let live: bool = msg_send![&*root, isAccessibilityElement];
        assert!(!live);
        let _: () = msg_send![&*root,setAccessibilityExpanded:true];
        tree.clear_selection();
        let stale = NSArray::from_slice(&[&*root]);
        let _: () = msg_send![&*outline,setAccessibilitySelectedRows:&*stale];
        assert_eq!(tree.selection_count(), 0);
        window.clear_view();
        let live: bool = msg_send![&*last, isAccessibilityElement];
        assert!(!live);
    }
    let large = TreeModel::from_nodes(vec![
        TreeNode::new("root", "Large tree").children(
            (0..100_000)
                .map(|i| {
                    if i == 99_999 {
                        TreeNode::new(i.to_string(), "Last branch")
                            .children(vec![TreeNode::new("deep", "Deep child")])
                    } else {
                        TreeNode::new(i.to_string(), format!("Item {i}"))
                    }
                })
                .collect(),
        ),
    ])
    .unwrap();
    large.expand("root").unwrap();
    let large_host = window
        .set_view({
            let tree = large.clone();
            move || tree.view()
        })
        .unwrap();
    unsafe {
        let roots: Retained<NSArray<AnyObject>> = msg_send![native, accessibilityChildren];
        let outline = roots.objectAtIndex(0);
        let rows: Retained<NSArray<AnyObject>> = msg_send![&*outline, accessibilityRows];
        assert_eq!(rows.len(), 100001);
        let cached: usize = msg_send![&*rows, scarsRealizedRowCount];
        assert_eq!(cached, 0);
        let last = rows.objectAtIndex(100000);
        let cached: usize = msg_send![&*rows, scarsRealizedRowCount];
        assert_eq!(cached, 1);
        let frame: NSRect = msg_send![&*last, accessibilityFrame];
        assert_eq!(frame.size.height, 0.);
        let level: isize = msg_send![&*last, accessibilityDisclosureLevel];
        assert_eq!(level, 1);
        let root = rows.objectAtIndex(0);
        let children: Retained<NSArray<AnyObject>> = msg_send![&*root, accessibilityDisclosedRows];
        assert_eq!(children.len(), 100000);
        let cached: usize = msg_send![&*children, scarsRealizedRowCount];
        assert_eq!(cached, 0);
        assert!(std::ptr::eq(&*children.objectAtIndex(99999), &*last));
        let attribute = NSString::from_str("AXRows");
        let one: Retained<NSArray<AnyObject>> = msg_send![&*outline,accessibilityArrayAttributeValues:&*attribute,index:100000usize,maxCount:1usize];
        assert_eq!(one.len(), 1);
        assert!(std::ptr::eq(&*one.objectAtIndex(0), &*last));
        let baseline: usize = msg_send![&*outline, scarsLogicalRowCacheCount];
        for index in 2..202usize {
            objc2::rc::autoreleasepool(|_| {
                let transient: Retained<NSArray<AnyObject>> = msg_send![&*outline,accessibilityArrayAttributeValues:&*attribute,index:index,maxCount:1usize];
                let _: Retained<AnyObject> = transient.objectAtIndex(0);
            });
        }
        let remaining: usize = msg_send![&*outline, scarsLogicalRowCacheCount];
        assert_eq!(remaining, baseline);
        let _: () = msg_send![&*last,setAccessibilityDisclosed:true];
        let expanded: bool = msg_send![&*last, isAccessibilityDisclosed];
        assert!(expanded);
        let children: Retained<NSArray<AnyObject>> = msg_send![&*last, accessibilityDisclosedRows];
        assert_eq!(children.len(), 1);
        let deep = children.objectAtIndex(0);
        let level: isize = msg_send![&*deep, accessibilityDisclosureLevel];
        assert_eq!(level, 2);
        let parent: Retained<AnyObject> = msg_send![&*deep, accessibilityDisclosedByRow];
        assert!(std::ptr::eq(&*parent, &*last));
        assert!(large_host.snapshot().unwrap().len() < 140);
        let _: () = msg_send![&*last,setAccessibilityDisclosed:false];
        let live: bool = msg_send![&*deep, isAccessibilityElement];
        assert!(!live);
        let _: () = msg_send![&*last,setAccessibilityDisclosed:true];
        let children: Retained<NSArray<AnyObject>> = msg_send![&*last, accessibilityDisclosedRows];
        assert!(std::ptr::eq(&*children.objectAtIndex(0), &*deep));
        window.clear_view();
        let live: bool = msg_send![&*last, isAccessibilityElement];
        assert!(!live);
    }
    window.show();
    app.quit_after(std::time::Duration::from_millis(200));
    app.run().unwrap();
    println!("Native tree hierarchy, actions, identity and lazy-array tests passed");
}
#[cfg(not(target_os = "macos"))]
fn main() {}
