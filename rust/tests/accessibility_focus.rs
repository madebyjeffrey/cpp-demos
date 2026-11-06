//! Focus establishes a text baseline; selection notifications describe subsequent edits.
#[cfg(target_os = "macos")]
fn main() {
    if std::env::var_os("SCARS_WINDOW_DESKTOP_TESTS").is_none() {
        println!("Native focus announcement tests skipped");
        return;
    }
    use foundation::{NSArray, NSDate, NSRange, NSRunLoop, NSString};
    use objc2::{msg_send, rc::Retained, runtime::AnyObject};
    use scars_window::{ui::*, *};
    fn pump() {
        NSRunLoop::mainRunLoop().runUntilDate(&NSDate::dateWithTimeIntervalSinceNow(0.04));
    }
    fn clear(n: &AnyObject) {
        pump();
        unsafe {
            let _: () = msg_send![n, scarsClearAccessibilityEvents];
        }
    }
    fn events(n: &AnyObject) -> Vec<String> {
        pump();
        unsafe {
            let a: Retained<NSArray<NSString>> = msg_send![n, scarsAccessibilityEvents];
            a.iter().map(|s| s.to_string()).collect()
        }
    }
    let app = Application::new(ApplicationOptions {
        name: "Focus notification tests".into(),
        quit_on_last_window_close: false,
        ..Default::default()
    })
    .unwrap();
    let value = State::new("Hello".to_owned());
    let window = Window::builder("Hidden focus test")
        .view(move || {
            View::column(vec![
                View::text_field("", value.binding()).key("first"),
                View::text_field("", value.binding()).key("second"),
            ])
        })
        .build(&app)
        .unwrap();
    let host = window.view_host().unwrap();
    host.set_active(true);
    let native = window.native_view();
    for name in ["first", "second"] {
        clear(native);
        let id = host
            .snapshot()
            .unwrap()
            .into_iter()
            .find(|n| n.key.as_deref() == Some(name))
            .unwrap()
            .id;
        host.focus(id).unwrap();
        let e = events(native);
        assert_eq!(
            e.iter()
                .filter(|s| s.starts_with("AXFocusedUIElementChanged@"))
                .count(),
            1,
            "{e:?}"
        );
        assert!(
            !e.iter().any(|s| s.starts_with("AXSelectedTextChanged@")),
            "focus must not announce initial selection twice: {e:?}"
        );
    }
    unsafe {
        let focus: Retained<AnyObject> = msg_send![native, accessibilityFocusedUIElement];
        clear(native);
        let _: () = msg_send![&*focus,setAccessibilitySelectedTextRange:NSRange::new(1,2)];
        let e = events(native);
        assert_eq!(
            e.iter()
                .filter(|s| s.starts_with("AXSelectedTextChanged@"))
                .count(),
            1,
            "{e:?}"
        );
        assert!(
            !e.iter()
                .any(|s| s.starts_with("AXFocusedUIElementChanged@"))
        );
    }
    let tabs = Tabs::new("a");
    let menu = Menu::new();
    window
        .set_view(move || {
            View::layer(vec![
                View::column(vec![
                    menu.button("Options", "options"),
                    tabs.view(vec![
                        Tab::new("a", "First", View::label("A")),
                        Tab::new("b", "Second", View::label("B")),
                    ]),
                ]),
                menu.view("options", vec![MenuAction::new("Action", || {})]),
            ])
        })
        .unwrap();
    fn find(o: &AnyObject, label: &str) -> Option<Retained<AnyObject>> {
        use objc2::Message;
        unsafe {
            let name: Option<Retained<NSString>> = msg_send![o, accessibilityLabel];
            if name.is_some_and(|n| n.to_string() == label) {
                return Some(o.retain());
            }
            let kids: Option<Retained<NSArray<AnyObject>>> = msg_send![o, accessibilityChildren];
            kids.and_then(|a| a.iter().find_map(|v| find(&v, label)))
        }
    }
    unsafe {
        let first = find(native, "First").unwrap();
        let second = find(native, "Second").unwrap();
        for tab in [&first, &second] {
            let expanded: bool = msg_send![&**tab,isAccessibilitySelectorAllowed:objc2::sel!(isAccessibilityExpanded)];
            assert!(
                !expanded,
                "selected and unselected tabs must omit expansion state"
            );
        }
        let changed: bool = msg_send![&*second, accessibilityPerformPress];
        assert!(changed);
        for (tab, expected) in [(&first, false), (&second, true)] {
            let selected: bool = msg_send![&**tab, isAccessibilitySelected];
            assert_eq!(selected, expected);
            let expanded: bool = msg_send![&**tab,isAccessibilitySelectorAllowed:objc2::sel!(isAccessibilityExpanded)];
            assert!(!expanded, "switching tabs must not add expansion state");
        }
        let menu = find(native, "Options").unwrap();
        let expanded: bool =
            msg_send![&*menu,isAccessibilitySelectorAllowed:objc2::sel!(isAccessibilityExpanded)];
        assert!(expanded, "menu buttons retain their expansion state");
        let expanded: bool = msg_send![&*menu, isAccessibilityExpanded];
        assert!(!expanded);
    }
    assert!(
        !window.native_window().isVisible(),
        "this audit must not disturb an interactive session"
    );
    window.close();
    println!("Focus posts once; subsequent selection changes remain announced");
}
#[cfg(not(target_os = "macos"))]
fn main() {}
