//! Real AppKit post names/targets, independent of drawing or an external AX client.
#[cfg(target_os = "macos")]
fn main() {
    if std::env::var_os("SCARS_WINDOW_DESKTOP_TESTS").is_none() {
        println!("Native notification tests skipped");
        return;
    }
    use foundation::{NSArray, NSDate, NSRange, NSRunLoop, NSString};
    use objc2::{Message, msg_send, rc::Retained, runtime::AnyObject};
    use scars_window::{ui::*, *};
    fn pump() {
        NSRunLoop::mainRunLoop().runUntilDate(&NSDate::dateWithTimeIntervalSinceNow(0.04));
    }
    fn find(o: &AnyObject, label: &str) -> Retained<AnyObject> {
        fn search(o: &AnyObject, label: &str) -> Option<Retained<AnyObject>> {
            unsafe {
                let text: Option<Retained<NSString>> = msg_send![o, accessibilityLabel];
                if text.is_some_and(|s| s.to_string() == label) {
                    return Some(o.retain());
                }
                let children: Option<Retained<NSArray<AnyObject>>> =
                    msg_send![o, accessibilityChildren];
                children.and_then(|a| a.iter().find_map(|e| search(&e, label)))
            }
        }
        search(o, label).unwrap_or_else(|| panic!("Missing {label}"))
    }
    fn clear(native: &AnyObject) {
        pump();
        unsafe {
            let _: () = msg_send![native, scarsClearAccessibilityEvents];
        }
    }
    fn events(native: &AnyObject) -> Vec<String> {
        pump();
        unsafe {
            let a: Retained<NSArray<NSString>> = msg_send![native, scarsAccessibilityEvents];
            a.iter().map(|s| s.to_string()).collect()
        }
    }
    fn count(events: &[String], name: &str, object: &AnyObject) -> usize {
        let prefix = format!("{name}@{:x}:", object as *const _ as usize);
        events.iter().filter(|s| s.starts_with(&prefix)).count()
    }
    fn posted(events: &[String], name: &str, object: &AnyObject) {
        assert_eq!(count(events, name, object), 1, "{name}: {events:?}");
    }
    fn layout(events: &[String]) {
        assert!(
            events
                .iter()
                .any(|s| s.starts_with("AXLayoutChanged@") && !s.ends_with(":0")),
            "layout payload: {events:?}"
        );
    }
    let app = Application::new(ApplicationOptions {
        name: "Notification contracts".into(),
        quit_on_last_window_close: false,
        ..Default::default()
    })
    .unwrap();
    let window = Window::builder("Notifications")
        .size(640., 520.)
        .build(&app)
        .unwrap();
    let native = window.native_view();
    let secret = State::new("secret-one".to_owned());
    let text = State::new("Document".to_owned());
    let volume = State::new(2.);
    window
        .set_view({
            let secret = secret.clone();
            let text = text.clone();
            let volume = volume.clone();
            move || {
                View::column(vec![
                    View::password_field("", secret.binding()).accessible_label("Secret"),
                    View::text_field("", text.binding()).accessible_label("Name"),
                    View::slider(volume.binding(), 0., 10., 1.).accessible_label("Volume"),
                ])
            }
        })
        .unwrap();
    unsafe {
        let password = find(native, "Secret");
        let field = find(native, "Name");
        let slider = find(native, "Volume");
        clear(native);
        secret.set("secret-two".into()); // Same length, different private value.
        let e = events(native);
        posted(&e, "AXValueChanged", &password);
        assert!(!e.iter().any(|s| s.contains("secret")));
        assert_eq!(count(&e, "AXValueChanged", &field), 0);
        clear(native);
        let _: () = msg_send![&*field, setAccessibilityFocused:true];
        posted(&events(native), "AXFocusedUIElementChanged", &field);
        clear(native);
        let _: () = msg_send![&*field, setAccessibilitySelectedTextRange:NSRange::new(1, 3)];
        let e = events(native);
        posted(&e, "AXSelectedTextChanged", &field);
        assert_eq!(count(&e, "AXValueChanged", &field), 0);
        clear(native);
        text.set("New name".into());
        posted(&events(native), "AXValueChanged", &field);
        clear(native);
        let marked = NSString::from_str("かな");
        let _: () = msg_send![native, setMarkedText:&*marked,selectedRange:NSRange::new(2,0),replacementRange:NSRange::new(isize::MAX as usize,0)];
        posted(&events(native), "AXTextInputMarkingSessionBegan", &field);
        clear(native);
        let _: () = msg_send![native, unmarkText];
        posted(&events(native), "AXTextInputMarkingSessionEnded", &field);
        clear(native);
        volume.set(7.);
        posted(&events(native), "AXValueChanged", &slider);
        clear(native);
        assert!(events(native).is_empty(), "idle posts must settle");
    }
    let list = ListModel::new(
        (0..100_000)
            .map(|i| ListItem::new(i.to_string(), format!("Row {i}")))
            .collect(),
        32.,
    )
    .multiple();
    window
        .set_view({
            let list = list.clone();
            move || list.view().accessible_label("Items")
        })
        .unwrap();
    unsafe {
        let root = find(native, "Items");
        let rows: Retained<NSArray<AnyObject>> = msg_send![&*root, accessibilityChildren];
        assert_eq!(rows.len(), 100_000);
        let realized: usize = msg_send![&*rows, scarsRealizedRowCount];
        assert_eq!(realized, 0, "count must not allocate logical rows");
        let a = rows.objectAtIndex(1);
        let b = rows.objectAtIndex(99_999);
        list.set_selection(&["1".into()]).unwrap();
        clear(native);
        list.set_selection(&["99999".into()]).unwrap();
        let e = events(native);
        posted(&e, "AXSelectedRowsChanged", &root);
        posted(&e, "AXSelectedChildrenChanged", &root);
        clear(native);
        let _: () = msg_send![&*a, setAccessibilityFocused:true];
        posted(&events(native), "AXFocusedUIElementChanged", &a);
        assert!(list.is_selected("99999"), "focus must preserve selection");
        let snapshot: usize = msg_send![native, scarsAccessibilitySnapshotCount];
        assert!(snapshot < 200, "snapshot grew to {snapshot}");
        let attr = NSString::from_str("AXChildren");
        for index in 100..300usize {
            objc2::rc::autoreleasepool(|_| {
                let transient: Retained<NSArray<AnyObject>> = msg_send![&*root, accessibilityArrayAttributeValues:&*attr,index:index,maxCount:1usize];
                drop(transient.objectAtIndex(0));
            });
        }
        let after: usize = msg_send![native, scarsAccessibilitySnapshotCount];
        assert_eq!(
            snapshot, after,
            "released logical queries must leave no snapshot entries"
        );
        clear(native);
        list.set_items(vec![ListItem::new("1", "Survivor")])
            .unwrap();
        let e = events(native);
        posted(&e, "AXRowCountChanged", &root);
        posted(&e, "AXUIElementDestroyed", &b);
        layout(&e);
        clear(native);
        assert!(events(native).is_empty());
    }
    let custom = ListModel::new(vec![ListItem::new("one", "One")], 40.);
    let draft = State::new("Custom".to_owned());
    window
        .set_view({
            let custom = custom.clone();
            let draft = draft.clone();
            move || {
                custom
                    .view_with(|_| {
                        View::text_field("", draft.binding()).accessible_label("Row editor")
                    })
                    .accessible_label("Custom list")
            }
        })
        .unwrap();
    unsafe {
        let root = find(native, "Custom list");
        let rows: Retained<NSArray<AnyObject>> = msg_send![&*root, accessibilityChildren];
        let row = rows.objectAtIndex(0);
        let editor = find(&row, "Row editor");
        let parent: Retained<AnyObject> = msg_send![&*editor, accessibilityParent];
        assert!(std::ptr::eq(&*parent, &*row));
        clear(native);
        let _: () = msg_send![&*editor, setAccessibilityFocused:true];
        posted(&events(native), "AXFocusedUIElementChanged", &editor);
        clear(native);
        draft.set("Changed custom content".into());
        posted(&events(native), "AXValueChanged", &editor);
    }
    let tree = TreeModel::from_nodes(vec![
        TreeNode::new("branch", "Branch").children(vec![TreeNode::new("child", "Child")]),
    ])
    .unwrap();
    window
        .set_view({
            let tree = tree.clone();
            move || tree.view().accessible_label("Outline")
        })
        .unwrap();
    unsafe {
        let root = find(native, "Outline");
        let rows: Retained<NSArray<AnyObject>> = msg_send![&*root, accessibilityRows];
        let branch = rows.objectAtIndex(0);
        clear(native);
        tree.expand("branch").unwrap();
        let e = events(native);
        posted(&e, "AXRowExpanded", &branch);
        posted(&e, "AXRowCountChanged", &root);
        layout(&e);
        let rows: Retained<NSArray<AnyObject>> = msg_send![&*root, accessibilityRows];
        let child = rows.objectAtIndex(1);
        clear(native);
        tree.collapse("branch").unwrap();
        let e = events(native);
        posted(&e, "AXRowCollapsed", &branch);
        assert_eq!(
            count(&e, "AXUIElementDestroyed", &child),
            0,
            "collapse preserves the logical lifetime"
        );
        clear(native);
        tree.set_nodes(vec![]).unwrap();
        let e = events(native);
        posted(&e, "AXUIElementDestroyed", &branch);
        posted(&e, "AXUIElementDestroyed", &child);
    }
    clear(native);
    let table = TableModel::with_options(
        vec![TableColumn::new("name", "Name")],
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
    window
        .set_view({
            let table = table.clone();
            move || table.view().accessible_label("Table")
        })
        .unwrap();
    unsafe {
        let root = find(native, "Table");
        let a: Retained<AnyObject> =
            msg_send![&*root, accessibilityCellForColumn:0isize,row:0isize];
        let b: Retained<AnyObject> =
            msg_send![&*root, accessibilityCellForColumn:0isize,row:1isize];
        table.select_cell(TableCell::new("a", "name"), ListSelection::Replace);
        clear(native);
        table.select_cell(TableCell::new("b", "name"), ListSelection::Replace);
        let e = events(native);
        posted(&e, "AXSelectedCellsChanged", &root);
        assert_eq!(count(&e, "AXSelectedRowsChanged", &root), 0);
        clear(native);
        let _: () = msg_send![&*b, setAccessibilityFocused:true];
        posted(&events(native), "AXFocusedUIElementChanged", &b);
        clear(native);
        table.set_text("a", "name", "Changed").unwrap();
        posted(&events(native), "AXValueChanged", &a);
        clear(native);
        table
            .set_rows(vec![TableRow::new("b").cell("name", "B")])
            .unwrap();
        posted(&events(native), "AXUIElementDestroyed", &a);
        clear(native);
        window.close();
        let e = events(native);
        posted(&e, "AXUIElementDestroyed", &root);
        assert_eq!(
            count(&e, "AXUIElementDestroyed", &a),
            0,
            "retired targets are destroyed only once"
        );
        posted(&e, "AXUIElementDestroyed", &b);
        clear(native);
        assert!(
            events(native).is_empty(),
            "pending work must not post after unmount"
        );
    }
    window.close();
    println!(
        "Native value, text selection, logical focus, row/cell selection, disclosure, destruction and bounded notification contracts passed"
    );
}
#[cfg(not(target_os = "macos"))]
fn main() {}
