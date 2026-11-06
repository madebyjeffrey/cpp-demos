//! Opt-in main-thread AppKit table contracts, including logical offscreen cells.
#[cfg(target_os = "macos")]
fn main() {
    if std::env::var_os("SCARS_WINDOW_DESKTOP_TESTS").is_none() {
        println!("Native table accessibility tests skipped");
        return;
    }
    use foundation::{NSArray, NSPoint, NSRange, NSRect, NSString};
    use objc2::{msg_send, rc::Retained, runtime::AnyObject};
    use scars_window::{ui::*, *};
    let app = Application::named("Table accessibility tests").unwrap();
    let table = TableModel::with_options(
        vec![
            TableColumn::new("name", "Document")
                .editable(true)
                .validate(|_, s| {
                    if s.is_empty() {
                        Err(scars_window::ui::Error("Name is required".into()))
                    } else {
                        Ok(())
                    }
                }),
            TableColumn::new("rank", "Rank"),
            TableColumn::new("fixed", "Readonly")
                .sortable(false)
                .content(|_, _| View::button("Inspect", || {}).key("inspect")),
        ],
        (0..100_000)
            .map(|i| {
                TableRow::new(i.to_string())
                    .cell("name", format!("Document {i:06}"))
                    .cell("rank", i.to_string())
                    .cell("fixed", "Constant")
                    .enabled(i != 1)
            })
            .collect(),
        TableOptions {
            selection_mode: TableSelectionMode::Cells,
            multiple_selection: true,
            ..Default::default()
        },
    )
    .unwrap();
    let enabled = State::new(true);
    let window = Window::builder("Table contracts")
        .size(620., 480.)
        .view({
            let table = table.clone();
            let enabled = enabled.clone();
            move || {
                table
                    .view()
                    .accessible_label("Workspace documents")
                    .enabled(enabled.get())
            }
        })
        .build(&app)
        .unwrap();
    let host = window.view_host().unwrap();
    let native = window.native_view();
    unsafe {
        let roots: Retained<NSArray<AnyObject>> = msg_send![native, accessibilityChildren];
        assert_eq!(roots.len(), 1);
        let root = roots.objectAtIndex(0);
        let role: Retained<NSString> = msg_send![&*root, accessibilityRole];
        assert_eq!(role.to_string(), "AXTable");
        let label: Retained<NSString> = msg_send![&*root, accessibilityLabel];
        assert_eq!(label.to_string(), "Workspace documents");
        let _: () = msg_send![&*root,setAccessibilityFocused:true];
        let focused: Retained<AnyObject> = msg_send![native, accessibilityFocusedUIElement];
        assert!(std::ptr::eq(&*focused, &*root));
        let claims_focus: bool = msg_send![&*root, isAccessibilityFocused];
        assert!(claims_focus);
        let count: isize = msg_send![&*root, accessibilityRowCount];
        assert_eq!(count, 100000);
        let count: isize = msg_send![&*root, accessibilityColumnCount];
        assert_eq!(count, 3);
        let rows: Retained<NSArray<AnyObject>> = msg_send![&*root, accessibilityRows];
        assert_eq!(rows.len(), 100000);
        let realized: usize = msg_send![&*rows, scarsRealizedEntryCount];
        assert_eq!(realized, 0);
        let copied: Retained<NSArray<AnyObject>> = msg_send![&*rows, copy];
        assert!(std::ptr::eq(&*rows, &*copied));
        let columns: Retained<NSArray<AnyObject>> = msg_send![&*root, accessibilityColumns];
        assert_eq!(columns.len(), 3);
        let realized: usize = msg_send![&*columns, scarsRealizedEntryCount];
        assert_eq!(realized, 0);
        let first = columns.objectAtIndex(0);
        let role: Retained<NSString> = msg_send![&*first, accessibilityRole];
        assert_eq!(role.to_string(), "AXColumn");
        let header: Retained<AnyObject> = msg_send![&*first, accessibilityHeader];
        let headers: Retained<NSArray<AnyObject>> =
            msg_send![&*root, accessibilityColumnHeaderUIElements];
        assert!(std::ptr::eq(&*header, &*headers.objectAtIndex(0)));
        let children: Retained<NSArray<AnyObject>> = msg_send![&*root, accessibilityChildren];
        assert_eq!(children.len(), 100004);
        let realized: usize = msg_send![&*children, scarsRealizedEntryCount];
        assert_eq!(realized, 0);
        assert!(std::ptr::eq(&*children.objectAtIndex(100000), &*first));
        let header_group = children.objectAtIndex(100003);
        let direct_header: Retained<AnyObject> = msg_send![&*root, accessibilityHeader];
        assert!(std::ptr::eq(&*direct_header, &*header_group));
        let allowed: bool = msg_send![&*root,isAccessibilitySelectorAllowed:objc2::sel!(setAccessibilitySelectedColumns:)];
        assert!(!allowed);

        let group_headers: Retained<NSArray<AnyObject>> =
            msg_send![&*header_group, accessibilityChildren];
        assert_eq!(group_headers.len(), 3);
        assert!(std::ptr::eq(&*group_headers.objectAtIndex(0), &*header));
        let parent: Retained<AnyObject> = msg_send![&*header, accessibilityParent];
        assert!(std::ptr::eq(&*parent, &*header_group));
        let column_cells: Retained<NSArray<AnyObject>> = msg_send![&*first, accessibilityChildren];
        assert_eq!(column_cells.len(), 100000);
        let realized: usize = msg_send![&*column_cells, scarsRealizedEntryCount];
        assert_eq!(realized, 0);
        let lastrow = rows.objectAtIndex(99999);
        let rowcells: Retained<NSArray<AnyObject>> = msg_send![&*lastrow, accessibilityChildren];
        assert_eq!(rowcells.len(), 3);
        let cell: Retained<AnyObject> =
            msg_send![&*root,accessibilityCellForColumn:0isize,row:99999isize];
        assert!(std::ptr::eq(&*cell, &*rowcells.objectAtIndex(0)));
        assert!(std::ptr::eq(&*cell, &*column_cells.objectAtIndex(99999)));
        let role: Retained<NSString> = msg_send![&*cell, accessibilityRole];
        assert_eq!(role.to_string(), "AXCell");
        let parent: Retained<AnyObject> = msg_send![&*cell, accessibilityParent];
        assert!(std::ptr::eq(&*parent, &*lastrow));
        let index: NSRange = msg_send![&*cell, accessibilityRowIndexRange];
        assert_eq!(index, NSRange::new(99999, 1));
        let index: NSRange = msg_send![&*cell, accessibilityColumnIndexRange];
        assert_eq!(index, NSRange::new(0, 1));
        let frame: NSRect = msg_send![&*cell, accessibilityFrame];
        assert_eq!(frame.size.height, 0.);
        let absent: Option<Retained<AnyObject>> =
            msg_send![&*root,accessibilityCellForColumn:-1isize,row:0isize];
        assert!(absent.is_none());
        let absent: Option<Retained<AnyObject>> =
            msg_send![&*root,accessibilityCellForColumn:3isize,row:0isize];
        assert!(absent.is_none());
        let visible: Retained<NSArray<AnyObject>> = msg_send![&*root, accessibilityVisibleCells];
        assert!(visible.len() < 100);
        let values = NSArray::from_slice(&[&*cell]);
        let _: () = msg_send![&*root,setAccessibilitySelectedCells:&*values];
        assert_eq!(table.selection_count(), 1);
        let selected: Retained<NSArray<AnyObject>> = msg_send![&*root, accessibilitySelectedCells];
        assert!(std::ptr::eq(&*selected.objectAtIndex(0), &*cell));
        let off: Retained<AnyObject> =
            msg_send![&*root,accessibilityCellForColumn:0isize,row:1isize];
        let available: bool = msg_send![&*off, isAccessibilityEnabled];
        assert!(!available);
        let invalid = NSArray::from_slice(&[&*cell, &*off]);
        let _: () = msg_send![&*root,setAccessibilitySelectedCells:&*invalid];
        assert_eq!(table.selection_count(), 1);
        let _: () = msg_send![&*cell,setAccessibilityFocused:true];
        let focus: Retained<AnyObject> = msg_send![native, accessibilityFocusedUIElement];
        assert!(std::ptr::eq(&*focus, &*cell));
        let editing: bool = msg_send![&*cell, accessibilityPerformConfirm];
        assert!(editing);
        let focus: Retained<AnyObject> = msg_send![native, accessibilityFocusedUIElement];
        let role: Retained<NSString> = msg_send![&*focus, accessibilityRole];
        assert_eq!(role.to_string(), "AXTextField");
        let parent: Retained<AnyObject> = msg_send![&*focus, accessibilityParent];
        assert!(std::ptr::eq(&*parent, &*cell));
        let children: Retained<NSArray<AnyObject>> = msg_send![&*cell, accessibilityChildren];
        assert!(std::ptr::eq(&*children.objectAtIndex(0), &*focus));
        let empty = NSString::from_str("");
        let _: () = msg_send![&*cell,setAccessibilityValue:&*empty];
        let help: Retained<NSString> = msg_send![&*cell, accessibilityHelp];
        assert_eq!(help.to_string(), "Name is required");
        let draft: Retained<NSString> = msg_send![&*cell, accessibilityValue];
        assert_eq!(draft.to_string(), "");
        let good = NSString::from_str("Edited document");
        let _: () = msg_send![&*cell,setAccessibilityValue:&*good];
        assert_eq!(
            table.text("99999", "name").as_deref(),
            Some("Edited document")
        );
        let retired: bool = msg_send![&*focus, isAccessibilityElement];
        assert!(!retired);
        let readonly: Retained<AnyObject> =
            msg_send![&*root,accessibilityCellForColumn:2isize,row:99999isize];
        let allowed: bool = msg_send![&*readonly,isAccessibilitySelectorAllowed:objc2::sel!(setAccessibilityValue:)];
        assert!(!allowed);
        let _: () = msg_send![&*readonly,setAccessibilityValue:&*good];
        assert_eq!(table.text("99999", "fixed").as_deref(), Some("Constant"));
        let controls: Retained<NSArray<AnyObject>> = msg_send![&*readonly, accessibilityChildren];
        assert_eq!(controls.len(), 1);
        let inspect = controls.objectAtIndex(0);
        let _: () = msg_send![&*inspect,setAccessibilityFocused:true];
        let focused: Retained<AnyObject> = msg_send![native, accessibilityFocusedUIElement];
        assert!(std::ptr::eq(&*focused, &*inspect));
        let claimed: bool = msg_send![&*readonly, isAccessibilityFocused];
        assert!(!claimed);
        let parent: Retained<AnyObject> = msg_send![&*inspect, accessibilityParent];
        assert!(std::ptr::eq(&*parent, &*readonly));
        let frame: NSRect = msg_send![&*inspect, accessibilityFrame];
        let point = NSPoint::new(
            frame.origin.x + frame.size.width / 2.,
            frame.origin.y + frame.size.height / 2.,
        );
        let hit: Retained<AnyObject> = msg_send![native,accessibilityHitTest:point];
        assert!(std::ptr::eq(&*hit, &*inspect));
        let sort_before = table.sort();
        let _: () = msg_send![&*header,setAccessibilityFocused:true];
        assert_eq!(table.sort(), sort_before);
        let sorted: bool = msg_send![&*header, accessibilityPerformPress];
        assert!(sorted);
        let direction: app_kit::NSAccessibilitySortDirection =
            msg_send![&*header, accessibilitySortDirection];
        assert_eq!(direction, app_kit::NSAccessibilitySortDirection::Ascending);
        let current = table.row_index("99999").unwrap();
        let index: NSRange = msg_send![&*cell, accessibilityRowIndexRange];
        assert_eq!(index.location, current);
        let fresh: Retained<AnyObject> =
            msg_send![&*root,accessibilityCellForColumn:0isize,row:current as isize];
        assert!(std::ptr::eq(&*fresh, &*cell));
        table.move_column("name", 2).unwrap();
        let index: NSRange = msg_send![&*cell, accessibilityColumnIndexRange];
        assert_eq!(index, NSRange::new(2, 1));
        let fresh: Retained<AnyObject> =
            msg_send![&*root,accessibilityCellForColumn:2isize,row:current as isize];
        assert!(std::ptr::eq(&*fresh, &*cell));
        // Existing immutable arrays keep original order while live attributes update.
        assert!(std::ptr::eq(&*columns.objectAtIndex(0), &*first));
        let column_index: isize = msg_send![&*first, accessibilityIndex];
        assert_eq!(column_index, 2);
        table.set_selection_mode(TableSelectionMode::Rows, true);
        let selected = NSArray::from_slice(&[&*lastrow]);
        let _: () = msg_send![&*root,setAccessibilitySelectedRows:&*selected];
        assert_eq!(table.selected_rows(0, 1), vec!["99999"]);
        let selected: bool = msg_send![&*lastrow, isAccessibilitySelected];
        assert!(selected);
        let _: () = msg_send![&*lastrow,setAccessibilityFocused:true];
        let focus: Retained<AnyObject> = msg_send![native, accessibilityFocusedUIElement];
        assert!(std::ptr::eq(&*focus, &*lastrow));
        let before: usize = msg_send![&*root, scarsTableEntryCacheCount];
        let attr = NSString::from_str("AXRows");
        for index in 200..400usize {
            objc2::rc::autoreleasepool(|_| {
                let a: Retained<NSArray<AnyObject>> = msg_send![&*root,accessibilityArrayAttributeValues:&*attr,index:index,maxCount:1usize];
                let _: Retained<AnyObject> = a.objectAtIndex(0);
            });
        }
        let after: usize = msg_send![&*root, scarsTableEntryCacheCount];
        assert_eq!(before, after);
        assert!(host.snapshot().unwrap().len() < 180);
        enabled.set(false);
        let available: bool = msg_send![&*cell, isAccessibilityEnabled];
        assert!(!available);
        let sort = table.sort();
        let acted: bool = msg_send![&*header, accessibilityPerformPress];
        assert!(!acted);
        assert_eq!(table.sort(), sort);
        let _: () = msg_send![&*cell,setAccessibilityValue:&*empty];
        assert_eq!(
            table.text("99999", "name").as_deref(),
            Some("Edited document")
        );
        enabled.set(true);
        table.set_rows(vec![]).unwrap();
        table
            .set_rows(vec![TableRow::new("99999").cell("name", "Replacement")])
            .unwrap();
        let stale: bool = msg_send![&*cell, isAccessibilityElement];
        assert!(!stale);
        let _: () = msg_send![&*cell,setAccessibilityValue:&*good];
        assert_eq!(table.text("99999", "name").as_deref(), Some("Replacement"));
        assert_eq!(rows.len(), 100000);
        window.clear_view();
        let stale: bool = msg_send![&*header, isAccessibilityElement];
        assert!(!stale);
    }
    window.show();
    app.quit_after(std::time::Duration::from_millis(200));
    app.run().unwrap();
    println!("Native table roles, logical cells, actions, editing and lazy arrays passed");
}
#[cfg(not(target_os = "macos"))]
fn main() {}
