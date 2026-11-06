//! Hidden-window date keyboard, text composition and accessibility contracts.
#[cfg(target_os = "macos")]
fn main() {
    if std::env::var_os("SCARS_WINDOW_DESKTOP_TESTS").is_none() {
        println!("Native date tests skipped");
        return;
    }
    use foundation::{NSArray, NSRange, NSString};
    use objc2::{Message, msg_send, rc::Retained, runtime::AnyObject};
    use scars_window::ui::{Key, KeyEvent};
    use scars_window::{ui::*, *};
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
        name: "Date protocol tests".into(),
        quit_on_last_window_close: false,
        ..Default::default()
    })
    .unwrap();
    let value = State::new(Date::new(2024, 1, 31).unwrap());
    let picker = DatePicker::new(value.binding())
        .bounds(
            Date::new(2024, 1, 1).unwrap(),
            Date::new(2024, 12, 31).unwrap(),
        )
        .disabled_dates(|d| d.day() == 29);
    let window = Window::builder("Hidden date contracts")
        .size(640., 640.)
        .view({
            let p = picker.clone();
            move || p.view()
        })
        .build(&app)
        .unwrap();
    let host = window.view_host().unwrap();
    host.set_active(true);
    let native = window.native_view();
    let day = host
        .snapshot()
        .unwrap()
        .into_iter()
        .find(|n| n.calendar_day.is_some_and(|d| d.current))
        .unwrap();
    host.focus(day.id).unwrap();
    host.key(KeyEvent {
        key: Key::PageDown,
        down: true,
        repeat: false,
        modifiers: Default::default(),
    })
    .unwrap();
    unsafe {
        let disabled: Retained<AnyObject> = msg_send![native, accessibilityFocusedUIElement];
        let label: Retained<NSString> = msg_send![&*disabled, accessibilityLabel];
        assert_eq!(label.to_string(), "2024-02-29");
        let enabled: bool = msg_send![&*disabled, isAccessibilityEnabled];
        assert!(!enabled);
        let press: bool = msg_send![&*disabled, accessibilityPerformPress];
        assert!(!press);
        assert_eq!(value.get(), Date::new(2024, 1, 31).unwrap());
        host.key(KeyEvent {
            key: Key::Right,
            down: true,
            repeat: false,
            modifiers: Default::default(),
        })
        .unwrap();
        let current: Retained<AnyObject> = msg_send![native, accessibilityFocusedUIElement];
        let press: bool = msg_send![&*current, accessibilityPerformPress];
        assert!(press);
        assert_eq!(value.get(), Date::new(2024, 3, 1).unwrap());
        let selected: bool = msg_send![&*current, isAccessibilitySelected];
        assert!(selected);
        let group = find(native, "Date · YYYY-MM-DD", "AXGroup");
        let children: Retained<NSArray<AnyObject>> =
            msg_send![&*group, accessibilitySelectedChildren];
        assert_eq!(children.len(), 1);
        assert!(std::ptr::eq(&*current, &*children.objectAtIndex(0)));
        let field = find(native, "Date · YYYY-MM-DD", "AXTextField");
        let id = host
            .snapshot()
            .unwrap()
            .into_iter()
            .find(|n| n.kind == Kind::TextField)
            .unwrap()
            .id;
        host.focus(id).unwrap();
        let draft = NSString::from_str("2024-04-15");
        let _: () = msg_send![&*field,setAccessibilityValue:&*draft];
        let _: () = msg_send![native,doCommandBySelector:objc2::sel!(insertNewline:)];
        assert_eq!(value.get(), Date::new(2024, 4, 15).unwrap());
        let again = find(native, "Date · YYYY-MM-DD", "AXTextField");
        assert!(
            std::ptr::eq(&*field, &*again),
            "month changes preserve the field object"
        );
        let invalid = NSString::from_str("2024-04-29");
        let _: () = msg_send![&*field,setAccessibilityValue:&*invalid];
        let _: () = msg_send![native,doCommandBySelector:objc2::sel!(insertNewline:)];
        assert_eq!(value.get(), Date::new(2024, 4, 15).unwrap());
        let help: Retained<NSString> = msg_send![&*field, accessibilityHelp];
        assert!(help.to_string().contains("unavailable"));
        window.clear_view();
        let stale: bool = msg_send![&*current, accessibilityPerformPress];
        assert!(!stale);
    }
    // Real NSTextInputClient composition transport: Return first commits preedit.
    let picker = DatePicker::new(value.binding());
    let host = window.set_view(move || picker.view()).unwrap();
    host.set_active(true);
    let id = host
        .snapshot()
        .unwrap()
        .into_iter()
        .find(|n| n.kind == Kind::TextField)
        .unwrap()
        .id;
    host.focus(id).unwrap();
    unsafe {
        let field = find(native, "Date · YYYY-MM-DD", "AXTextField");
        let prefix = NSString::from_str("2024-02-2");
        let _: () = msg_send![&*field,setAccessibilityValue:&*prefix];
        let marked = NSString::from_str("9");
        let _: () = msg_send![native,setMarkedText:&*marked,selectedRange:NSRange::new(1,0),replacementRange:NSRange::new(9,0)];
        let _: () = msg_send![native,doCommandBySelector:objc2::sel!(insertNewline:)];
        assert_eq!(value.get(), Date::new(2024, 4, 15).unwrap());
        let _: () = msg_send![native,doCommandBySelector:objc2::sel!(insertNewline:)];
        assert_eq!(value.get(), Date::new(2024, 2, 29).unwrap());
    }
    assert!(!window.native_window().isVisible());
    window.close();
    println!(
        "Hidden native date navigation, selection, validation, retained field, composition and teardown passed"
    );
}
#[cfg(not(target_os = "macos"))]
fn main() {}
