//! Main-thread native identity, modal scope, coordinates and teardown contracts.
#[cfg(target_os = "macos")]
fn main() {
    if std::env::var_os("SCARS_WINDOW_DESKTOP_TESTS").is_none() {
        println!("Native accessibility lifecycle tests skipped");
        return;
    }
    use foundation::{NSArray, NSDate, NSPoint, NSRange, NSRect, NSRunLoop, NSSize, NSString};
    use objc2::{msg_send, rc::Retained, runtime::AnyObject};
    use scars_window::{ui::*, *};
    use std::{cell::Cell, rc::Rc};
    fn find(o: &AnyObject, label: &str) -> Retained<AnyObject> {
        fn search(o: &AnyObject, label: &str) -> Option<Retained<AnyObject>> {
            unsafe {
                let text: Option<Retained<NSString>> = msg_send![o, accessibilityLabel];
                if text.is_some_and(|s| s.to_string() == label) {
                    use objc2::Message;
                    return Some(o.retain());
                }
                let children: Option<Retained<NSArray<AnyObject>>> =
                    msg_send![o, accessibilityChildren];
                children.and_then(|a| a.iter().find_map(|e| search(&e, label)))
            }
        }
        search(o, label).unwrap_or_else(|| panic!("Missing {label}"))
    }
    fn near(a: f64, b: f64) {
        assert!((a - b).abs() < 0.01, "{a} differs from {b}");
    }
    let app = Application::new(ApplicationOptions {
        name: "Accessibility lifecycle tests".into(),
        quit_on_last_window_close: false,
        ..Default::default()
    })
    .unwrap();
    let draws = Rc::new(Cell::new(0usize));
    let window = Window::builder("Lifecycle contracts")
        .size(640., 520.)
        .on_draw({
            let draws = draws.clone();
            move |_, _| draws.set(draws.get() + 1)
        })
        .build(&app)
        .unwrap();
    let native = window.native_view();
    fn pump() {
        NSRunLoop::mainRunLoop().runUntilDate(&NSDate::dateWithTimeIntervalSinceNow(0.04));
    }
    let model = State::new("Before".to_owned());
    window
        .set_view({
            let model = model.clone();
            move || View::label(model.get())
        })
        .unwrap();
    pump();
    assert!(!window.native_window().isVisible());
    assert_eq!(draws.get(), 0);
    unsafe {
        let before: u64 = msg_send![native, scarsAccessibilityPostCount];
        model.set("First change".into());
        model.set("Second change".into());
        let immediate: u64 = msg_send![native, scarsAccessibilityPostCount];
        assert_eq!(
            before, immediate,
            "notifications are deferred to avoid invalidation reentrancy"
        );
        pump();
        let after: u64 = msg_send![native, scarsAccessibilityPostCount];
        assert_eq!(
            after,
            before + 1,
            "coalesced layout change posts once without a drawable"
        );
        pump();
        let idle: u64 = msg_send![native, scarsAccessibilityPostCount];
        assert_eq!(idle, after);
        assert_eq!(draws.get(), 0);
        model.set("Pending unmount".into());
        window.clear_view();
        let closed: u64 = msg_send![native, scarsAccessibilityPostCount];
        pump();
        let later: u64 = msg_send![native, scarsAccessibilityPostCount];
        assert_eq!(
            closed, later,
            "retired mounts cancel queued notification work"
        );
    }
    let tabs = Tabs::new("draft");
    let visible = State::new(true);
    let text = State::new("Retained draft".to_owned());
    window
        .set_view({
            let tabs = tabs.clone();
            let visible = visible.clone();
            let text = text.clone();
            move || {
                if visible.get() {
                    tabs.view(vec![
                        Tab::lazy("draft", "Draft", {
                            let text = text.clone();
                            move || {
                                View::text_field("", text.binding())
                                    .accessible_label("Document name")
                                    .key("name")
                            }
                        }),
                        Tab::new("other", "Other", View::label("Other page")),
                    ])
                    .accessible_label("Documents")
                } else {
                    View::label("Removed")
                }
            }
        })
        .unwrap();
    unsafe {
        let field = find(native, "Document name");
        let identifier: Retained<NSString> = msg_send![&*field, accessibilityIdentifier];
        assert!(identifier.to_string().starts_with("node/"));
        tabs.select("other");
        let _: Retained<NSArray<AnyObject>> = msg_send![native, accessibilityChildren];
        let live: bool = msg_send![&*field, isAccessibilityElement];
        assert!(!live);
        tabs.select("draft");
        let again = find(native, "Document name");
        assert!(
            std::ptr::eq(&*again, &*field),
            "retained pages preserve native identity across hide/reveal"
        );
        let fresh_id: Retained<NSString> = msg_send![&*again, accessibilityIdentifier];
        assert_eq!(fresh_id.to_string(), identifier.to_string());
        visible.set(false);
        let _: Retained<NSArray<AnyObject>> = msg_send![native, accessibilityChildren];
        visible.set(true);
        let replacement = find(native, "Document name");
        assert!(!std::ptr::eq(&*replacement, &*field));
        let stale = NSString::from_str("Stale write");
        let _: () = msg_send![&*field,setAccessibilityValue:&*stale];
        assert_eq!(text.get(), "Retained draft");
    }
    let combo = ComboBox::new(vec![
        OptionItem::new("a", "Alpha"),
        OptionItem::new("b", "Beta"),
    ])
    .unwrap();
    let count = State::new(0usize);
    let host = window
        .set_view({
            let combo = combo.clone();
            let count = count.clone();
            move || {
                View::column(vec![
                    combo.view("Choose").accessible_label("Choice"),
                    View::spacer(),
                    View::button("Background action", count.action(|n| n.update(|v| *v += 1)))
                        .key("background"),
                ])
                .padding(20.)
            }
        })
        .unwrap();
    unsafe {
        let background = find(native, "Background action");
        let node = host
            .snapshot()
            .unwrap()
            .into_iter()
            .find(|n| n.key.as_deref() == Some("background"))
            .unwrap();
        let screen = window
            .native_window()
            .convertRectToScreen(native.convertRect_toView(
                NSRect::new(
                    NSPoint::new(
                        node.bounds.x + node.bounds.width / 2.,
                        node.bounds.y + node.bounds.height / 2.,
                    ),
                    NSSize::ZERO,
                ),
                None,
            ))
            .origin;
        combo.show();
        let _: Retained<NSArray<AnyObject>> = msg_send![native, accessibilityChildren];
        let live: bool = msg_send![&*background, isAccessibilityElement];
        assert!(!live);
        let hit: Option<Retained<AnyObject>> = msg_send![native,accessibilityHitTest:screen];
        assert!(
            hit.is_none(),
            "outside a suggestion popup, do not return an out-of-scope background proxy"
        );
        let pressed: bool = msg_send![&*background, accessibilityPerformPress];
        assert!(!pressed);
        assert_eq!(count.get(), 0);
        combo.cancel();
        let _: Retained<NSArray<AnyObject>> = msg_send![native, accessibilityChildren];
        let again = find(native, "Background action");
        assert!(std::ptr::eq(&*background, &*again));
    }
    let outer = Popup::new();
    let inner = Popup::new();
    outer.show();
    window
        .set_view({
            let outer = outer.clone();
            let inner = inner.clone();
            move || {
                outer.modal(
                    "Outer dialog",
                    View::layer(vec![
                        View::label("Owner content"),
                        inner.modal("Inner dialog", View::label("Nested content")),
                    ]),
                )
            }
        })
        .unwrap();
    unsafe {
        let owner = find(native, "Outer dialog");
        inner.show();
        let child = find(native, "Inner dialog");
        let active: bool = msg_send![&*owner, isAccessibilityElement];
        assert!(!active);
        let cancelled: bool = msg_send![&*owner, accessibilityPerformCancel];
        assert!(!cancelled);
        assert!(inner.open.get());
        let cancelled: bool = msg_send![&*child, accessibilityPerformCancel];
        assert!(cancelled);
        let restored = find(native, "Outer dialog");
        assert!(std::ptr::eq(&*owner, &*restored));
        inner.show();
        let replacement = find(native, "Inner dialog");
        assert!(!std::ptr::eq(&*child, &*replacement));
        let stale: bool = msg_send![&*child, accessibilityPerformCancel];
        assert!(!stale);
        assert!(inner.open.get());
        outer.dismiss();
        let _: Retained<NSArray<AnyObject>> = msg_send![native, accessibilityChildren];
        let live: bool = msg_send![&*replacement, isAccessibilityElement];
        assert!(!live);
        assert!(!inner.open.get());
    }
    let reverse = State::new(false);
    window
        .set_view({
            let reverse = reverse.clone();
            move || {
                let left = View::panel(vec![View::button("First local key", || {}).key("same")])
                    .key("left");
                let right = View::panel(vec![View::button("Second local key", || {}).key("same")])
                    .key("right");
                View::column(if reverse.get() {
                    vec![right, left]
                } else {
                    vec![left, right]
                })
            }
        })
        .unwrap();
    unsafe {
        let first = find(native, "First local key");
        let second = find(native, "Second local key");
        let a: Retained<NSString> = msg_send![&*first, accessibilityIdentifier];
        let b: Retained<NSString> = msg_send![&*second, accessibilityIdentifier];
        assert_ne!(
            a.to_string(),
            b.to_string(),
            "local application keys are not global native identities"
        );
        reverse.set(true);
        let again = find(native, "First local key");
        assert!(std::ptr::eq(&*first, &*again));
    }
    let weak = objc2::rc::autoreleasepool(|_| {
        let element = find(native, "First local key");
        objc2::rc::Weak::from_retained(&element)
    });
    assert!(
        weak.load().is_some(),
        "mounted nodes own their native elements beyond temporary queries"
    );
    let text = State::new("A🦀e\u{301}Z".to_owned());
    let host = window
        .set_view({
            let text = text.clone();
            move || {
                View::column(vec![
                    View::text_field("", text.binding())
                        .accessible_label("Screen field")
                        .key("field"),
                ])
                .padding(24.)
            }
        })
        .unwrap();
    window
        .set_position(scars_window::Point::new(180., 160.))
        .unwrap();
    unsafe {
        let field = find(native, "Screen field");
        let before: NSRect = msg_send![&*field, accessibilityFrame];
        let node = host
            .snapshot()
            .unwrap()
            .into_iter()
            .find(|n| n.key.as_deref() == Some("field"))
            .unwrap();
        near(before.size.width, node.clip.width);
        near(before.size.height, node.clip.height);
        let local =
            native.convertRect_fromView(window.native_window().convertRectFromScreen(before), None);
        near(local.origin.x, node.clip.x);
        near(local.origin.y, node.clip.y);
        let old = window.position();
        window
            .set_position(scars_window::Point::new(old.x + 37., old.y + 29.))
            .unwrap();
        let new = window.position();
        let moved: NSRect = msg_send![&*field, accessibilityFrame];
        near(moved.origin.x - before.origin.x, new.x - old.x);
        near(moved.origin.y - before.origin.y, new.y - old.y);
        window
            .set_content_size(scars_window::Size::new(710., 560.))
            .unwrap();
        let resized: NSRect = msg_send![&*field, accessibilityFrame];
        assert!(resized.size.width > moved.size.width);
        let point = NSPoint::new(
            resized.origin.x + resized.size.width / 2.,
            resized.origin.y + resized.size.height / 2.,
        );
        let hit: Retained<AnyObject> = msg_send![native,accessibilityHitTest:point];
        assert!(std::ptr::eq(&*hit, &*field));
        println!(
            "Native frame round trip passed at actual backing scale {}",
            window.native_window().backingScaleFactor()
        );
        let range: NSRect = msg_send![&*field,accessibilityFrameForRange:NSRange::new(1,2)];
        let node = host
            .snapshot()
            .unwrap()
            .into_iter()
            .find(|n| n.key.as_deref() == Some("field"))
            .unwrap();
        let glyph = host.text_rectangles(node.id, 1..5)[0];
        let expected = window
            .native_window()
            .convertRectToScreen(native.convertRect_toView(
                NSRect::new(
                    NSPoint::new(glyph.x, glyph.y),
                    NSSize::new(glyph.width, glyph.height),
                ),
                None,
            ));
        near(range.origin.x, expected.origin.x);
        near(range.origin.y, expected.origin.y);
        near(range.size.width, expected.size.width);
        near(range.size.height, expected.size.height);
        let invalid: NSRect = msg_send![&*field,accessibilityFrameForRange:NSRange::new(2,1)];
        assert_eq!(invalid, NSRect::ZERO);
        window.clear_view();
        let live: bool = msg_send![&*field, isAccessibilityElement];
        assert!(!live);
        let frame: NSRect = msg_send![&*field, accessibilityFrame];
        assert_eq!(frame, NSRect::ZERO);
        let _: () = msg_send![&*field,setAccessibilityValue:&*NSString::from_str("After unmount")];
        assert_eq!(text.get(), "A🦀e\u{301}Z");
        drop(host);
    }
    struct Resource(Rc<Cell<bool>>);
    impl Drop for Resource {
        fn drop(&mut self) {
            self.0.set(true)
        }
    }
    let dropped = Rc::new(Cell::new(false));
    let resource = Resource(dropped.clone());
    let button = View::button("Owned action", move || {
        let _ = &resource;
    });
    let host = window.set_view(move || button.clone()).unwrap();
    unsafe {
        let element = find(native, "Owned action");
        window.clear_view();
        drop(host);
        assert!(
            dropped.get(),
            "retained native proxies must not retain application callbacks"
        );
        let pressed: bool = msg_send![&*element, accessibilityPerformPress];
        assert!(!pressed);
    }
    let calls = State::new(0usize);
    let host = window
        .set_view({
            let calls = calls.clone();
            move || View::button("Close action", calls.action(|n| n.update(|v| *v += 1)))
        })
        .unwrap();
    unsafe {
        let element = find(native, "Close action");
        window.close();
        let live: bool = msg_send![&*element, isAccessibilityElement];
        assert!(!live);
        let pressed: bool = msg_send![&*element, accessibilityPerformPress];
        assert!(!pressed);
        assert_eq!(calls.get(), 0);
        drop(host);
    }
    app.quit_after(std::time::Duration::from_millis(100));
    app.run().unwrap();
    println!("Native identity, scoped hit testing, coordinates and teardown passed");
}
#[cfg(not(target_os = "macos"))]
fn main() {}
