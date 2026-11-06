//! Opt-in main-thread protocol/action audit for controls and presentations.
#[cfg(target_os = "macos")]
fn main() {
    if std::env::var_os("SCARS_WINDOW_DESKTOP_TESTS").is_none() {
        println!("Native control semantics tests skipped");
        return;
    }
    use foundation::{NSArray, NSNumber, NSString};
    use objc2::{msg_send, rc::Retained, runtime::AnyObject};
    use scars_window::{ui::*, *};
    fn find(o: &AnyObject, label: &str) -> Retained<AnyObject> {
        fn search(o: &AnyObject, label: &str) -> Option<Retained<AnyObject>> {
            unsafe {
                let value: Option<Retained<NSString>> = msg_send![o, accessibilityLabel];
                let role: Option<Retained<NSString>> = msg_send![o, accessibilityRole];
                if value.is_some_and(|v| v.to_string() == label)
                    && role.is_some_and(|r| r.to_string() != "AXStaticText")
                {
                    use objc2::Message;
                    return Some(o.retain());
                }
                let children: Option<Retained<NSArray<AnyObject>>> =
                    msg_send![o, accessibilityChildren];
                children.and_then(|a| a.iter().find_map(|o| search(&o, label)))
            }
        }
        search(o, label).unwrap_or_else(|| panic!("Missing accessibility label {label}"))
    }
    fn role(o: &AnyObject, expected: &str) {
        unsafe {
            let role: Retained<NSString> = msg_send![o, accessibilityRole];
            assert_eq!(role.to_string(), expected);
            let description: Retained<NSString> = msg_send![o, accessibilityRoleDescription];
            assert!(!description.to_string().is_empty());
        }
    }
    let app = Application::named("Control semantics tests").unwrap();
    let window = Window::builder("Control contracts")
        .size(680., 620.)
        .build(&app)
        .unwrap();
    let native = window.native_view();
    let count = State::new(1i64);
    let number = Numeric::new(count.binding(), 0, 2, 1);
    let slider = State::new(2.);
    let check = State::new(CheckState::Indeterminate);
    let toggle = State::new(false);
    let text = State::new(String::new());
    window
        .set_view({
            let number = number.clone();
            let slider = slider.clone();
            let check = check.clone();
            let toggle = toggle.clone();
            let text = text.clone();
            move || {
                View::column(vec![
                    View::stepper("Count", &number),
                    View::numeric_field("Amount", &number),
                    View::slider(slider.binding(), 0., 10., 1.).accessible_label("Volume"),
                    View::tri_checkbox("Include", check.binding()),
                    View::toggle("Pinned", toggle.binding()),
                    View::text_editor("Notes", text.binding())
                        .accessible_label("Notes")
                        .accessible_help("Keep a short note")
                        .validation(Validation {
                            status: ValidationStatus::Incomplete,
                            message: Some("A note is required".into()),
                            dirty: true,
                        }),
                ])
            }
        })
        .unwrap();
    unsafe {
        let step = find(native, "Count");
        role(&step, "AXIncrementor");
        let up: Retained<AnyObject> = msg_send![&*step, accessibilityIncrementButton];
        let down: Retained<AnyObject> = msg_send![&*step, accessibilityDecrementButton];
        role(&up, "AXButton");
        role(&down, "AXButton");
        let allowed: bool = msg_send![&*step,isAccessibilitySelectorAllowed:objc2::sel!(accessibilityPerformIncrement)];
        assert!(allowed);
        let changed: bool = msg_send![&*step, accessibilityPerformIncrement];
        assert!(changed);
        assert_eq!(count.get(), 2);
        let allowed: bool = msg_send![&*step,isAccessibilitySelectorAllowed:objc2::sel!(accessibilityPerformIncrement)];
        assert!(!allowed);
        let changed: bool = msg_send![&*step, accessibilityPerformIncrement];
        assert!(!changed);
        let changed: bool = msg_send![&*step, accessibilityPerformDecrement];
        assert!(changed);
        assert_eq!(count.get(), 1);
        let volume = find(native, "Volume");
        role(&volume, "AXSlider");
        let _: () = msg_send![&*volume,setAccessibilityValue:&*NSNumber::new_f64(10.)];
        assert_eq!(slider.get(), 10.);
        let allowed: bool = msg_send![&*volume,isAccessibilitySelectorAllowed:objc2::sel!(accessibilityPerformIncrement)];
        assert!(!allowed);
        let include = find(native, "Include");
        role(&include, "AXCheckBox");
        let _: () = msg_send![&*include,setAccessibilityValue:&*NSNumber::new_i32(0)];
        assert_eq!(check.get(), CheckState::Unchecked);
        let _: () = msg_send![&*include,setAccessibilityValue:&*NSNumber::new_i32(2)];
        assert_eq!(check.get(), CheckState::Indeterminate);
        let pin = find(native, "Pinned");
        role(&pin, "AXButton");
        let subrole: Retained<NSString> = msg_send![&*pin, accessibilitySubrole];
        assert_eq!(subrole.to_string(), "AXToggle");
        let _: () = msg_send![&*pin,setAccessibilityValue:&*NSNumber::new_bool(true)];
        assert!(toggle.get());
        let notes = find(native, "Notes");
        role(&notes, "AXTextArea");
        let help: Retained<NSString> = msg_send![&*notes, accessibilityHelp];
        assert_eq!(help.to_string(), "A note is required. Keep a short note");
        let desc: Retained<NSString> = msg_send![&*notes, accessibilityValueDescription];
        assert_eq!(desc.to_string(), "Incomplete value");
        number.draft.binding().set("-".into());
        let amount = find(native, "Amount");
        let desc: Retained<NSString> = msg_send![&*amount, accessibilityValueDescription];
        assert_eq!(desc.to_string(), "Incomplete value");
        let changed: bool = msg_send![&*step, accessibilityPerformIncrement];
        assert!(!changed);
        assert_eq!(count.get(), 1);
        number.cancel();
    }
    let tabs = Tabs::new("a");
    let choice = State::new("x".to_owned());
    window
        .set_view({
            let tabs = tabs.clone();
            let choice = choice.clone();
            move || {
                View::column(vec![
                    tabs.view(vec![
                        Tab::new("a", "General", View::label("General page")),
                        Tab::new("b", "Advanced", View::label("Advanced page")),
                        Tab::new("off", "Unavailable", View::label("Hidden")).enabled(false),
                    ])
                    .accessible_label("Preferences"),
                    View::segmented(
                        choice.binding(),
                        vec![OptionItem::new("x", "One"), OptionItem::new("y", "Two")],
                    )
                    .accessible_label("Display"),
                ])
            }
        })
        .unwrap();
    unsafe {
        let group = find(native, "Preferences");
        role(&group, "AXTabGroup");
        let options: Retained<NSArray<AnyObject>> = msg_send![&*group, accessibilityTabs];
        assert_eq!(options.len(), 3);
        let attr = NSString::from_str("AXTabs");
        let count: usize = msg_send![&*group,accessibilityArrayAttributeCount:&*attr];
        assert_eq!(count, 3);
        let subset: Retained<NSArray<AnyObject>> = msg_send![&*group,accessibilityArrayAttributeValues:&*attr,index:1usize,maxCount:1usize];
        assert_eq!(subset.len(), 1);

        let a = options.objectAtIndex(0);
        let b = options.objectAtIndex(1);
        role(&b, "AXRadioButton");
        let selected: bool = msg_send![&*a, isAccessibilitySelected];
        assert!(selected);
        let desired = NSArray::from_slice(&[&*b]);
        let _: () = msg_send![&*group,setAccessibilitySelectedChildren:&*desired];
        assert_eq!(tabs.selected(), "b");
        let attr = NSString::from_str("AXSelectedChildren");
        let count: usize = msg_send![&*group,accessibilityArrayAttributeCount:&*attr];
        assert_eq!(count, 1);
        let subset: Retained<NSArray<AnyObject>> = msg_send![&*group,accessibilityArrayAttributeValues:&*attr,index:0usize,maxCount:1usize];
        assert!(std::ptr::eq(&*subset.objectAtIndex(0), &*b));

        let value: Retained<AnyObject> = msg_send![&*group, accessibilityValue];
        assert!(std::ptr::eq(&*value, &*b));
        let _: () = msg_send![&*group,setAccessibilityValue:&*a];
        assert_eq!(tabs.selected(), "a");
        let _: () = msg_send![&*a,setAccessibilitySelected:false];
        assert_eq!(tabs.selected(), "a");
        let off = options.objectAtIndex(2);
        let _: () = msg_send![&*group,setAccessibilityValue:&*off];
        assert_eq!(tabs.selected(), "a");
        let display = find(native, "Display");
        role(&display, "AXRadioGroup");
        let two = find(native, "Two");
        let desired = NSArray::from_slice(&[&*two]);
        let _: () = msg_send![&*display,setAccessibilitySelectedChildren:&*desired];
        assert_eq!(choice.get(), "y");
        let _: () = msg_send![&*group,setAccessibilitySelectedChildren:&*desired];
        assert_eq!(tabs.selected(), "a");
    }
    let split = Split::new(0.5).limits(60., 60.);
    window
        .set_view({
            let split = split.clone();
            move || {
                split
                    .view(View::label("Before"), View::label("After"))
                    .accessible_label("Workspace")
            }
        })
        .unwrap();
    unsafe {
        let group = find(native, "Workspace");
        role(&group, "AXSplitGroup");
        let dividers: Retained<NSArray<AnyObject>> = msg_send![&*group, accessibilitySplitters];
        let divider = dividers.objectAtIndex(0);
        let attr = NSString::from_str("AXSplitters");
        let count: usize = msg_send![&*group,accessibilityArrayAttributeCount:&*attr];
        assert_eq!(count, 1);
        let subset: Retained<NSArray<AnyObject>> = msg_send![&*group,accessibilityArrayAttributeValues:&*attr,index:0usize,maxCount:1usize];
        assert!(std::ptr::eq(&*subset.objectAtIndex(0), &*divider));

        role(&divider, "AXSplitter");
        let allowed: bool = msg_send![&*divider,isAccessibilitySelectorAllowed:objc2::sel!(accessibilityPerformIncrement)];
        assert!(allowed);
        let before = split.ratio();
        let changed: bool = msg_send![&*divider, accessibilityPerformIncrement];
        assert!(changed);
        assert!(split.ratio() > before);
        let allowed: bool =
            msg_send![&*divider,isAccessibilitySelectorAllowed:objc2::sel!(setAccessibilityValue:)];
        assert!(allowed);
        let _: () = msg_send![&*divider,setAccessibilityValue:&*NSNumber::new_f64(100.)];
        let previous: Retained<NSArray<AnyObject>> =
            msg_send![&*divider, accessibilityPreviousContents];
        let next: Retained<NSArray<AnyObject>> = msg_send![&*divider, accessibilityNextContents];
        assert_eq!(previous.len(), 1);
        assert_eq!(next.len(), 1);
    }
    let menu = Menu::new();
    let checked = State::new(false);
    window
        .set_view({
            let menu = menu.clone();
            let checked = checked.clone();
            move || {
                View::layer(vec![
                    View::panel(vec![menu.button("Actions", "actions")]),
                    menu.view(
                        "actions",
                        vec![
                            MenuAction::submenu(
                                "More",
                                vec![MenuAction::check("Guides", checked.binding()).shortcut("⌘G")],
                            )
                            .key("more"),
                        ],
                    ),
                ])
            }
        })
        .unwrap();
    unsafe {
        let trigger = find(native, "Actions");
        role(&trigger, "AXMenuButton");
        let shown: bool = msg_send![&*trigger, accessibilityPerformShowMenu];
        assert!(shown);
        let shown: bool = msg_send![&*trigger, accessibilityPerformShowMenu];
        assert!(shown);
        let parent: Retained<AnyObject> = msg_send![&*trigger, accessibilityParent];
        let matches: bool = msg_send![&*parent,isEqual:native];
        assert!(matches);
        let expanded: bool = msg_send![&*trigger, isAccessibilityExpanded];
        assert!(expanded);
        let links: Retained<NSArray<AnyObject>> =
            msg_send![&*trigger, accessibilityLinkedUIElements];
        assert_eq!(links.len(), 1);
        let more = find(native, "More");
        role(&more, "AXMenuItem");
        let shown: bool = msg_send![&*more, accessibilityPerformShowMenu];
        assert!(shown);
        let children: Retained<NSArray<AnyObject>> = msg_send![&*more, accessibilityChildren];
        assert_eq!(children.len(), 1);
        let submenu = children.objectAtIndex(0);
        role(&submenu, "AXMenu");
        let parent: Retained<AnyObject> = msg_send![&*submenu, accessibilityParent];
        assert!(std::ptr::eq(&*parent, &*more));
        let guides = find(native, "Guides");
        let help: Retained<NSString> = msg_send![&*guides, accessibilityHelp];
        assert_eq!(help.to_string(), "⌘G");
        let pressed: bool = msg_send![&*guides, accessibilityPerformPress];
        assert!(pressed);
        assert!(checked.get());
        assert!(!menu.open.get());
        let stale: bool = msg_send![&*guides, isAccessibilityElement];
        assert!(!stale);
        let shown: bool = msg_send![&*trigger, accessibilityPerformShowMenu];
        assert!(shown);
        let more = find(native, "More");
        let _: bool = msg_send![&*more, accessibilityPerformShowMenu];
        let children: Retained<NSArray<AnyObject>> = msg_send![&*more, accessibilityChildren];
        let submenu = children.objectAtIndex(0);
        let cancelled: bool = msg_send![&*submenu, accessibilityPerformCancel];
        assert!(cancelled);
        assert!(menu.open.get());
        let root = find(native, "More");
        let cancelled: bool = msg_send![&*root, accessibilityPerformCancel];
        assert!(cancelled);
        assert!(!menu.open.get());
    }
    let alert = Alert::new(
        "Confirm changes",
        vec![
            AlertAction::new("cancel", "Cancel").role(AlertActionRole::Cancel),
            AlertAction::new("save", "Save").role(AlertActionRole::Default),
        ],
        |_| {},
    )
    .unwrap();
    window
        .set_view({
            let alert = alert.clone();
            move || alert.view(View::label("Save this document?"))
        })
        .unwrap();
    unsafe {
        let dialog = find(native, "Confirm changes");
        role(&dialog, "AXGroup");
        let subrole: Retained<NSString> = msg_send![&*dialog, accessibilitySubrole];
        assert_eq!(subrole.to_string(), "AXDialog");
        let modal: bool = msg_send![&*dialog, isAccessibilityModal];
        assert!(modal);
        let default: Retained<AnyObject> = msg_send![&*dialog, accessibilityDefaultButton];
        let cancel: Retained<AnyObject> = msg_send![&*dialog, accessibilityCancelButton];
        role(&default, "AXButton");
        role(&cancel, "AXButton");
        let confirmed: bool = msg_send![&*dialog, accessibilityPerformConfirm];
        assert!(confirmed);
        assert_eq!(alert.result().unwrap().action.as_deref(), Some("save"));
        let again: bool = msg_send![&*dialog, accessibilityPerformConfirm];
        assert!(!again);
    }
    let alert = Alert::new(
        "Cancel changes",
        vec![AlertAction::new("cancel", "Cancel").role(AlertActionRole::Cancel)],
        |_| {},
    )
    .unwrap();
    window
        .set_view({
            let alert = alert.clone();
            move || alert.view(View::label("Cancel?"))
        })
        .unwrap();
    unsafe {
        let dialog = find(native, "Cancel changes");
        let cancelled: bool = msg_send![&*dialog, accessibilityPerformCancel];
        assert!(cancelled);
        assert_eq!(alert.result().unwrap().reason, AlertReason::Escape);
    }
    let color = State::new(Color::rgba(18, 52, 86, 255));
    let picker = ColorPicker::new(color.binding());
    let popup = Popup::new();
    window
        .set_view({
            let picker = picker.clone();
            let popup = popup.clone();
            move || {
                let (button, body) = picker.well(&popup, "color");
                View::layer(vec![button, body])
            }
        })
        .unwrap();
    unsafe {
        let well = find(native, "Color");
        role(&well, "AXColorWell");
        let value: Retained<NSString> = msg_send![&*well, accessibilityValue];
        assert_eq!(value.to_string(), "#123456FF");
        let pressed: bool = msg_send![&*well, accessibilityPerformPress];
        assert!(pressed);
        let dialog = find(native, "Color");
        let subrole: Retained<NSString> = msg_send![&*dialog, accessibilitySubrole];
        assert_eq!(subrole.to_string(), "AXDialog");
        let _: bool = msg_send![&*dialog, accessibilityPerformCancel];
        assert!(!popup.open.get());
    }
    let popup = Popup::new().options(OverlayOptions {
        dismiss_on_escape: false,
        ..Default::default()
    });
    popup.show();
    window
        .set_view({
            let popup = popup.clone();
            move || popup.modal("Required", View::label("Choose Done"))
        })
        .unwrap();
    unsafe {
        let dialog = find(native, "Required");
        let allowed: bool = msg_send![&*dialog,isAccessibilitySelectorAllowed:objc2::sel!(accessibilityPerformCancel)];
        assert!(!allowed);
        let cancelled: bool = msg_send![&*dialog, accessibilityPerformCancel];
        assert!(!cancelled);
        assert!(popup.open.get());
    }
    window.clear_view();
    window.show();
    app.quit_after(std::time::Duration::from_millis(200));
    app.run().unwrap();
    println!("Native control roles, validation, value actions and presentation semantics passed");
}
#[cfg(not(target_os = "macos"))]
fn main() {}
