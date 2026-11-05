//! Main-thread native integration suite. Opt in with SCARS_WINDOW_DESKTOP_TESTS=1.
#[path = "support/mod.rs"]
mod support;
pub fn run() {
    if std::env::var_os("SCARS_WINDOW_DESKTOP_TESTS").is_none() {
        println!(
            "Desktop tests skipped; set SCARS_WINDOW_DESKTOP_TESTS=1 on an awake macOS desktop to run them."
        );
        return;
    }
    use objc2::MainThreadMarker;
    use raw_window_handle::HasWindowHandle;
    use scars_window::*;
    use std::{cell::Cell, rc::Rc, time::Duration};

    assert!(MainThreadMarker::new().is_some());
    let errors = Rc::new(Cell::new(0));
    let app = Rc::new(
        Application::new(ApplicationOptions {
            name: "Rust Window Tests".into(),
            quit_on_last_window_close: false, // This suite intentionally opens/closes several batches.
            on_error: Some(Rc::new({
                let errors = errors.clone();
                move |e| {
                    eprintln!("contained: {e}");
                    errors.set(errors.get() + 1);
                }
            })),
            ..Default::default()
        })
        .expect("Desktop tests require Metal and WindowServer access"),
    );
    assert!(
        std::thread::spawn(|| Application::new(ApplicationOptions::default()).is_err())
            .join()
            .unwrap()
    );
    assert!(Application::new(ApplicationOptions::default()).is_err());

    let window = Window::new(
        &app,
        WindowOptions {
            title: "Rust native regression suite".into(),
            ..Default::default()
        },
    )
    .unwrap();
    let retained = window.surface_clone();
    let frames = Rc::new(Cell::new(0));
    let closed = Rc::new(Cell::new(0));
    let veto = Rc::new(Cell::new(true));
    window.set_callbacks(Callbacks {
        on_draw: Some(Rc::new({
            let frames = frames.clone();
            move |frame| {
                assert!(frame.info.logical_size.width > 0.);
                assert!(frame.info.scale >= 1.);
                assert_eq!(
                    frame.canvas().base_layer_size(),
                    skia::ISize::new(
                        frame.info.pixel_size.width as i32,
                        frame.info.pixel_size.height as i32,
                    ),
                );
                frame.clear([0.02, 0.1, 0.2, 1.]);
                if frames.get() == 0 {
                    support::skia_rendering(frame);
                }
                frames.set(frames.get() + 1);
            }
        })),
        on_closed: Some(Rc::new({
            let closed = closed.clone();
            move || closed.set(closed.get() + 1)
        })),
        on_close_request: Some(Rc::new({
            let veto = veto.clone();
            move || !veto.get()
        })),
        ..Default::default()
    });
    assert!(window.window_handle().is_ok());
    assert_eq!(
        window.native_window().tabbingMode(),
        app_kit::NSWindowTabbingMode::Disallowed
    );
    window.set_title("Changed title");
    assert_eq!(window.title(), "Changed title");
    window.set_content_size(Size::new(720., 520.)).unwrap();
    assert_eq!(window.content_size(), Size::new(720., 520.));
    assert!(window.set_content_size(Size::new(f64::NAN, 520.)).is_err());
    window.set_position(Point::new(100., 150.)).unwrap();
    assert_eq!(window.position(), Point::new(100., 150.));
    window.set_appearance(Appearance::Dark);
    assert_eq!(window.appearance(), Appearance::Dark);
    let changes = Rc::new(Cell::new(0));
    window.set_titlebar_theme_toggle(true, {
        let changes = changes.clone();
        move |light| {
            assert!(!light);
            changes.set(changes.get() + 1);
        }
    });
    assert_eq!(window.appearance(), Appearance::Light);
    let accessory = window
        .native_window()
        .titlebarAccessoryViewControllers()
        .objectAtIndex(0);
    let button = accessory
        .view()
        .subviews()
        .objectAtIndex(0)
        .downcast::<app_kit::NSButton>()
        .unwrap();
    // SAFETY: This is the manager's NSButton and its live target/action pair.
    unsafe {
        button.performClick(None);
    }
    assert_eq!(changes.get(), 1);
    assert_eq!(window.appearance(), Appearance::Dark);
    window.set_titlebar_theme_toggle_state(true);
    assert_eq!(changes.get(), 1);
    window.set_titlebar_fullscreen_button(true);
    window.set_titlebar_fullscreen_button(true);
    assert_eq!(
        window
            .native_window()
            .titlebarAccessoryViewControllers()
            .count(),
        2
    );
    window.set_titlebar_fullscreen_button(false);
    assert_eq!(
        window
            .native_window()
            .titlebarAccessoryViewControllers()
            .count(),
        1
    );
    window.set_titlebar_fullscreen_button(true);
    let fullscreen_button = window
        .native_window()
        .titlebarAccessoryViewControllers()
        .objectAtIndex(1)
        .view()
        .subviews()
        .objectAtIndex(0)
        .downcast::<app_kit::NSButton>()
        .unwrap();
    assert!(fullscreen_button.isEnabled());
    assert!(fullscreen_button.image().is_some());
    support::text_input(&window);

    let commands = Rc::new(Cell::new(0));
    window
        .set_menu(
            vec![MenuItem::submenu(
                "Test",
                vec![MenuItem::action(1, "Test", Shortcut::default())],
            )],
            {
                let commands = commands.clone();
                move |_| commands.set(commands.get() + 1)
            },
        )
        .unwrap();
    assert!(window.invoke_command(1));
    assert_eq!(commands.get(), 1);
    window.set_command_state(1, false, true);
    assert!(!window.invoke_command(1));
    window.set_command_state(1, true, false);
    assert!(
        window
            .set_menu(
                vec![MenuItem::action(0, "Invalid", Shortcut::default())],
                |_| {}
            )
            .is_err()
    );
    assert!(window.invoke_command(1)); // Invalid replacement leaves old menu intact.

    let dialog = Dialog::new(
        &window,
        WindowOptions {
            title: "Sheet".into(),
            ..Default::default()
        },
    )
    .unwrap();
    let completions = Rc::new(Cell::new(0));
    dialog
        .show_modal({
            let completions = completions.clone();
            move |r| {
                assert_eq!(r, DialogResult::Accepted);
                completions.set(completions.get() + 1);
            }
        })
        .unwrap();
    assert!(dialog.hide().is_err());
    assert!(
        !fullscreen_button.isEnabled(),
        "Parent-modal sheets must disable the titlebar full-screen action"
    );
    assert!(!window.invoke_command(1));
    assert!(dialog.show_modal(|_| {}).is_err());
    dialog.dismiss(DialogResult::Accepted);
    dialog.dismiss(DialogResult::Cancelled);
    assert_eq!(completions.get(), 1);
    assert!(fullscreen_button.isEnabled());
    let self_destroying = Rc::new(std::cell::RefCell::new(Some(
        Dialog::new(&window, WindowOptions::default()).unwrap(),
    )));
    self_destroying
        .borrow()
        .as_ref()
        .unwrap()
        .show_modal({
            let weak = Rc::downgrade(&self_destroying);
            move |_| {
                weak.upgrade().unwrap().borrow_mut().take();
            }
        })
        .unwrap();
    let dialog_handle = self_destroying.borrow().as_ref().unwrap().surface_clone();
    dialog_handle.close();
    assert!(self_destroying.borrow().is_none());

    window.show();
    app.post({
        let window = window.surface_clone();
        move || window.focus()
    });
    support::run_for(&app, Duration::from_millis(700));
    assert!(window.is_visible());
    assert!(
        frames.get() > 0,
        "No visible Metal frame delivered; use an awake foreground desktop"
    );
    let before_resize = frames.get();
    window.set_content_size(Size::new(760., 540.)).unwrap();
    support::run_for(&app, Duration::from_millis(100));
    assert!(
        frames.get() > before_resize,
        "Resize must redraw the Skia target"
    );
    window.set_animating(true);
    support::run_for(&app, Duration::from_millis(150));
    let animated = frames.get();
    assert!(animated > 1);
    window.set_animating(false);
    support::run_for(&app, Duration::from_millis(100));
    let paused = frames.get();
    support::run_for(&app, Duration::from_millis(100));
    assert_eq!(frames.get(), paused);
    window
        .set_redraw_delay(Some(Duration::from_millis(10)))
        .unwrap();
    support::run_for(&app, Duration::from_millis(100));
    assert!(frames.get() > paused);
    window.hide().unwrap();
    assert!(!window.is_visible());
    window.show();
    assert!(!app.quit());
    assert!(window.is_open()); // Veto must preserve all windows.
    veto.set(false);
    let child = Dialog::new(&window, WindowOptions::default()).unwrap();
    let cancelled = Rc::new(Cell::new(false));
    child
        .show_modal({
            let cancelled = cancelled.clone();
            move |r| {
                assert_eq!(r, DialogResult::Cancelled);
                cancelled.set(true);
            }
        })
        .unwrap();
    window.close();
    window.close();
    assert_eq!(closed.get(), 1);
    assert!(!child.is_open());
    assert!(cancelled.get());
    assert!(!retained.is_open());
    assert!(!fullscreen_button.isEnabled());
    assert!(retained.window_handle().is_err());
    assert_eq!(errors.get(), 0);

    // Replacement/close inside a callback must never retain a RefCell borrow.
    let reentrant = Window::new(&app, WindowOptions::default()).unwrap();
    let weak = reentrant.downgrade();
    reentrant
        .set_menu(
            vec![MenuItem::submenu(
                "Test",
                vec![MenuItem::action(9, "Close", Shortcut::default())],
            )],
            move |_| {
                let s = weak.upgrade().unwrap();
                s.set_callbacks(Callbacks::default());
                s.close();
            },
        )
        .unwrap();
    assert!(reentrant.invoke_command(9));
    assert!(!reentrant.is_open());
    let panicking = Window::new(&app, WindowOptions::default()).unwrap();
    panicking.set_callbacks(Callbacks {
        on_draw: Some(Rc::new(|_| panic!("intentional draw callback panic"))),
        ..Default::default()
    });
    panicking.show();
    let recovered = Rc::new(Cell::new(false));
    app.post_after(Duration::from_millis(500), {
        let surface = panicking.surface_clone();
        let recovered = recovered.clone();
        move || {
            surface.set_callbacks(Callbacks {
                on_draw: Some(Rc::new(move |frame| {
                    frame.clear([0., 0., 0., 1.]);
                    recovered.set(true);
                })),
                ..Default::default()
            })
        }
    });
    app.post_after(Duration::from_millis(1000), {
        let weak = Rc::downgrade(&app);
        move || {
            weak.upgrade().unwrap().quit();
        }
    });
    app.run().unwrap();
    assert_eq!(errors.get(), 1);
    assert!(recovered.get());
    assert!(!panicking.is_open());
    support::managed_windows(&app);
    let survivor = Window::new(&app, WindowOptions::default()).unwrap();
    drop(app);
    assert!(!survivor.is_open());
    support::last_window_exit();
    println!(
        "PASS: native windows, ownership, menus, theme button, sheets, IME, Skia/Metal pixels, text, SVG, images, timers, veto and reentrant callbacks ({animated} animation frames)"
    );
}
