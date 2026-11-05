//! Opt in: this test enters and exits a native macOS full-screen Space.
fn fullscreen_button(
    window: &scars_window::Surface,
) -> objc2::rc::Retained<scars_window::app_kit::NSButton> {
    for accessory in window.native_window().titlebarAccessoryViewControllers() {
        for child in accessory.view().subviews() {
            if let Ok(button) = child.downcast::<scars_window::app_kit::NSButton>()
                && button.action() == Some(objc2::sel!(fullScreenButton:))
            {
                return button;
            }
        }
    }
    panic!("Explicit full-screen titlebar button is missing")
}

pub fn run() {
    if std::env::var_os("SCARS_WINDOW_FULLSCREEN_TESTS").is_none() {
        println!(
            "Full-screen transition test skipped; set SCARS_WINDOW_FULLSCREEN_TESTS=1 on an awake desktop."
        );
        return;
    }
    use scars_window::*;
    use std::{cell::Cell, rc::Rc, time::Duration};
    let app = Rc::new(Application::named("Full-screen integration test").unwrap());
    let entered = Rc::new(Cell::new(false));
    let exited = Rc::new(Cell::new(false));
    let expired = Rc::new(Cell::new(false));
    let window = Window::builder("Explicit full-screen command")
        .theme_toggle(Appearance::Dark)
        .full_screen_button()
        .on_draw(|_, frame| frame.clear([0.02, 0.05, 0.1, 1.]))
        .on_event({
            let entered = entered.clone();
            let exited = exited.clone();
            let weak = Rc::downgrade(&app);
            move |window, event| {
                if let Event::FullscreenChanged(fullscreen) = event {
                    assert_eq!(window.is_fullscreen(), *fullscreen);
                    let button = fullscreen_button(window);
                    assert!(button.isEnabled());
                    assert!(button.image().is_some());
                    assert_eq!(
                        button.toolTip().unwrap().to_string(),
                        if *fullscreen {
                            "Exit Full Screen"
                        } else {
                            "Enter Full Screen"
                        }
                    );
                    if *fullscreen {
                        entered.set(true);
                        window.set_fullscreen(true).unwrap(); // Repeating a completed state must not toggle it.
                        let surface = window.clone();
                        weak.upgrade()
                            .unwrap()
                            .post_after(Duration::from_millis(100), move || {
                                // SAFETY: Live manager-owned button and target/action pair.
                                unsafe {
                                    fullscreen_button(&surface).performClick(None);
                                }
                            });
                    } else if entered.get() {
                        exited.set(true);
                        window.request_close(); // Default last-window policy returns from run().
                    }
                }
            }
        })
        .build(&app)
        .unwrap();
    assert!(window.supports_fullscreen());
    assert!(!window.is_fullscreen());
    window.set_fullscreen(false).unwrap();
    window.show();
    app.post_after(Duration::from_millis(300), {
        let surface = window.surface_clone();
        move || {
            let native =
                app_kit::NSApplication::sharedApplication(objc::MainThreadMarker::new().unwrap());
            // Command-line native test, explicitly launched by the developer.
            #[allow(deprecated)]
            native.activateIgnoringOtherApps(true);
            surface.focus();
            // SAFETY: Click the explicit titlebar button's live target/action pair.
            unsafe {
                fullscreen_button(&surface).performClick(None);
            }
        }
    });
    app.post_after(Duration::from_secs(8), {
        let weak = Rc::downgrade(&app);
        let expired = expired.clone();
        move || {
            expired.set(true);
            if let Some(app) = weak.upgrade() {
                app.quit();
            }
        }
    });
    app.run().unwrap();
    assert!(
        !expired.get(),
        "Native full-screen transition did not complete; desktop must allow foreground activation"
    );
    assert!(entered.get() && exited.get());
    assert!(!window.is_open());
    println!(
        "PASS: titlebar button enters/exits full screen; icon, tooltip and state events stay synchronized"
    );
}
