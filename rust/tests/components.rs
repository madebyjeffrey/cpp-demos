//! Opt-in main-thread integration test of the mounted component bridge.
#[cfg(target_os = "macos")]
fn main() {
    if std::env::var_os("SCARS_WINDOW_DESKTOP_TESTS").is_none() {
        println!("Component desktop tests skipped");
        return;
    }
    use foundation::{NSArray, NSRange, NSString};
    use objc2::{Message, msg_send, rc::Retained, runtime::AnyObject};
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
    // Native line/document selectors remain distinct, and explicit IME replacement
    // commits one transaction rather than recording an intermediate marked string.
    let command_text = State::new("first\nsecond".to_owned());
    let command_host = window
        .set_view({
            let text = command_text.clone();
            move || view! {text_editor("",bind:text);}
        })
        .unwrap();
    let id = command_host.snapshot().unwrap().last().unwrap().id;
    command_host.focus(id).unwrap();
    command_host.set_active(true);
    command_host
        .select_text(
            id,
            ui::Selection {
                anchor: 8,
                caret: 8,
            },
        )
        .unwrap();
    unsafe {
        let _: () = msg_send![native,doCommandBySelector:objc2::sel!(moveToBeginningOfLine:)];
        assert_eq!(
            command_host.text_input().unwrap().unwrap().selection.caret,
            6
        );
        let _: () = msg_send![native,doCommandBySelector:objc2::sel!(moveToEndOfDocument:)];
        assert_eq!(
            command_host.text_input().unwrap().unwrap().selection.caret,
            12
        );
        let marked = NSString::from_str("日");
        let _: () = msg_send![native,setMarkedText:&*marked,selectedRange:NSRange::new(1,0),replacementRange:NSRange::new(isize::MAX as usize,0)];
        assert_eq!(command_text.get(), "first\nsecond");
        let commit = NSString::from_str("日本");
        let _: () = msg_send![native,insertText:&*commit,replacementRange:NSRange::new(12,1)];
        assert_eq!(command_text.get(), "first\nsecond日本");
        let _: () = msg_send![native,doCommandBySelector:objc2::sel!(undo:)];
        assert_eq!(command_text.get(), "first\nsecond");
        let mut actual = NSRange::new(0, 0);
        let caret: foundation::NSRect = msg_send![native,firstRectForCharacterRange:NSRange::new(12,0),actualRange:&mut actual as *mut NSRange];
        assert_eq!(actual, NSRange::new(12, 0));
        assert!(
            caret.size.width > 0.
                && caret.size.height > 0.
                && caret.origin.x.is_finite()
                && caret.origin.y.is_finite()
        );
        let point = foundation::NSPoint::new(
            caret.origin.x + 0.25,
            caret.origin.y + caret.size.height / 2.,
        );
        let index: usize = msg_send![native,characterIndexForPoint:point];
        assert_eq!(index, 12);
    }
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
        .set_view(
            move || view! {password_field("Secret",bind:secret,allow_reveal:true,max_length:40);},
        )
        .unwrap();
    let id = secure_host
        .snapshot()
        .unwrap()
        .into_iter()
        .find(|n| n.kind == ui::Kind::TextField)
        .unwrap()
        .id;
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
        let role: Retained<NSString> = msg_send![&*element, accessibilitySubrole];
        assert_eq!(role.to_string(), "AXSecureTextField");
        assert!(secure_host.set_password_revealed(id, true).unwrap());
        let value: Option<Retained<AnyObject>> = msg_send![&*element, accessibilityValue];
        assert!(value.is_none());
        secure_host
            .text_command(ui::TextCommand::SelectAll, false)
            .unwrap();
        let board = app_kit::NSPasteboard::generalPasteboard();
        let before = board.changeCount();
        let _: () = msg_send![native,doCommandBySelector:objc2::sel!(copy:)];
        let _: () = msg_send![native,doCommandBySelector:objc2::sel!(cut:)];
        assert_eq!(
            board.changeCount(),
            before,
            "Password copy/cut must not change the clipboard"
        );
        assert_eq!(
            secure_host.text_input().unwrap().unwrap().text,
            "do not expose"
        );
        let marked = NSString::from_str("日");
        let _: () = msg_send![native,setMarkedText:&*marked,selectedRange:NSRange::new(1,0),replacementRange:NSRange::new(isize::MAX as usize,0)];
        let _: () = msg_send![native, unmarkText];
        assert!(!secure_host.text_input().unwrap().unwrap().can_undo);
    }
    let query = State::new(String::new());
    let called = State::new(0);
    let search_host=window.set_view({let called=called.clone();move||{
        let called=called.clone();view!{search_field("Search",bind:query,debounce:std::time::Duration::from_secs(10),on_query:move|_|called.update(|n|*n+=1));}
    }}).unwrap();
    let id = search_host.snapshot().unwrap().last().unwrap().id;
    search_host.focus(id).unwrap();
    search_host.set_active(true);
    search_host.insert_text("pending", None).unwrap();
    assert!(search_host.next_deadline().is_some());
    window.clear_view();
    assert!(search_host.next_deadline().is_none());
    search_host.run_due_at(std::time::Instant::now() + std::time::Duration::from_secs(20));
    assert_eq!(called.get(), 0);
    let exact = State::new(9_007_199_254_740_993i64);
    let number = ui::Numeric::new(exact.binding(), i64::MIN, i64::MAX, 1);
    let step_host = window
        .set_view(move || ui::stepper("Count", &number))
        .unwrap();
    step_host.set_active(true);
    unsafe {
        let children: Retained<NSArray<AnyObject>> = msg_send![native, accessibilityChildren];
        let step = children.objectAtIndex(0);
        let role: Retained<NSString> = msg_send![&*step, accessibilityRole];
        assert_eq!(role.to_string(), "AXIncrementor");
        let value: Retained<foundation::NSNumber> = msg_send![&*step, accessibilityValue];
        assert_eq!(value.longLongValue(), 9_007_199_254_740_993);
        let min: Retained<foundation::NSNumber> = msg_send![&*step, accessibilityMinValue];
        assert_eq!(min.longLongValue(), i64::MIN);
        let max: Retained<foundation::NSNumber> = msg_send![&*step, accessibilityMaxValue];
        assert_eq!(max.longLongValue(), i64::MAX);
        let increased: bool = msg_send![&*step, accessibilityPerformIncrement];
        assert!(increased);
        assert_eq!(exact.get(), 9_007_199_254_740_994);
        let decreased: bool = msg_send![&*step, accessibilityPerformDecrement];
        assert!(decreased);
        assert_eq!(exact.get(), 9_007_199_254_740_993);
        let renderer = ui::Renderer::default();
        step_host
            .layout(ui::Size::new(240., 60.), &renderer)
            .unwrap();
        let up = step_host
            .snapshot()
            .unwrap()
            .into_iter()
            .find(|n| n.label == "Increase Count")
            .unwrap();
        step_host
            .pointer(ui::Pointer {
                kind: ui::PointerKind::Down,
                position: ui::Point {
                    x: up.bounds.x + 5.,
                    y: up.bounds.y + 5.,
                },
                button: 0,
                modifiers: Default::default(),
            })
            .unwrap();
        step_host
            .layout(ui::Size::new(240., 60.), &renderer)
            .unwrap();
        assert!(step_host.next_deadline().is_some());
        window.clear_view();
        assert!(step_host.next_deadline().is_none());
        step_host.run_due_at(std::time::Instant::now() + std::time::Duration::from_secs(1));
        assert_eq!(exact.get(), 9_007_199_254_740_994);
        let increased: bool = msg_send![&*step, accessibilityPerformIncrement];
        assert!(!increased);
    }
    let split = ui::Split::new(0.4).limits(50., 50.);
    let ratio = split.clone();
    let split_host = window
        .set_view(move || {
            split.view(
                ui::View::button("First", || {}),
                ui::View::button("Second", || {}),
            )
        })
        .unwrap();
    unsafe {
        let roots: Retained<NSArray<AnyObject>> = msg_send![native, accessibilityChildren];
        let group = roots.objectAtIndex(0);
        let role: Retained<NSString> = msg_send![&*group, accessibilityRole];
        assert_eq!(role.to_string(), "AXSplitGroup");
        let splitters: Retained<NSArray<AnyObject>> = msg_send![&*group, accessibilitySplitters];
        assert_eq!(splitters.len(), 1);
        let divider = splitters.objectAtIndex(0);
        let role: Retained<NSString> = msg_send![&*divider, accessibilityRole];
        assert_eq!(role.to_string(), "AXSplitter");
        let orientation: app_kit::NSAccessibilityOrientation =
            msg_send![&*divider, accessibilityOrientation];
        assert_eq!(orientation, app_kit::NSAccessibilityOrientation::Vertical);
        let previous: Retained<NSArray<AnyObject>> =
            msg_send![&*divider, accessibilityPreviousContents];
        assert_eq!(previous.len(), 1);
        let first_pane = previous.objectAtIndex(0);
        let next: Retained<NSArray<AnyObject>> = msg_send![&*divider, accessibilityNextContents];
        assert_eq!(next.len(), 1);
        let minimum: Retained<foundation::NSNumber> = msg_send![&*divider, accessibilityMinValue];
        assert_eq!(minimum.doubleValue(), 50.);
        let maximum: Retained<foundation::NSNumber> = msg_send![&*divider, accessibilityMaxValue];
        assert!(maximum.doubleValue() > minimum.doubleValue());
        let before: Retained<foundation::NSNumber> = msg_send![&*divider, accessibilityValue];
        let increased: bool = msg_send![&*divider, accessibilityPerformIncrement];
        assert!(increased);
        let after: Retained<foundation::NSNumber> = msg_send![&*divider, accessibilityValue];
        assert!((after.doubleValue() - before.doubleValue() - 10.).abs() < 0.001);
        let value = foundation::NSNumber::new_f64(after.doubleValue() - 20.);
        let _: () = msg_send![&*divider,setAccessibilityValue:&*value];
        let changed: Retained<foundation::NSNumber> = msg_send![&*divider, accessibilityValue];
        assert!((changed.doubleValue() - value.doubleValue()).abs() < 0.001);
        let pressed: bool = msg_send![&*divider, accessibilityPerformPress];
        assert!(pressed);
        let description: Retained<NSString> = msg_send![&*divider, accessibilityValueDescription];
        assert!(description.to_string().contains("collapsed"));
        let previous: Retained<NSArray<AnyObject>> =
            msg_send![&*divider, accessibilityPreviousContents];
        assert_eq!(previous.len(), 0);
        let stale: bool = msg_send![&*first_pane, accessibilityPerformPress];
        assert!(!stale);

        let children: Retained<NSArray<AnyObject>> = msg_send![&*group, accessibilityChildren];
        assert_eq!(children.len(), 2);
        ratio.restore();
        let children: Retained<NSArray<AnyObject>> = msg_send![&*group, accessibilityChildren];
        assert_eq!(children.len(), 3);
        let divider_id = split_host
            .snapshot()
            .unwrap()
            .into_iter()
            .find(|n| n.kind == ui::Kind::SplitDivider)
            .unwrap()
            .id;
        let b = split_host
            .snapshot()
            .unwrap()
            .into_iter()
            .find(|n| n.id == divider_id)
            .unwrap()
            .bounds;
        split_host
            .pointer(ui::Pointer {
                kind: ui::PointerKind::Down,
                position: ui::Point {
                    x: b.x + 4.,
                    y: b.y + 4.,
                },
                button: 0,
                modifiers: Default::default(),
            })
            .unwrap();
        let saved = ratio.ratio();
        window.clear_view();
        split_host
            .pointer(ui::Pointer {
                kind: ui::PointerKind::Move,
                position: ui::Point { x: 500., y: 10. },
                button: 0,
                modifiers: Default::default(),
            })
            .unwrap();
        assert_eq!(ratio.ratio(), saved);
        let increased: bool = msg_send![&*divider, accessibilityPerformIncrement];
        assert!(!increased);
    }
    // Menus expose native roles/check values, accept printable key events
    // without claiming an editor, and leave retained closed items inert.
    let menu = ui::Menu::new();
    let checked_menu = State::new(false);
    let menu_host = window
        .set_view({
            let menu = menu.clone();
            let checked = checked_menu.clone();
            move || {
                ui::View::layer(vec![
                    ui::View::button("Open", || {}).key("open"),
                    menu.view(
                        "open",
                        vec![
                            ui::MenuAction::new("Alpha", || {}),
                            ui::MenuAction::check("Beta", checked.binding()).shortcut("⌘B"),
                            ui::MenuAction::submenu("Empty", vec![]),
                        ],
                    ),
                ])
            }
        })
        .unwrap();
    menu_host.set_active(true);
    menu.show();
    unsafe {
        let roots: Retained<NSArray<AnyObject>> = msg_send![native, accessibilityChildren];
        let scope = roots.objectAtIndex(0);
        let panels: Retained<NSArray<AnyObject>> = msg_send![&*scope, accessibilityChildren];
        let panel = panels.objectAtIndex(0);
        let role: Retained<NSString> = msg_send![&*panel, accessibilityRole];
        assert_eq!(role.to_string(), "AXMenu");
        let rows: Retained<NSArray<AnyObject>> = msg_send![&*panel, accessibilityChildren];
        let beta = rows.objectAtIndex(1);
        let role: Retained<NSString> = msg_send![&*beta, accessibilityRole];
        assert_eq!(role.to_string(), "AXMenuItem");
        let value: Retained<foundation::NSNumber> = msg_send![&*beta, accessibilityValue];
        assert!(!value.boolValue());
        let help: Retained<NSString> = msg_send![&*beta, accessibilityHelp];
        assert_eq!(help.to_string(), "⌘B");
        let characters = NSString::from_str("b");
        let event = app_kit::NSEvent::keyEventWithType_location_modifierFlags_timestamp_windowNumber_context_characters_charactersIgnoringModifiers_isARepeat_keyCode(
            app_kit::NSEventType::KeyDown, foundation::NSPoint::ZERO, app_kit::NSEventModifierFlags::empty(), 0., window.native_window().windowNumber(), None, &characters, &characters, false, 11).unwrap();
        let _: () = msg_send![native, keyDown:&*event];
        assert_eq!(
            menu_host.focused(),
            menu_host
                .snapshot()
                .unwrap()
                .iter()
                .find(|n| n.text == "Beta")
                .map(|n| n.id)
        );
        assert!(menu_host.text_input().unwrap().is_none());
        let pressed: bool = msg_send![&*beta, accessibilityPerformPress];
        assert!(pressed);
        assert!(checked_menu.get());
        assert!(!menu.open.get());
        let pressed: bool = msg_send![&*beta, accessibilityPerformPress];
        assert!(!pressed);
        menu.show();
        let roots: Retained<NSArray<AnyObject>> = msg_send![native, accessibilityChildren];
        let scope = roots.objectAtIndex(0);
        let panels: Retained<NSArray<AnyObject>> = msg_send![&*scope, accessibilityChildren];
        let panel = panels.objectAtIndex(0);
        let empty = menu_host
            .snapshot()
            .unwrap()
            .into_iter()
            .find(|n| n.text == "Empty")
            .unwrap();
        menu_host.activate(empty.id).unwrap();
        let _: Retained<NSArray<AnyObject>> = msg_send![native, accessibilityChildren];
        let rows: Retained<NSArray<AnyObject>> = msg_send![&*panel, accessibilityChildren];
        let empty = rows.objectAtIndex(2);
        let expanded: bool = msg_send![&*empty, isAccessibilityExpanded];
        assert!(expanded);
        window.clear_view();
        assert!(!menu.open.get());
    }
    let reasons = State::new(Vec::new());
    let popup = ui::Popup::new()
        .options(ui::OverlayOptions {
            width: Some(240.),
            margin: 8.,
            ..Default::default()
        })
        .on_dismiss(reasons.handler(|s, r| s.update(|v| v.push(r))));
    let context = ui::Menu::new();
    let draft = State::new("before".to_owned());
    let overlay_host = window
        .set_view({
            let popup = popup.clone();
            let context = context.clone();
            let draft = draft.clone();
            move || {
                ui::View::layer(vec![
                    popup.button("Open", "open"),
                    popup.popover(
                        "open",
                        ui::View::text_field("Draft", draft.binding())
                            .key("draft")
                            .tooltip("Editor help")
                            .context_menu(&context, vec![ui::MenuAction::new("Copy draft", || {})]),
                    ),
                ])
            }
        })
        .unwrap();
    overlay_host.set_active(true);
    popup.show();
    unsafe {
        let _: Retained<NSArray<AnyObject>> = msg_send![native, accessibilityChildren];
        let field = overlay_host
            .snapshot()
            .unwrap()
            .into_iter()
            .find(|n| n.key.as_deref() == Some("draft"))
            .unwrap();
        assert_eq!(overlay_host.focused(), Some(field.id));
        let focused: Retained<AnyObject> = msg_send![native, accessibilityFocusedUIElement];
        let help: Retained<NSString> = msg_send![&*focused, accessibilityHelp];
        assert_eq!(help.to_string(), "Editor help");
        let marked = NSString::from_str("日本");
        let _: () = msg_send![native,setMarkedText:&*marked,selectedRange:NSRange::new(2,0),replacementRange:NSRange::new(isize::MAX as usize,0)];
        let _: () = msg_send![native,doCommandBySelector:objc2::sel!(cancelOperation:)];
        assert!(popup.open.get());
        assert!(overlay_host.text_input().unwrap().unwrap().marked.is_none());
        assert_eq!(draft.get(), "before");
        let characters = NSString::from_str("\u{f70d}");
        let event=app_kit::NSEvent::keyEventWithType_location_modifierFlags_timestamp_windowNumber_context_characters_charactersIgnoringModifiers_isARepeat_keyCode(app_kit::NSEventType::KeyDown,foundation::NSPoint::ZERO,app_kit::NSEventModifierFlags::Shift,0.,window.native_window().windowNumber(),None,&characters,&characters,false,109).unwrap();
        let _: () = msg_send![native,keyDown:&*event];
        assert!(context.open.get());
        overlay_host
            .key(ui::KeyEvent {
                key: ui::Key::Escape,
                down: true,
                repeat: false,
                modifiers: Default::default(),
            })
            .unwrap();
        let _: Retained<NSArray<AnyObject>> = msg_send![native, accessibilityChildren];
        assert_eq!(overlay_host.focused(), Some(field.id));
        assert!(popup.open.get());
        let _: () = msg_send![native,doCommandBySelector:objc2::sel!(cancelOperation:)];
        let _: Retained<NSArray<AnyObject>> = msg_send![native, accessibilityChildren];
        assert!(!popup.open.get());
        assert_eq!(reasons.get(), vec![ui::DismissReason::Escape]);
        popup.show();
        let _: Retained<NSArray<AnyObject>> = msg_send![native, accessibilityChildren];
        window.clear_view();
        assert!(!popup.open.get());
        assert_eq!(
            reasons.get(),
            vec![ui::DismissReason::Escape, ui::DismissReason::Unmounted]
        );
    }
    let tabs = ui::Tabs::new("a");
    let draft = State::new("draft".to_owned());
    let tab_host = window
        .set_view({
            let tabs = tabs.clone();
            let draft = draft.clone();
            move || {
                tabs.view(vec![
                    ui::Tab::new(
                        "a",
                        "Alpha",
                        ui::View::text_field("Draft", draft.binding()).key("tab-draft"),
                    ),
                    ui::Tab::new("disabled", "Unavailable", ui::View::label("No")).enabled(false),
                    ui::Tab::new("b", "Beta", ui::View::button("Other", || {})),
                ])
                .accessible_label("Documents")
            }
        })
        .unwrap();
    tab_host.set_active(true);
    unsafe {
        let roots: Retained<NSArray<AnyObject>> = msg_send![native, accessibilityChildren];
        let group = roots.objectAtIndex(0);
        let role: Retained<NSString> = msg_send![&*group, accessibilityRole];
        assert_eq!(role.to_string(), "AXTabGroup");
        let headers: Retained<NSArray<AnyObject>> = msg_send![&*group, accessibilityTabs];
        assert_eq!(headers.len(), 3);
        let alpha = headers.objectAtIndex(0);
        let unavailable = headers.objectAtIndex(1);
        let beta = headers.objectAtIndex(2);
        let role: Retained<NSString> = msg_send![&*alpha, accessibilityRole];
        assert_eq!(role.to_string(), "AXRadioButton");
        let subrole: Retained<NSString> = msg_send![&*alpha, accessibilitySubrole];
        assert_eq!(subrole.to_string(), "AXTabButton");
        let parent: Retained<AnyObject> = msg_send![&*alpha, accessibilityParent];
        assert!(std::ptr::eq(&*parent, &*group));
        let value: Retained<AnyObject> = msg_send![&*group, accessibilityValue];
        assert!(std::ptr::eq(&*value, &*alpha));
        let selected: Retained<NSArray<AnyObject>> =
            msg_send![&*group, accessibilitySelectedChildren];
        assert_eq!(selected.len(), 1);
        let pressed: bool = msg_send![&*unavailable, accessibilityPerformPress];
        assert!(!pressed);
        let contents: Retained<NSArray<AnyObject>> = msg_send![&*group, accessibilityContents];
        assert_eq!(contents.len(), 1);
        let panel = contents.objectAtIndex(0);
        let fields: Retained<NSArray<AnyObject>> = msg_send![&*panel, accessibilityChildren];
        let field = fields.objectAtIndex(0);
        let field_id = tab_host
            .snapshot()
            .unwrap()
            .into_iter()
            .find(|n| n.key.as_deref() == Some("tab-draft"))
            .unwrap()
            .id;
        tab_host.focus(field_id).unwrap();
        tab_host
            .select_text(
                field_id,
                ui::Selection {
                    anchor: 5,
                    caret: 5,
                },
            )
            .unwrap();
        let marked = NSString::from_str("日本");
        let _: () = msg_send![native,setMarkedText:&*marked,selectedRange:NSRange::new(2,0),replacementRange:NSRange::new(isize::MAX as usize,0)];
        let pressed: bool = msg_send![&*beta, accessibilityPerformPress];
        assert!(pressed);
        assert_eq!(draft.get(), "draft日本");
        let _: Retained<NSArray<AnyObject>> = msg_send![native, accessibilityChildren];
        let stale = NSString::from_str("hidden edit");
        let _: () = msg_send![&*field,setAccessibilityValue:&*stale];
        assert_eq!(draft.get(), "draft日本");
        let pressed: bool = msg_send![&*alpha, accessibilityPerformPress];
        assert!(pressed);
        let _: Retained<NSArray<AnyObject>> = msg_send![native, accessibilityChildren];
        let root_id = tab_host
            .snapshot()
            .unwrap()
            .into_iter()
            .find(|n| n.kind == ui::Kind::Tabs)
            .unwrap()
            .id;
        assert!(tab_host.focus_tab_page(root_id).unwrap());
        assert_eq!(tab_host.focused(), Some(field_id));
        let _: () = msg_send![native,doCommandBySelector:objc2::sel!(undo:)];
        assert_eq!(draft.get(), "draft");
        let characters = NSString::from_str("\t");
        let event=app_kit::NSEvent::keyEventWithType_location_modifierFlags_timestamp_windowNumber_context_characters_charactersIgnoringModifiers_isARepeat_keyCode(app_kit::NSEventType::KeyDown,foundation::NSPoint::ZERO,app_kit::NSEventModifierFlags::Control,0.,window.native_window().windowNumber(),None,&characters,&characters,false,48).unwrap();
        let _: () = msg_send![native,keyDown:&*event];
        let _: Retained<NSArray<AnyObject>> = msg_send![native, accessibilityChildren];
        assert_eq!(tabs.selected(), "b");
        window.clear_view();
        let pressed: bool = msg_send![&*beta, accessibilityPerformPress];
        assert!(!pressed);
    }
    // Wizard actions validate live data after native IME commit; old Next
    // elements must not become Finish actions after a step transition.
    fn find_label(object: &AnyObject, label: &str) -> Option<Retained<AnyObject>> {
        unsafe {
            let current: Option<Retained<NSString>> = msg_send![object, accessibilityLabel];
            if current.is_some_and(|s| s.to_string() == label) {
                return Some(object.retain());
            }
            let children: Option<Retained<NSArray<AnyObject>>> =
                msg_send![object, accessibilityChildren];
            children.and_then(|nodes| nodes.iter().find_map(|n| find_label(&n, label)))
        }
    }
    let wizard_name = State::new(String::new());
    let finished = State::new(0);
    let cancelled = State::new(0);
    let changes = State::new(Vec::new());
    let done = State::new(false);
    let setup = ui::Wizard::new(
        "identity",
        vec![
            ui::WizardStep::new("identity", "Identity").validate({
                let name = wizard_name.clone();
                move || {
                    if name.get().is_empty() {
                        Err("A name is required.".into())
                    } else {
                        Ok(())
                    }
                }
            }),
            ui::WizardStep::new("review", "Review"),
        ],
    )
    .unwrap()
    .on_finish({
        let finished = finished.clone();
        let done = done.clone();
        move || {
            finished.update(|n| *n += 1);
            done.set(true);
        }
    })
    .on_cancel(cancelled.action(|s| s.update(|n| *n += 1)))
    .on_change(changes.handler(|s, c: ui::WizardChange| s.update(|v| v.push(c))));
    let wizard_host = window
        .set_view({
            let setup = setup.clone();
            let name = wizard_name.clone();
            let done = done.clone();
            move || {
                if done.get() {
                    ui::View::label("Created")
                } else {
                    setup.view(vec![
                        (
                            "identity",
                            ui::View::text_field("Project name", name.binding()).key("wizard-name"),
                        ),
                        ("review", ui::View::label("Review project")),
                    ])
                }
            }
        })
        .unwrap();
    wizard_host.set_active(true);
    unsafe {
        let next = find_label(native, "Next").unwrap();
        let enabled: bool = msg_send![&*next, isAccessibilityEnabled];
        assert!(!enabled);
        let pressed: bool = msg_send![&*next, accessibilityPerformPress];
        assert!(!pressed);
        assert_eq!(setup.current(), Some("identity".into()));
        assert!(find_label(native, "Wizard validation: A name is required.").is_some());
        let field = find_label(native, "Project name").unwrap();
        let name = NSString::from_str("Kestrel");
        let _: () = msg_send![&*field,setAccessibilityValue:&*name];
        let field_id = wizard_host
            .snapshot()
            .unwrap()
            .into_iter()
            .find(|n| n.key.as_deref() == Some("wizard-name"))
            .unwrap()
            .id;
        wizard_host.focus(field_id).unwrap();
        wizard_host
            .select_text(
                field_id,
                ui::Selection {
                    anchor: 7,
                    caret: 7,
                },
            )
            .unwrap();
        let marked = NSString::from_str("!");
        let _: () = msg_send![native,setMarkedText:&*marked,selectedRange:NSRange::new(1,0),replacementRange:NSRange::new(isize::MAX as usize,0)];
        let next = find_label(native, "Next").unwrap();
        let pressed: bool = msg_send![&*next, accessibilityPerformPress];
        assert!(pressed);
        assert_eq!(wizard_name.get(), "Kestrel!");
        assert_eq!(setup.current(), Some("review".into()));
        let finish = find_label(native, "Finish").unwrap();
        let pressed: bool = msg_send![&*next, accessibilityPerformPress];
        assert!(!pressed);
        assert_eq!(finished.get(), 0);
        assert_eq!(
            changes.get(),
            vec![ui::WizardChange {
                from: "identity".into(),
                to: "review".into()
            }]
        );
        let pressed: bool = msg_send![&*finish, accessibilityPerformPress];
        assert!(pressed);
        assert_eq!(finished.get(), 1);
        assert!(find_label(native, "Created").is_some());
        let pressed: bool = msg_send![&*finish, accessibilityPerformPress];
        assert!(!pressed);
        wizard_name.set(String::new());
        window
            .set_view({
                let setup = setup.clone();
                move || setup.navigation()
            })
            .unwrap();
        assert!(setup.go_to("identity"));
        let cancel = find_label(native, "Cancel").unwrap();
        let pressed: bool = msg_send![&*cancel, accessibilityPerformPress];
        assert!(pressed);
        assert_eq!(cancelled.get(), 1);
        window.clear_view();
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
