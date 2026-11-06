//! Hidden AppKit joint-draft, calendar, clock and composition contracts.
#[cfg(target_os = "macos")]
fn main() {
    if std::env::var_os("SCARS_WINDOW_DESKTOP_TESTS").is_none() {
        println!("Native datetime tests skipped");
        return;
    }
    use foundation::{NSArray, NSNumber, NSRange, NSString};
    use objc2::{Message, msg_send, rc::Retained, runtime::AnyObject};
    use scars_window::{ui::*, *};
    fn dt(s: &str) -> DateTime {
        s.parse().unwrap()
    }
    fn find(o: &AnyObject, label: &str, role: &str) -> Retained<AnyObject> {
        fn search(o: &AnyObject, label: &str, role: &str) -> Option<Retained<AnyObject>> {
            unsafe {
                let name: Option<Retained<NSString>> = msg_send![o, accessibilityLabel];
                let kind: Option<Retained<NSString>> = msg_send![o, accessibilityRole];
                if name.is_some_and(|n| n.to_string() == label)
                    && kind.is_some_and(|n| n.to_string() == role)
                {
                    return Some(o.retain());
                }
                let children: Option<Retained<NSArray<AnyObject>>> =
                    msg_send![o, accessibilityChildren];
                children.and_then(|a| a.iter().find_map(|o| search(&o, label, role)))
            }
        }
        search(o, label, role).unwrap_or_else(|| panic!("Missing {role}: {label}"))
    }
    let app = Application::new(ApplicationOptions {
        name: "Datetime contracts".into(),
        quit_on_last_window_close: false,
        ..Default::default()
    })
    .unwrap();
    let source = State::new(dt("2026-11-02T23:00:05.123456789"));
    let picker = DateTimePicker::new(source.binding())
        .show_seconds(true)
        .bounds(dt("2026-11-02T22:00"), dt("2026-11-03T02:00"));
    let window = Window::builder("Hidden datetime contracts")
        .size(680., 700.)
        .view({
            let p = picker.clone();
            move || p.view()
        })
        .build(&app)
        .unwrap();
    let host = window.view_host().unwrap();
    host.set_active(true);
    let native = window.native_view();
    let id = host
        .snapshot()
        .unwrap()
        .into_iter()
        .find(|n| n.kind == Kind::TextField)
        .unwrap()
        .id;
    host.focus(id).unwrap();
    unsafe {
        let field = find(
            native,
            "Date and time · YYYY-MM-DDTHH:MM:SS[.fraction]",
            "AXTextField",
        );
        let hour = find(native, "Hour", "AXIncrementor");
        let changed: bool = msg_send![&*hour, accessibilityPerformIncrement];
        assert!(changed);
        assert_eq!(source.get(), dt("2026-11-03T00:00:05.123456789"));
        let value: Retained<NSNumber> = msg_send![&*hour, accessibilityValue];
        assert_eq!(value.as_i64(), 0);
        let invalid = NSString::from_str("2026-11-03T23:00");
        let _: () = msg_send![&*field,setAccessibilityValue:&*invalid];
        let _: () = msg_send![native,doCommandBySelector:objc2::sel!(insertNewline:)];
        assert_eq!(source.get(), dt("2026-11-03T00:00:05.123456789"));
        let text: Retained<NSString> = msg_send![&*field, accessibilityValue];
        assert_eq!(text.to_string(), "2026-11-03T23:00");
        let valid = NSString::from_str("2026-11-03T01:00");
        let _: () = msg_send![&*field,setAccessibilityValue:&*valid];
        assert_eq!(source.get(), dt("2026-11-03T00:00:05.123456789"));
        let _: () = msg_send![native,doCommandBySelector:objc2::sel!(insertNewline:)];
        assert_eq!(source.get(), dt("2026-11-03T01:00:05.123456789"));
        let again = find(
            native,
            "Date and time · YYYY-MM-DDTHH:MM:SS[.fraction]",
            "AXTextField",
        );
        assert!(std::ptr::eq(&*field, &*again));
        let unavailable = find(native, "2026-11-02", "AXButton");
        let enabled: bool = msg_send![&*unavailable, isAccessibilityEnabled];
        assert!(!enabled);
        let press: bool = msg_send![&*unavailable, accessibilityPerformPress];
        assert!(!press);
        let selected = find(native, "2026-11-03", "AXButton");
        let is_selected: bool = msg_send![&*selected, isAccessibilitySelected];
        assert!(is_selected);
        let prefix = NSString::from_str("2026-11-03T01:30:0");
        let _: () = msg_send![&*field,setAccessibilityValue:&*prefix];
        let marked = NSString::from_str("7");
        let _: () = msg_send![native,setMarkedText:&*marked,selectedRange:NSRange::new(1,0),replacementRange:NSRange::new(18,0)];
        let _: () = msg_send![native,doCommandBySelector:objc2::sel!(insertNewline:)];
        assert_eq!(source.get(), dt("2026-11-03T01:00:05.123456789"));
        let _: () = msg_send![native,doCommandBySelector:objc2::sel!(insertNewline:)];
        assert_eq!(source.get(), dt("2026-11-03T01:30:07"));
        window.clear_view();
        let changed: bool = msg_send![&*hour, accessibilityPerformDecrement];
        assert!(!changed);
        let picker = DateTimePicker::new(source.binding()).read_only(true);
        window.set_view(move || picker.view()).unwrap();
        let field = find(native, "Date and time · YYYY-MM-DDTHH:MM", "AXTextField");
        let replacement = NSString::from_str("2026-11-04T12:00");
        let _: () = msg_send![&*field,setAccessibilityValue:&*replacement];
        let hour = find(native, "Hour", "AXIncrementor");
        let changed: bool = msg_send![&*hour, accessibilityPerformIncrement];
        assert!(!changed);
        assert_eq!(source.get(), dt("2026-11-03T01:30:07"));
    }
    assert!(!window.native_window().isVisible());
    window.close();
    println!(
        "Hidden native datetime joint drafts, exact clock/calendar values, composition, readonly and teardown passed"
    );
}
#[cfg(not(target_os = "macos"))]
fn main() {}
