//! Opt-in main-thread integration test of the mounted component bridge.
#[cfg(target_os = "macos")]
fn main() {
    if std::env::var_os("SCARS_WINDOW_DESKTOP_TESTS").is_none() {
        println!("Component desktop tests skipped");
        return;
    }
    use foundation::{NSArray, NSRange, NSString};
    use objc2::{msg_send, rc::Retained, runtime::AnyObject};
    use scars_window::{
        ui::{self, State, view},
        *,
    };
    let app = Application::named("Component tests").unwrap();
    let text = State::new(String::new());
    let checked = State::new(false);
    let window = Window::builder("Components")
        .view({
            let (text, checked) = (text.clone(), checked.clone());
            move || {
                view! {
                    column(padding: 20) {
                        text_field("Name", bind: text, key: "name");
                        checkbox("Enabled", bind: checked, key: "check");
                    }
                }
            }
        })
        .build(&app)
        .unwrap();
    let host = window.view_host().unwrap();
    let field = host
        .snapshot()
        .unwrap()
        .into_iter()
        .find(|n| n.kind == ui::Kind::TextField)
        .unwrap()
        .id;
    host.focus(field).unwrap();
    host.set_active(true);
    let native = window.native_view();
    // SAFETY: These are AppKit's documented accessibility selectors on our content view/elements.
    unsafe {
        let children: Retained<NSArray<AnyObject>> = msg_send![native, accessibilityChildren];
        assert_eq!(children.len(), 2);
        let check = children.objectAtIndex(1);
        let acted: bool = msg_send![&*check, accessibilityPerformPress];
        assert!(acted);
        assert!(checked.get());
        let field_ax = children.objectAtIndex(0);
        let label: Retained<NSString> = msg_send![&*field_ax, accessibilityLabel];
        assert_eq!(label.to_string(), "Name");
        host.focus(field).unwrap();
        host.set_active(true);
        let value = NSString::from_str("A😀B");
        let _: () = msg_send![native, insertText: &*value, replacementRange: NSRange::new(isize::MAX as usize,0)];
        assert_eq!(text.get(), "A😀B");
        let selected: NSRange = msg_send![native, selectedRange];
        assert_eq!(selected, NSRange::new(4, 0));
        let marked = NSString::from_str("日本");
        let _: () = msg_send![native, setMarkedText: &*marked, selectedRange: NSRange::new(2,0), replacementRange: NSRange::new(isize::MAX as usize,0)];
        assert_eq!(text.get(), "A😀B");
        let has_marked: bool = msg_send![native, hasMarkedText];
        assert!(has_marked);
        let _: () = msg_send![native, unmarkText];
        assert_eq!(text.get(), "A😀B日本");
        window.clear_view();
        let acted: bool = msg_send![&*check, accessibilityPerformPress];
        assert!(!acted);
    }
    // Unmount must end a stationary selection drag and its redraw deadline.
    let drag_text = State::new(
        (0..80)
            .map(|i| format!("Line {i} words here\n"))
            .collect::<String>(),
    );
    let drag_host = window
        .set_view(move || view! {text_editor("",bind:drag_text,key:"drag",height:80);})
        .unwrap();
    drag_host.set_active(true);
    let renderer = ui::Renderer::default();
    drag_host
        .layout(ui::Size::new(240., 80.), &renderer)
        .unwrap();
    for (kind, y) in [(ui::PointerKind::Down, 20.), (ui::PointerKind::Move, 110.)] {
        drag_host
            .pointer(ui::Pointer {
                kind,
                position: ui::Point { x: 50., y },
                button: 0,
                modifiers: Default::default(),
            })
            .unwrap();
    }
    drag_host
        .layout(ui::Size::new(240., 80.), &renderer)
        .unwrap();
    assert!(drag_host.next_deadline().is_some());
    window.clear_view();
    assert!(drag_host.focused().is_none());
    assert!(drag_host.next_deadline().is_none());
    // Invalid replacement must preserve the existing mount.
    window.set_view(|| view! { label("retained"); }).unwrap();
    assert!(
        window
            .set_view(|| view! { label("a",key:"same"); label("b",key:"same"); })
            .is_err()
    );
    assert!(
        window
            .view_host()
            .unwrap()
            .snapshot()
            .unwrap()
            .iter()
            .any(|n| n.text == "retained")
    );
    let secret = State::new("do not expose".to_owned());
    let secure_host = window
        .set_view(move || view! {password_field("Secret",bind:secret);})
        .unwrap();
    let id = secure_host.snapshot().unwrap().last().unwrap().id;
    secure_host.focus(id).unwrap();
    secure_host.set_active(true);
    unsafe {
        let children: Retained<NSArray<AnyObject>> = msg_send![native, accessibilityChildren];
        let element = children.objectAtIndex(0);
        let value: Option<Retained<AnyObject>> = msg_send![&*element, accessibilityValue];
        assert!(value.is_none());
        let value: Option<Retained<NSString>> =
            msg_send![&*element,accessibilityStringForRange:NSRange::new(0,3)];
        assert!(value.is_none());
    }
    let list = ui::ListModel::new(
        (0..100_000)
            .map(|i| ui::ListItem::new(i.to_string(), format!("Row {i}")))
            .collect(),
        32.,
    );
    let list_host = window
        .set_view({
            let list = list.clone();
            move || list.view()
        })
        .unwrap();
    unsafe {
        let children: Retained<NSArray<AnyObject>> = msg_send![native, accessibilityChildren];
        let container = children.objectAtIndex(0);
        let attribute = NSString::from_str("AXChildren");
        let count: usize = msg_send![&*container,accessibilityArrayAttributeCount:&*attribute];
        assert_eq!(count, 100_000);
        let rows: Retained<NSArray<AnyObject>> = msg_send![&*container,accessibilityArrayAttributeValues:&*attribute,index:99_998usize,maxCount:10usize];
        assert_eq!(rows.len(), 2);
        let row = rows.objectAtIndex(1);
        let label: Retained<NSString> = msg_send![&*row, accessibilityLabel];
        assert_eq!(label.to_string(), "Row 99999");
        let pressed: bool = msg_send![&*row, accessibilityPerformPress];
        assert!(pressed);
        assert!(list.selected().contains("99999"));
        assert!(list_host.snapshot().unwrap().len() < 100);
        window.clear_view();
        let pressed: bool = msg_send![&*row, accessibilityPerformPress];
        assert!(!pressed);
    }
    window.show();
    app.quit_after(std::time::Duration::from_millis(200));
    app.run().unwrap();
    println!("Component native input, accessibility and replacement tests passed");
}
#[cfg(not(target_os = "macos"))]
fn main() {}
