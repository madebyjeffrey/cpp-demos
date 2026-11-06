//! Native text transport and AX values for explicitly localized controls.
#[cfg(target_os = "macos")]
fn main() {
    if std::env::var_os("SCARS_WINDOW_DESKTOP_TESTS").is_none() {
        println!("Native localized value tests skipped");
        return;
    }
    use foundation::{NSArray, NSNumber, NSString};
    use objc2::{Message, msg_send, rc::Retained, runtime::AnyObject};
    use scars_window::{ui::*, *};
    fn find(o: &AnyObject, label: &str, role: &str) -> Retained<AnyObject> {
        fn search(o: &AnyObject, label: &str, role: &str) -> Option<Retained<AnyObject>> {
            unsafe {
                let name: Option<Retained<NSString>> = msg_send![o, accessibilityLabel];
                let kind: Option<Retained<NSString>> = msg_send![o, accessibilityRole];
                if name.is_some_and(|s| s.to_string() == label)
                    && kind.is_some_and(|s| s.to_string() == role)
                {
                    return Some(o.retain());
                }
                let children: Option<Retained<NSArray<AnyObject>>> =
                    msg_send![o, accessibilityChildren];
                children.and_then(|a| a.iter().find_map(|e| search(&e, label, role)))
            }
        }
        search(o, label, role).unwrap_or_else(|| panic!("Missing {role}: {label}"))
    }
    let app = Application::new(ApplicationOptions {
        name: "Localized values".into(),
        quit_on_last_window_close: false,
        ..Default::default()
    })
    .unwrap();
    let window = Window::builder("Localized value contracts")
        .size(680., 620.)
        .build(&app)
        .unwrap();
    let native = window.native_view();
    let exact = State::new(9_007_199_254_740_993i64);
    let number = Numeric::new(exact.binding(), i64::MIN, i64::MAX, 1)
        .locale(EditLocale::new("de-DE").unwrap());
    let amount = State::new(1234.5);
    let arabic =
        Numeric::new(amount.binding(), -1e10, 1e10, 0.5).locale(EditLocale::new("ar-EG").unwrap());
    let date = State::new(Date::new(2024, 2, 29).unwrap());
    let picker = DatePicker::new(date.binding()).locale(EditLocale::new("de-DE").unwrap());
    let host = window
        .set_view({
            let number = number.clone();
            let arabic = arabic.clone();
            move || {
                View::column(vec![
                    View::numeric_field("Ganzzahl", &number),
                    View::numeric_field("المبلغ", &arabic),
                    picker.view(),
                ])
            }
        })
        .unwrap();
    host.set_active(true);
    unsafe {
        let field = find(native, "Ganzzahl", "AXTextField");
        let value: Retained<NSString> = msg_send![&*field, accessibilityValue];
        assert_eq!(value.to_string(), "9.007.199.254.740.993");
        let help: Retained<NSString> = msg_send![&*field, accessibilityHelp];
        assert!(help.to_string().contains("1.234,5"));
        let step = find(native, "Ganzzahl", "AXIncrementor");
        let exact_value: Retained<NSNumber> = msg_send![&*step, accessibilityValue];
        assert_eq!(exact_value.as_i64(), 9_007_199_254_740_993);
        let _: () = msg_send![&*field,setAccessibilityFocused:true];
        let draft = NSString::from_str("9.007.199.254.740.994");
        let _: () = msg_send![&*field,setAccessibilityValue:&*draft];
        assert_eq!(exact.get(), 9_007_199_254_740_993, "editing stages a draft");
        let changed: bool = msg_send![&*step, accessibilityPerformIncrement];
        assert!(changed);
        assert_eq!(exact.get(), 9_007_199_254_740_995);
        let field = find(native, "المبلغ", "AXTextField");
        let value: Retained<NSString> = msg_send![&*field, accessibilityValue];
        assert_eq!(value.to_string(), "١٬٢٣٤٫٥");
        let _: () = msg_send![&*field,setAccessibilityFocused:true];
        let draft = NSString::from_str("-١٬٢٣٤٫٥");
        let _: () = msg_send![&*field,setAccessibilityValue:&*draft];
        let _: () = msg_send![native,doCommandBySelector:objc2::sel!(insertNewline:)];
        assert_eq!(amount.get(), -1234.5);
        let committed: Retained<NSString> = msg_send![&*field, accessibilityValue];
        assert!(committed.to_string().contains("-١٬٢٣٤٫٥")); // ICU's exact sign affix may include an ALM.
        let step = find(native, "المبلغ", "AXIncrementor");
        let value: Retained<NSNumber> = msg_send![&*step, accessibilityValue];
        assert_eq!(value.as_f64(), -1234.5);
        let field = find(native, "Datum · TT.MM.JJJJ", "AXTextField");
        let _: () = msg_send![&*field,setAccessibilityFocused:true];
        let draft = NSString::from_str("04.03.2026");
        let _: () = msg_send![&*field,setAccessibilityValue:&*draft];
        let _: () = msg_send![native,doCommandBySelector:objc2::sel!(insertNewline:)];
        assert_eq!(date.get(), Date::new(2026, 3, 4).unwrap());
        let draft = NSString::from_str("2026-03-04T12:00:00Z");
        let _: () = msg_send![&*field,setAccessibilityValue:&*draft];
        let _: () = msg_send![native,doCommandBySelector:objc2::sel!(insertNewline:)];
        assert_eq!(date.get(), Date::new(2026, 3, 4).unwrap());
    }
    window.close();
    println!(
        "Native localized text edits, exact AX numeric values, stepping and date validation passed"
    );
}
#[cfg(not(target_os = "macos"))]
fn main() {}
