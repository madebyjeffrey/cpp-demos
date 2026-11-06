use objc2::{msg_send, rc::Retained, runtime::AnyObject};
use objc2_foundation::{NSNotFound, NSRange, NSString};
use scars_window::*;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

mod rendering;
pub use rendering::skia_rendering;

/// Exercise display links through AppKit's real application loop. A manual event
/// pump does not set NSApplication's running state, which display-link delivery needs.
pub fn run_for(app: &Application, duration: std::time::Duration) {
    app.post_after(duration, || {
        let native = app_kit::NSApplication::sharedApplication(objc2::MainThreadMarker::new().unwrap());
        native.stop(None);
        let wake = app_kit::NSEvent::otherEventWithType_location_modifierFlags_timestamp_windowNumber_context_subtype_data1_data2(
            app_kit::NSEventType::ApplicationDefined,
            objc2_foundation::NSPoint::new(0.,0.),
            app_kit::NSEventModifierFlags::empty(),0.,0,None,0,0,0,
        ).unwrap();
        native.postEvent_atStart(&wake,false);
    });
    app.run().unwrap();
}

struct Editor {
    text: RefCell<String>,
    marked: Cell<Option<TextRange>>,
    deactivated: Cell<usize>,
    secure: Cell<bool>,
    commands: Cell<usize>,
}
impl TextInputClient for Editor {
    fn state(&self) -> Option<TextInputState> {
        Some(TextInputState {
            text: self.text.borrow().clone(),
            selection: TextRange {
                start: 1,
                length: 2,
            },
            marked: self.marked.get(),
            active: true,
            editable: true,
            secure: self.secure.get(),
            ..Default::default()
        })
    }
    fn insert(&self, text: String, _: Option<TextRange>) {
        *self.text.borrow_mut() = text;
    }
    fn mark(&self, text: String, selection: TextRange, _: Option<TextRange>) {
        *self.text.borrow_mut() = text;
        self.marked.set(Some(selection));
    }
    fn unmark(&self, _: bool) {
        self.marked.set(None);
    }
    fn command(&self, _: &str) -> bool {
        self.commands.set(self.commands.get() + 1);
        true
    }
    fn rectangle(&self, _: TextRange) -> Rect {
        Rect {
            origin: Point::new(12., 30.),
            size: Size::new(2., 18.),
        }
    }
    fn index(&self, _: Point) -> usize {
        1
    }
    fn deactivate(&self) {
        self.deactivated.set(self.deactivated.get() + 1);
    }
}

/// Exercise the native text-input protocol without requiring a widget engine.
pub fn text_input(window: &Window) {
    let editor = Rc::new(Editor {
        text: RefCell::new("A😀B".into()),
        marked: Cell::new(None),
        deactivated: Cell::new(0),
        secure: Cell::new(false),
        commands: Cell::new(0),
    });
    window.set_text_input_client(Some(editor.clone()));
    // SAFETY: Calls use the exact NSTextInputClient signatures on a manager-owned view.
    unsafe {
        let selection: NSRange = msg_send![window.native_view(), selectedRange];
        assert_eq!(selection, NSRange::new(1, 2));
        let mut actual = NSRange::new(0, 0);
        let value: Option<Retained<objc2_foundation::NSAttributedString>> = msg_send![window.native_view(),attributedSubstringForProposedRange:NSRange::new(2,1),actualRange:&mut actual as *mut NSRange];
        assert_eq!(value.unwrap().string().to_string(), "😀");
        assert_eq!(actual, NSRange::new(1, 2));
        editor.secure.set(true);
        let hidden: Option<Retained<objc2_foundation::NSAttributedString>> = msg_send![window.native_view(),attributedSubstringForProposedRange:NSRange::new(0,4),actualRange:&mut actual as *mut NSRange];
        assert!(hidden.is_none());
        editor.secure.set(false);
        let _: () = msg_send![window.native_view(),insertText:&*NSString::from_str("Inserted"),replacementRange:NSRange::new(NSNotFound as usize,0)];
        assert_eq!(&*editor.text.borrow(), "Inserted");
        let _: () = msg_send![window.native_view(),setMarkedText:&*NSString::from_str("Composition"),selectedRange:NSRange::new(0,1),replacementRange:NSRange::new(NSNotFound as usize,0)];
        let marked: bool = msg_send![window.native_view(), hasMarkedText];
        assert!(marked);
        let _: () = msg_send![window.native_view(), unmarkText];
        assert!(editor.marked.get().is_none());
        let _: () = msg_send![window.native_view(),copy:None::<&AnyObject>];
        assert_eq!(editor.commands.get(), 1);
    }
    window.set_text_input_client(None);
    assert_eq!(editor.deactivated.get(), 1);
}

/// Exercise managed ownership, callback context and the actual New Window menu.
pub fn managed_windows(app: &Application) {
    let veto = Rc::new(Cell::new(true));
    let closed = Rc::new(Cell::new(0));
    let window = app
        .open_window(
            Window::builder("Managed window")
                .theme_toggle(Appearance::Dark)
                .full_screen_button()
                .on_close_request({
                    let veto = veto.clone();
                    move |_| !veto.get()
                })
                .on_closed({
                    let closed = closed.clone();
                    move |window| {
                        assert!(!window.is_open());
                        closed.set(closed.get() + 1);
                    }
                }),
        )
        .unwrap();
    assert!(window.is_visible());
    let native = app_kit::NSApplication::sharedApplication(objc2::MainThreadMarker::new().unwrap());
    let menu = native.mainMenu().unwrap();
    assert_eq!(
        menu.numberOfItems(),
        1,
        "Titlebar buttons must not install application menus"
    );
    let application = menu.itemAtIndex(0).unwrap().submenu().unwrap();
    assert_eq!(application.numberOfItems(), 1);
    assert!(
        application
            .itemAtIndex(0)
            .unwrap()
            .title()
            .to_string()
            .starts_with("Quit ")
    );
    let weak = window.downgrade();
    let button = window
        .native_window()
        .titlebarAccessoryViewControllers()
        .objectAtIndex(0)
        .view()
        .subviews()
        .objectAtIndex(0)
        .downcast::<app_kit::NSButton>()
        .unwrap();
    // SAFETY: The button and its target/action belong to this live managed window.
    unsafe {
        button.performClick(None);
    }
    assert_eq!(window.appearance(), Appearance::Light);
    window.toggle_theme();
    assert_eq!(window.appearance(), Appearance::Dark);
    window.request_close();
    assert!(window.is_open());
    veto.set(false);
    window.request_close();
    assert!(!window.is_open());
    assert_eq!(closed.get(), 1);
    drop(window);
    assert!(
        weak.upgrade().is_none(),
        "Closed managed window must release its owner"
    );
    assert!(
        app.open_window(Window::builder("Invalid").size(-1., 50.))
            .is_err()
    );

    let created = Rc::new(Cell::new(0));
    let closed = Rc::new(Cell::new(0));
    let frames = Rc::new(Cell::new(0));
    // Query the window menu after the first frame explicitly selects that window.
    // Protocol tests can run while macOS keeps another application active.
    app.post_after(std::time::Duration::from_millis(100), || {
        let native =
            app_kit::NSApplication::sharedApplication(objc2::MainThreadMarker::new().unwrap());
        let menu = native.mainMenu().unwrap();
        let application = menu.itemAtIndex(0).unwrap().submenu().unwrap();
        assert!(
            application
                .itemArray()
                .into_iter()
                .all(|item| item.title().to_string() != "New Window")
        );
        let file = menu
            .itemArray()
            .into_iter()
            .find(|item| item.title().to_string() == "File")
            .unwrap()
            .submenu()
            .unwrap();
        let item = file.itemAtIndex(0).unwrap();
        assert_eq!(item.title().to_string(), "New Window");
        assert_eq!(item.keyEquivalent().to_string(), "n");
        let view = menu
            .itemArray()
            .into_iter()
            .find(|item| item.title().to_string() == "View")
            .unwrap()
            .submenu()
            .unwrap();
        view.update();
        assert!(
            view.itemArray()
                .into_iter()
                .any(|item| item.title().to_string() == "App-specific view command")
        );
        // SAFETY: This menu item's action and target were installed by the manager.
        let sent = unsafe {
            native.sendAction_to_from(
                item.action().unwrap(),
                item.target().as_deref(),
                Some(&*item),
            )
        };
        assert!(sent);
    });
    app.quit_after(std::time::Duration::from_millis(700));
    app.run_with_windows({
        let created = created.clone();
        let closed = closed.clone();
        let frames = frames.clone();
        move || {
            created.set(created.get() + 1);
            Window::builder("Factory window")
                .theme_toggle(Appearance::Dark)
                .menu(
                    vec![
                        MenuItem::submenu(
                            "File",
                            vec![MenuItem::action(1, "New Window", Shortcut::command(Key::N))],
                        ),
                        MenuItem::submenu(
                            "View",
                            vec![MenuItem::action(
                                2,
                                "App-specific view command",
                                Shortcut::command(Key::L),
                            )],
                        ),
                    ],
                    |window, command| {
                        if command == 1 {
                            window.request_new_window();
                        }
                    },
                )
                .on_draw({
                    let frames = frames.clone();
                    move |window, frame| {
                        assert!(window.is_open());
                        if frames.get() == 0 {
                            window.focus();
                        }
                        frame.clear([0., 0., 0., 1.]);
                        frames.set(frames.get() + 1);
                    }
                })
                .on_closed({
                    let closed = closed.clone();
                    move |_| closed.set(closed.get() + 1)
                })
        }
    })
    .unwrap();
    assert_eq!(created.get(), 2);
    assert_eq!(closed.get(), 2);
    assert!(
        frames.get() > 0,
        "Managed factory windows must remain alive and render"
    );
}

fn quit_watchdog(app: &Rc<Application>) -> Rc<Cell<bool>> {
    let expired = Rc::new(Cell::new(false));
    app.post_after(std::time::Duration::from_secs(2), {
        let expired = expired.clone();
        let weak = Rc::downgrade(app);
        move || {
            expired.set(true);
            if let Some(app) = weak.upgrade() {
                app.quit();
            }
        }
    });
    expired
}

pub fn last_window_exit() {
    assert!(ApplicationOptions::default().quit_on_last_window_close);
    let app = Rc::new(Application::named("Default last-window exit").unwrap());
    let first = app.open_window(Window::builder("First")).unwrap();
    let second = app.open_window(Window::builder("Second")).unwrap();
    let expired = quit_watchdog(&app);
    app.post_after(std::time::Duration::from_millis(20), {
        let first = first.clone();
        move || first.request_close()
    });
    app.post_after(std::time::Duration::from_millis(60), {
        let second = second.clone();
        move || second.request_close()
    });
    app.run().unwrap();
    assert!(
        !expired.get(),
        "Closing the last window must end run without an explicit quit"
    );
    assert!(
        !first.is_open() && !second.is_open(),
        "Closing one of two windows must not quit"
    );
    drop(app);

    let app = Rc::new(Application::named("Suppressed last-window exit").unwrap());
    app.set_quit_on_last_window_close(false);
    let window = app.open_window(Window::builder("Keep alive")).unwrap();
    let continued = Rc::new(Cell::new(false));
    let expired = quit_watchdog(&app);
    app.post_after(std::time::Duration::from_millis(20), move || {
        window.request_close()
    });
    app.post_after(std::time::Duration::from_millis(60), {
        let weak = Rc::downgrade(&app);
        let continued = continued.clone();
        move || {
            continued.set(true);
            weak.upgrade().unwrap().quit();
        }
    });
    app.run().unwrap();
    assert!(!expired.get());
    assert!(
        continued.get(),
        "Suppression must keep the event loop alive after the last close"
    );
    drop(app);

    let app = Rc::new(Application::named("Replacement window on close").unwrap());
    let replacement_closed = Rc::new(Cell::new(false));
    let initial = app
        .open_window(Window::builder("Initial").on_closed({
            let weak = Rc::downgrade(&app);
            let replacement_closed = replacement_closed.clone();
            move |_| {
                let app = weak.upgrade().unwrap();
                let replacement = app
                    .open_window(Window::builder("Replacement").on_closed({
                        let replacement_closed = replacement_closed.clone();
                        move |_| replacement_closed.set(true)
                    }))
                    .unwrap();
                app.post_after(std::time::Duration::from_millis(40), move || {
                    replacement.request_close()
                });
            }
        }))
        .unwrap();
    let expired = quit_watchdog(&app);
    app.post_after(std::time::Duration::from_millis(20), move || {
        initial.request_close()
    });
    app.run().unwrap();
    assert!(!expired.get());
    assert!(
        replacement_closed.get(),
        "A close callback's replacement window must remain alive until closed"
    );
}
