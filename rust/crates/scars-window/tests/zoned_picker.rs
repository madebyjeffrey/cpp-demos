//! Hidden native zoned text, occurrence, zone, composition and teardown contracts.
#[cfg(target_os = "macos")]
fn main() {
    if std::env::var_os("SCARS_WINDOW_DESKTOP_TESTS").is_none() {
        println!("Native zoned tests skipped");
        return;
    }
    use foundation::{NSArray, NSRange, NSString};
    use objc2::{Message, msg_send, rc::Retained, runtime::AnyObject};
    use scars_window::{ui::*, *};
    fn ts(s: &str) -> Timestamp {
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
    fn press(o: &AnyObject) {
        let changed: bool = unsafe { msg_send![o, accessibilityPerformPress] };
        assert!(changed);
    }
    let app = Application::new(ApplicationOptions {
        name: "Zoned contracts".into(),
        quit_on_last_window_close: false,
        ..Default::default()
    })
    .unwrap();
    let value = State::new(ts("2024-11-03T06:30:05.123456789Z"));
    let picker = ZonedPicker::new(value.binding(), "America/Edmonton")
        .unwrap()
        .show_seconds(true)
        .bounds(
            ts("2024-01-01T00:00:00Z"),
            ts("2024-12-31T23:59:59.999999999Z"),
        );
    let window = Window::builder("Hidden zoned contracts")
        .size(760., 1000.)
        .view({
            let p = picker.clone();
            move || View::scroll(vec![p.view()])
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
        .find(|n| n.key.as_deref() == Some("$datetime-field"))
        .unwrap()
        .id;
    host.focus(id).unwrap();
    unsafe {
        let field = find(
            native,
            "Date and time · YYYY-MM-DDTHH:MM:SS[.fraction]",
            "AXTextField",
        );
        let fold = NSString::from_str("2024-11-03T01:30:05.123456789");
        let _: () = msg_send![&*field,setAccessibilityValue:&*fold];
        let _: () = msg_send![native,doCommandBySelector:objc2::sel!(insertNewline:)];
        assert_eq!(value.get(), ts("2024-11-03T06:30:05.123456789Z"));
        let help: Retained<NSString> = msg_send![&*field, accessibilityHelp];
        assert!(help.to_string().contains("twice"));
        press(&find(native, "Later", "AXRadioButton"));
        let _: () = msg_send![native,doCommandBySelector:objc2::sel!(insertNewline:)];
        assert_eq!(value.get(), ts("2024-11-03T08:30:05.123456789Z"));
        press(&find(native, "Earlier", "AXRadioButton"));
        press(&find(native, "Apply occurrence", "AXButton"));
        assert_eq!(value.get(), ts("2024-11-03T07:30:05.123456789Z"));
        let zone = find(native, "Time zone", "AXTextField");
        let target = NSString::from_str("Asia/Tokyo");
        let _: () = msg_send![&*zone,setAccessibilityValue:&*target];
        press(&find(native, "Change zone", "AXButton"));
        assert_eq!(value.get(), ts("2024-11-03T07:30:05.123456789Z"));
        assert_eq!(picker.time_zone().iana_name(), Some("Asia/Tokyo"));
        let again = find(
            native,
            "Date and time · YYYY-MM-DDTHH:MM:SS[.fraction]",
            "AXTextField",
        );
        assert!(
            std::ptr::eq(&*field, &*again),
            "zone and fold UI retain the text object"
        );
        let text: Retained<NSString> = msg_send![&*field, accessibilityValue];
        assert_eq!(text.to_string(), "2024-11-03T16:30:05.123456789");
        let invalid = NSString::from_str("Invalid/Zone");
        let _: () = msg_send![&*zone,setAccessibilityValue:&*invalid];
        press(&find(native, "Change zone", "AXButton"));
        assert_eq!(picker.time_zone().iana_name(), Some("Asia/Tokyo"));
        let text: Retained<NSString> = msg_send![&*zone, accessibilityValue];
        assert_eq!(text.to_string(), "Invalid/Zone");
        let help: Retained<NSString> = msg_send![&*zone, accessibilityHelp];
        assert!(
            !help.to_string().is_empty(),
            "zone errors are exposed to accessibility"
        );
        host.focus(id).unwrap();
        let prefix = NSString::from_str("2024-11-04T16:30:0");
        let _: () = msg_send![&*field,setAccessibilityValue:&*prefix];
        let marked = NSString::from_str("9");
        let _: () = msg_send![native,setMarkedText:&*marked,selectedRange:NSRange::new(1,0),replacementRange:NSRange::new(18,0)];
        let _: () = msg_send![native,doCommandBySelector:objc2::sel!(insertNewline:)];
        assert_eq!(value.get(), ts("2024-11-03T07:30:05.123456789Z"));
        let _: () = msg_send![native,doCommandBySelector:objc2::sel!(insertNewline:)];
        assert_eq!(value.get(), ts("2024-11-04T07:30:09Z"));
        let hour = find(native, "Hour", "AXIncrementor");
        window.clear_view();
        let changed: bool = msg_send![&*hour, accessibilityPerformIncrement];
        assert!(!changed);
        let p = picker.clone().read_only(true);
        window
            .set_view(move || View::scroll(vec![p.view()]))
            .unwrap();
        let later = find(native, "Later", "AXRadioButton");
        let changed: bool = msg_send![&*later, accessibilityPerformPress];
        assert!(!changed);
        let zone = find(native, "Time zone", "AXTextField");
        let _: () = msg_send![&*zone,setAccessibilityValue:&*target];
        assert_eq!(value.get(), ts("2024-11-04T07:30:09Z"));
    }
    assert!(!window.native_window().isVisible());
    window.close();
    println!(
        "Hidden native zoned drafts, occurrences, zones, retained field, composition, readonly and teardown passed"
    );
}
#[cfg(not(target_os = "macos"))]
fn main() {}
