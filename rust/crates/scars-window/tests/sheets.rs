//! Opt-in native sheet ownership, isolation, IME and focus regression test.
#[cfg(target_os = "macos")]
fn main() {
    if std::env::var_os("SCARS_WINDOW_DESKTOP_TESTS").is_none() {
        println!("Native sheet tests skipped");
        return;
    }
    use foundation::{NSArray, NSRange, NSString};
    use objc2::{Message, msg_send, rc::Retained, runtime::AnyObject, sel};
    use scars_window::{
        ui::{self, State, view},
        *,
    };
    use std::{cell::Cell, rc::Rc, time::Duration};
    fn run_for(app: &Application) {
        app.post_after(Duration::from_millis(120), || {
            let native = app_kit::NSApplication::sharedApplication(objc2::MainThreadMarker::new().unwrap());
            native.stop(None);
            let wake = app_kit::NSEvent::otherEventWithType_location_modifierFlags_timestamp_windowNumber_context_subtype_data1_data2(
                app_kit::NSEventType::ApplicationDefined, foundation::NSPoint::new(0.,0.),
                app_kit::NSEventModifierFlags::empty(), 0.,0,None,0,0,0).unwrap();
            native.postEvent_atStart(&wake,false);
        });
        app.run().unwrap();
    }
    fn find(root: &AnyObject, label: &str) -> Option<Retained<AnyObject>> {
        // SAFETY: The root and descendants implement NSAccessibility.
        unsafe {
            let value: Option<Retained<NSString>> = msg_send![root, accessibilityLabel];
            if value.is_some_and(|s| s.to_string() == label) {
                return Some(root.retain());
            }
            let children: Option<Retained<NSArray<AnyObject>>> =
                msg_send![root, accessibilityChildren];
            children?.iter().find_map(|c| find(&c, label))
        }
    }
    fn actions() -> Vec<ui::AlertAction> {
        vec![
            ui::AlertAction::new("cancel", "Cancel").role(ui::AlertActionRole::Cancel),
            ui::AlertAction::new("save", "Save").role(ui::AlertActionRole::Default),
        ]
    }
    let errors = Rc::new(Cell::new(0));
    let app = Application::new(ApplicationOptions {
        name: "Sheet tests".into(),
        quit_on_last_window_close: false,
        on_error: Some(Rc::new({
            let errors = errors.clone();
            move |e| {
                eprintln!("{e}");
                errors.set(errors.get() + 1);
            }
        })),
        ..Default::default()
    })
    .unwrap();
    let parent_calls = State::new(0);
    let text = State::new("Original".to_owned());
    let parent=Window::builder("Sheet parent").view({let calls=parent_calls.clone();let text=text.clone();move||view!{column(padding:20){text_field("Name",bind:text,key:"name");button("Parent action",on_click:calls.action(|s|s.set(s.get()+1)));}}}).build(&app).unwrap();
    let other_calls = State::new(0);
    let other = Window::builder("Other window")
        .view({
            let calls = other_calls.clone();
            move || view! {button("Other action",on_click:calls.action(|s|s.set(s.get()+1)));}
        })
        .build(&app)
        .unwrap();
    other.show();
    parent.show();
    run_for(&app);
    let host = parent.view_host().unwrap();
    let field = host
        .snapshot()
        .unwrap()
        .iter()
        .find(|n| n.key.as_deref() == Some("name"))
        .unwrap()
        .id;
    host.focus(field).unwrap();
    let parent_ax = find(parent.native_view(), "Parent action").unwrap();
    let other_ax = find(other.native_view(), "Other action").unwrap();
    let results = State::new(vec![]);
    let alert = ui::Alert::new("Save changes?", actions(), {
        let results = results.clone();
        let parent = parent.surface_clone();
        move |result| {
            assert!(
                parent.native_window().attachedSheet().is_none(),
                "Release parent before completion"
            );
            results.update(|v| v.push(result));
        }
    })
    .unwrap();
    let draft = State::new(String::new());
    let sheet = parent
        .show_sheet(alert.clone(), {
            let draft = draft.clone();
            move || view! {text_field("Sheet name",bind:draft,key:"draft");}
        })
        .unwrap();
    run_for(&app);
    assert!(parent.native_window().attachedSheet().is_some());
    parent.request_close();
    assert!(parent.is_open());
    let rejected = ui::Alert::new("Second", actions(), |_| {}).unwrap();
    assert!(
        parent
            .show_sheet(rejected.clone(), || ui::View::label("Busy"))
            .is_err()
    );
    assert!(!rejected.completed());
    rejected.dismiss();
    // SAFETY: Documented accessibility and NSTextInputClient selectors on live native objects.
    unsafe {
        let acted: bool = msg_send![&*parent_ax, accessibilityPerformPress];
        assert!(!acted);
        assert_eq!(parent_calls.get(), 0);
        let acted: bool = msg_send![&*other_ax, accessibilityPerformPress];
        assert!(acted);
        assert_eq!(other_calls.get(), 1);
        let root = find(sheet.native_view(), "Save changes?").unwrap();
        let modal: bool = msg_send![&*root, isAccessibilityModal];
        assert!(modal);
        let default: Retained<AnyObject> = msg_send![&*root, accessibilityDefaultButton];
        let label: Retained<NSString> = msg_send![&*default, accessibilityLabel];
        assert_eq!(label.to_string(), "Save");
        let cancel: Retained<AnyObject> = msg_send![&*root, accessibilityCancelButton];
        let label: Retained<NSString> = msg_send![&*cancel, accessibilityLabel];
        assert_eq!(label.to_string(), "Cancel");
        let sh = sheet.view_host().unwrap();
        let field = sh
            .snapshot()
            .unwrap()
            .iter()
            .find(|n| n.key.as_deref() == Some("draft"))
            .unwrap()
            .id;
        sh.focus(field).unwrap();
        sh.set_active(true);
        let value = NSString::from_str("日本");
        let native = sheet.native_view();
        let _: () = msg_send![native,setMarkedText:&*value,selectedRange:NSRange::new(2,0),replacementRange:NSRange::new(isize::MAX as usize,0)];
        let _: () = msg_send![native,doCommandBySelector:sel!(insertNewline:)];
        assert_eq!(draft.get(), "日本");
        assert!(!alert.completed());
        assert!(parent.native_window().makeFirstResponder(None));
        let before: Retained<AnyObject> = msg_send![parent.native_window(), firstResponder];
        let was_content: bool = msg_send![&*before, isEqual:parent.native_view()];
        assert!(!was_content, "Exercise an actual native responder change");
        let _: () = msg_send![native,doCommandBySelector:sel!(insertNewline:)];
        assert_eq!(alert.result().unwrap().action.as_deref(), Some("save"));
        assert!(!sheet.is_open());
        let acted: bool = msg_send![&*cancel, accessibilityPerformPress];
        assert!(!acted);
    }
    run_for(&app);
    assert_eq!(results.get().len(), 1);
    assert_eq!(host.focused(), Some(field));
    unsafe {
        let responder: Retained<AnyObject> = msg_send![parent.native_window(), firstResponder];
        let restored: bool = msg_send![&*responder, isEqual:parent.native_view()];
        assert!(
            restored,
            "Restore the native content responder as well as logical focus"
        );
    }
    assert_eq!(
        host.text_input().unwrap().unwrap().active,
        parent.native_window().isKeyWindow()
    );
    unsafe {
        let acted: bool = msg_send![&*parent_ax, accessibilityPerformPress];
        assert!(acted);
    }
    assert_eq!(parent_calls.get(), 1);
    // A background sheet completion must not steal key status from another window.
    let alert = ui::Alert::new("Background", actions(), |_| {}).unwrap();
    let sheet = parent
        .show_sheet(alert.clone(), || ui::View::label("Other window is active"))
        .unwrap();
    run_for(&app);
    other.show();
    run_for(&app);
    let other_was_key = other.native_window().isKeyWindow();
    let foreground = std::env::args().any(|arg| arg == "--foreground");
    if foreground {
        assert!(
            other_was_key,
            "Foreground key-window audit requires an awake active desktop"
        );
    } else if !other_was_key {
        println!("Foreground key-window audit NOT RUN; use --foreground on an active desktop");
    }
    alert.dismiss();
    assert!(!sheet.is_open());
    if other_was_key {
        assert!(other.native_window().isKeyWindow());
    }
    parent.show();
    for path in ["escape", "dismiss", "unmount", "close", "parent"] {
        let results = State::new(vec![]);
        let a = ui::Alert::new(
            path,
            actions(),
            results.handler(|s, r| s.update(|v| v.push(r))),
        )
        .unwrap();
        let sheet = parent
            .show_sheet(a.clone(), || ui::View::label("Resolve once"))
            .unwrap();
        run_for(&app);
        match path {
            "escape" => unsafe {
                let _: () =
                    msg_send![sheet.native_view(),doCommandBySelector:sel!(cancelOperation:)];
            },
            "dismiss" => {
                a.dismiss();
            }
            "unmount" => sheet.clear_view(),
            "close" => sheet.request_close(),
            _ => parent.close(),
        }
        run_for(&app);
        assert!(
            !sheet.is_open(),
            "{path} did not close sheet; result {:?}",
            a.result()
        );
        assert_eq!(results.get().len(), 1);
        let result = &results.get()[0];
        assert_eq!(
            result.reason,
            match path {
                "escape" => ui::AlertReason::Escape,
                "dismiss" => ui::AlertReason::Dismissed,
                "unmount" => ui::AlertReason::Unmounted,
                _ => ui::AlertReason::Closed,
            }
        );
        assert!(!a.choose("save"));
        assert_eq!(results.get().len(), 1);
    }
    // An application-owned sheet outlives a discarded control handle, then the
    // parent owner's destruction resolves it without application Rc plumbing.
    let owner = Window::new(&app, WindowOptions::default()).unwrap();
    owner.show();
    let results = State::new(vec![]);
    let a = ui::Alert::new(
        "Drop parent",
        actions(),
        results.handler(|s, r| s.update(|v| v.push(r))),
    )
    .unwrap();
    let weak = owner
        .show_sheet(a.clone(), || ui::View::label("Owned by application"))
        .unwrap()
        .downgrade();
    run_for(&app);
    assert!(weak.upgrade().is_some_and(|s| s.is_open()));
    drop(owner);
    run_for(&app);
    assert_eq!(results.get().len(), 1);
    assert_eq!(results.get()[0].reason, ui::AlertReason::Closed);
    assert!(weak.upgrade().is_none_or(|s| !s.is_open()));
    let owner = Window::new(&app, WindowOptions::default()).unwrap();
    owner.show();
    let results = State::new(vec![]);
    let alert = ui::Alert::new(
        "Application teardown",
        actions(),
        results.handler(|s, r| s.update(|v| v.push(r))),
    )
    .unwrap();
    let sheet = owner
        .show_sheet(alert, || ui::View::label("Close with application"))
        .unwrap();
    run_for(&app);
    drop(app);
    assert!(!sheet.is_open());
    assert!(!owner.is_open());
    assert_eq!(results.get().len(), 1);
    assert_eq!(results.get()[0].reason, ui::AlertReason::Closed);
    assert_eq!(errors.get(), 0);
    println!(
        "Native sheets: actions, IME, isolation, focus, dismissal, close and ownership passed"
    );
}
#[cfg(not(target_os = "macos"))]
fn main() {
    println!("Native sheet tests require macOS");
}
