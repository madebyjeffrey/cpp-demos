//! Hidden AppKit time composition, exact values and incrementor contracts.
#[cfg(target_os = "macos")]
fn main() {
    if std::env::var_os("SCARS_WINDOW_DESKTOP_TESTS").is_none() {
        println!("Native time tests skipped");
        return;
    }
    use foundation::{NSArray, NSNumber, NSRange, NSString};
    use objc2::{Message, msg_send, rc::Retained, runtime::AnyObject};
    use scars_window::ui::{Key, KeyEvent};
    use scars_window::{ui::*, *};
    fn time(s: &str) -> Time {
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
        name: "Time contracts".into(),
        quit_on_last_window_close: false,
        ..Default::default()
    })
    .unwrap();
    let value = State::new(time("12:30:05.123456789"));
    let picker = TimePicker::new(value.binding())
        .show_seconds(true)
        .minute_step(5)
        .bounds(time("09:00"), time("17:00"));
    let window = Window::builder("Hidden time contracts")
        .size(640., 440.)
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
        let field = find(native, "Time · HH:MM:SS[.fraction]", "AXTextField");
        let text: Retained<NSString> = msg_send![&*field, accessibilityValue];
        assert_eq!(text.to_string(), "12:30:05.123456789");
        let step = find(native, "Minute", "AXIncrementor");
        let number: Retained<NSNumber> = msg_send![&*step, accessibilityValue];
        assert_eq!(number.as_i64(), 30);
        let changed: bool = msg_send![&*step, accessibilityPerformIncrement];
        assert!(changed);
        assert_eq!(value.get(), time("12:35:05.123456789"));
        let invalid = NSString::from_str("18:00");
        let _: () = msg_send![&*field,setAccessibilityValue:&*invalid];
        let changed: bool = msg_send![&*step, accessibilityPerformIncrement];
        assert!(!changed);
        assert_eq!(value.get(), time("12:35:05.123456789"));
        let text: Retained<NSString> = msg_send![&*field, accessibilityValue];
        assert_eq!(text.to_string(), "18:00");
        let prefix = NSString::from_str("12:30:0");
        let _: () = msg_send![&*field,setAccessibilityValue:&*prefix];
        let marked = NSString::from_str("3");
        let _: () = msg_send![native,setMarkedText:&*marked,selectedRange:NSRange::new(1,0),replacementRange:NSRange::new(7,0)];
        let _: () = msg_send![native,doCommandBySelector:objc2::sel!(insertNewline:)];
        assert_eq!(
            value.get(),
            time("12:35:05.123456789"),
            "confirming preedit does not submit"
        );
        let _: () = msg_send![native,doCommandBySelector:objc2::sel!(insertNewline:)];
        assert_eq!(value.get(), time("12:30:03"));
        let minute = host
            .snapshot()
            .unwrap()
            .into_iter()
            .find(|n| n.kind == Kind::Stepper && n.label == "Minute")
            .unwrap()
            .id;
        host.focus(minute).unwrap();
        host.key(KeyEvent {
            key: Key::Up,
            down: true,
            repeat: false,
            modifiers: Default::default(),
        })
        .unwrap();
        assert_eq!(value.get(), time("12:35:03"));
        window.clear_view();
        let changed: bool = msg_send![&*step, accessibilityPerformIncrement];
        assert!(!changed, "retired controls cannot mutate the source");
        let picker = TimePicker::new(value.binding())
            .hour_cycle(HourCycle::H12)
            .read_only(true);
        window.set_view(move || picker.view()).unwrap();
        let field = find(native, "Time · HH:MM AM/PM", "AXTextField");
        let text: Retained<NSString> = msg_send![&*field, accessibilityValue];
        assert_eq!(text.to_string(), "12:35 PM");
        let replacement = NSString::from_str("01:00 PM");
        let _: () = msg_send![&*field,setAccessibilityValue:&*replacement];
        let hour = find(native, "Hour", "AXIncrementor");
        let changed: bool = msg_send![&*hour, accessibilityPerformIncrement];
        assert!(!changed);
        assert_eq!(value.get(), time("12:35:03"));
    }
    assert!(!window.native_window().isVisible());
    window.close();
    println!(
        "Hidden native time values, steps, validation, composition, readonly and teardown passed"
    );
}
#[cfg(not(target_os = "macos"))]
fn main() {}
