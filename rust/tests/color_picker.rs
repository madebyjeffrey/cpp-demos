//! Hidden native color drafts, RGB/HSV, composition, popup rollback and teardown.
#[cfg(target_os = "macos")]
fn main() {
    if std::env::var_os("SCARS_WINDOW_DESKTOP_TESTS").is_none() {
        println!("Native color tests skipped");
        return;
    }
    use foundation::{NSArray, NSNumber, NSRange, NSString};
    use objc2::{Message, msg_send, rc::Retained, runtime::AnyObject};
    use scars_window::ui::Size;
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
    fn press(o: &AnyObject) {
        let changed: bool = unsafe { msg_send![o, accessibilityPerformPress] };
        assert!(changed);
    }
    fn sync(host: &Host) {
        host.layout(Size::new(700., 640.), &Renderer::default())
            .unwrap();
    }
    let app = Application::new(ApplicationOptions {
        name: "Color contracts".into(),
        quit_on_last_window_close: false,
        ..Default::default()
    })
    .unwrap();
    let value = State::new(Color::rgba(18, 52, 86, 120));
    let picker = ColorPicker::new(value.binding());
    let window = Window::builder("Hidden color contracts")
        .size(700., 640.)
        .view({
            let p = picker.clone();
            move || p.view()
        })
        .build(&app)
        .unwrap();
    let host = window.view_host().unwrap();
    host.set_active(true);
    let native = window.native_view();
    let hex_id = host
        .snapshot()
        .unwrap()
        .into_iter()
        .find(|n| n.key.as_deref() == Some("$color-hex"))
        .unwrap()
        .id;
    host.focus(hex_id).unwrap();
    unsafe {
        let _preview = find(native, "Color preview #12345678", "AXImage");
        let red = find(native, "Red value", "AXTextField");
        let typed = NSString::from_str("170");
        let _: () = msg_send![&*red,setAccessibilityValue:&*typed];
        let green = find(native, "Green", "AXSlider");
        let number = NSNumber::new_f64(200.);
        let _: () = msg_send![&*green,setAccessibilityValue:&*number];
        assert_eq!(value.get(), Color::rgba(18, 52, 86, 120));
        assert_eq!(picker.info().draft, Color::rgba(170, 200, 86, 120));
        press(&find(native, "Apply", "AXButton"));
        assert_eq!(value.get(), Color::rgba(170, 200, 86, 120));
        let hex = find(native, "Hex color", "AXTextField");
        press(&find(native, "HSV", "AXRadioButton"));
        let again = find(native, "Hex color", "AXTextField");
        assert!(std::ptr::eq(&*hex, &*again));
        let stale = NSString::from_str("1");
        let _: () = msg_send![&*red,setAccessibilityValue:&*stale];
        assert_eq!(
            value.get(),
            Color::rgba(170, 200, 86, 120),
            "retired RGB fields are inert"
        );
        host.focus(hex_id).unwrap();
        let prefix = NSString::from_str("#AABBCC");
        let _: () = msg_send![&*hex,setAccessibilityValue:&*prefix];
        let marked = NSString::from_str("DD");
        let _: () = msg_send![native,setMarkedText:&*marked,selectedRange:NSRange::new(2,0),replacementRange:NSRange::new(7,0)];
        let changed: bool = msg_send![
            &*find(native, "Apply", "AXButton"),
            accessibilityPerformPress
        ];
        assert!(!changed, "Apply cannot finalize composition");
        let _: () = msg_send![native,doCommandBySelector:objc2::sel!(insertNewline:)];
        assert_eq!(value.get(), Color::rgba(170, 200, 86, 120));
        let _: () = msg_send![native,doCommandBySelector:objc2::sel!(insertNewline:)];
        assert_eq!(value.get(), Color::rgba(170, 187, 204, 221));
    }
    let alpha_hidden = ColorPicker::new(value.binding()).alpha(false);
    let host = window
        .set_view({
            let p = alpha_hidden.clone();
            move || p.view()
        })
        .unwrap();
    host.set_active(true);
    unsafe {
        let hex = find(native, "Hex color", "AXTextField");
        let text: Retained<NSString> = msg_send![&*hex, accessibilityValue];
        assert_eq!(text.to_string(), "#AABBCC");
        let invalid = NSString::from_str("#010203FF");
        let _: () = msg_send![&*hex,setAccessibilityValue:&*invalid];
        let help: Retained<NSString> = msg_send![&*hex, accessibilityHelp];
        assert!(help.to_string().contains("Alpha"));
        assert!(!alpha_hidden.commit());
        assert_eq!(value.get().channels()[3], 221);
    }
    window.clear_view();
    let popup = Popup::new();
    let live = ColorPicker::new(value.binding()).live_preview(true);
    let host = window
        .set_view({
            let p = live.clone();
            let popup = popup.clone();
            move || {
                let (well, body) = p.well(&popup, "paint");
                View::layer(vec![well, body])
            }
        })
        .unwrap();
    host.set_active(true);
    let baseline = value.get();
    unsafe {
        let well = find(native, "Color", "AXColorWell");
        press(&well);
        sync(&host);
        let dialog = find(native, "Color", "AXPopover");
        let red = find(native, "Red", "AXSlider");
        let number = NSNumber::new_f64(42.);
        let _: () = msg_send![&*red,setAccessibilityValue:&*number];
        assert_eq!(value.get().channels()[0], 42);
        let changed: bool = msg_send![&*dialog, accessibilityPerformCancel];
        assert!(changed);
        sync(&host);
        assert_eq!(value.get(), baseline);
        assert!(!popup.open.get());
        press(&well);
        sync(&host);
        live.set_component(ColorChannel::Blue, 3.);
        sync(&host);
        press(&find(native, "Apply", "AXButton"));
        sync(&host);
        assert!(!popup.open.get());
        assert_eq!(value.get().channels()[2], 3);
        let accepted = value.get();
        press(&well);
        sync(&host);
        live.set_component(ColorChannel::Blue, 4.);
        sync(&host);
        window.clear_view();
        assert_eq!(
            value.get(),
            accepted,
            "native unmount rolls back a retained controller"
        );
    }
    let readonly = ColorPicker::new(value.binding()).read_only(true);
    window.set_view(move || readonly.view()).unwrap();
    unsafe {
        let field = find(native, "Hex color", "AXTextField");
        let changed = NSString::from_str("#FFFFFFFF");
        let before = value.get();
        let _: () = msg_send![&*field,setAccessibilityValue:&*changed];
        let red = find(native, "Red", "AXSlider");
        let number = NSNumber::new_f64(255.);
        let _: () = msg_send![&*red,setAccessibilityValue:&*number];
        assert_eq!(value.get(), before);
    }
    assert!(!window.native_window().isVisible());
    window.close();
    println!(
        "Hidden native RGB/HSV, alpha, fields/sliders, retained identity, composition, popup rollback, readonly and unmount passed"
    );
}
#[cfg(not(target_os = "macos"))]
fn main() {}
