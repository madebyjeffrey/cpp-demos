//! Hidden AppKit outline selection and retired inspector-row actions.
#[cfg(target_os = "macos")]
fn main() {
    if std::env::var_os("SCARS_WINDOW_DESKTOP_TESTS").is_none() {
        println!("Native inspector tests skipped");
        return;
    }
    use foundation::{NSArray, NSString};
    use objc2::{Message, msg_send, rc::Retained, runtime::AnyObject};
    use scars_window::ui::Size;
    use scars_window::{ui::*, *};
    fn find(o: &AnyObject, label: &str, role: &str) -> Option<Retained<AnyObject>> {
        unsafe {
            let name: Option<Retained<NSString>> = msg_send![o, accessibilityLabel];
            let kind: Option<Retained<NSString>> = msg_send![o, accessibilityRole];
            if name.is_some_and(|n| n.to_string() == label)
                && kind.is_some_and(|n| n.to_string() == role)
            {
                return Some(o.retain());
            }
            let kids: Option<Retained<NSArray<AnyObject>>> = msg_send![o, accessibilityChildren];
            kids.and_then(|a| a.iter().find_map(|c| find(&c, label, role)))
        }
    }
    fn sync(h: &Host) {
        h.layout(Size::new(1000., 860.), &Renderer::default())
            .unwrap();
    }
    let app = Application::new(ApplicationOptions {
        name: "Inspector contracts".into(),
        quit_on_last_window_close: false,
        ..Default::default()
    })
    .unwrap();
    let inspector = Inspector::new();
    let visible = State::new(true);
    let clicks = State::new(0);
    let window = Window::builder("Hidden inspector")
        .size(1000., 860.)
        .view({
            let (i, v, c) = (inspector.clone(), visible.clone(), clicks.clone());
            move || {
                View::row(vec![
                    View::column(if v.get() {
                        vec![View::button("Save", c.action(|s| s.set(1))).key("save")]
                    } else {
                        vec![]
                    })
                    .grow(1.),
                    i.view().width(380.),
                ])
            }
        })
        .build(&app)
        .unwrap();
    let host = window.view_host().unwrap();
    host.set_inspector(inspector.clone());
    host.set_active(true);
    sync(&host);
    let target = host
        .snapshot()
        .unwrap()
        .into_iter()
        .find(|n| n.key.as_deref() == Some("save"))
        .unwrap()
        .id;
    inspector.select(target);
    sync(&host);
    let native = window.native_view();
    unsafe {
        let outline = find(native, "Mounted hierarchy", "AXOutline").unwrap();
        let selected: Retained<NSArray<AnyObject>> =
            msg_send![&*outline, accessibilitySelectedRows];
        assert_eq!(selected.len(), 1);
        let row = selected.objectAtIndex(0);
        let label: Retained<NSString> = msg_send![&*row, accessibilityLabel];
        assert!(label.to_string().contains("save"));
        let rows: Retained<NSArray<AnyObject>> = msg_send![&*outline, accessibilityRows];
        let parent = rows
            .iter()
            .find(|r| {
                let name: Option<Retained<NSString>> = msg_send![&**r, accessibilityLabel];
                name.is_some_and(|n| n.to_string().starts_with("Column #"))
            })
            .unwrap();
        let selected = NSArray::from_retained_slice(&[parent]);
        let _: () = msg_send![&*outline,setAccessibilitySelectedRows:&*selected];
        assert_ne!(
            inspector.selected.get().unwrap().id,
            target,
            "AX selection is independent of source layout"
        );
        sync(&host);
        assert_ne!(inspector.selected.get().unwrap().id, target);
        assert_eq!(clicks.get(), 0);
        inspector.select(target);
        sync(&host);
        inspector.show_page(InspectorPage::Layout);
        sync(&host);
        let field = find(native, "Override width", "AXTextField").unwrap();
        let value = NSString::from_str("150");
        let _: () = msg_send![&*field, setAccessibilityValue: &*value];
        sync(&host);
        let apply = find(native, "Apply width", "AXButton").unwrap();
        let pressed: bool = msg_send![&*apply, accessibilityPerformPress];
        assert!(pressed);
        sync(&host);
        assert_eq!(
            inspector.style_override(target).width,
            Some(Length::Points(150.))
        );
        assert_eq!(inspector.selected.get().unwrap().bounds.width, 150.);
        let reset = find(native, "Reset width", "AXButton").unwrap();
        let pressed: bool = msg_send![&*reset, accessibilityPerformPress];
        assert!(pressed);
        sync(&host);
        assert!(inspector.style_override(target).is_empty());
        assert_eq!(clicks.get(), 0, "editing must never activate the source");
        visible.set(false);
        sync(&host);
        assert!(inspector.selected.get().is_none());
        let pressed: bool = msg_send![&*row, accessibilityPerformPress];
        assert!(!pressed, "retired inspector rows must be inert");
        assert_eq!(clicks.get(), 0);
    }
    assert!(!window.native_window().isVisible());
    window.close();
    println!("Hidden inspector selection, property edits/reset and retirement passed");
}
#[cfg(not(target_os = "macos"))]
fn main() {}
