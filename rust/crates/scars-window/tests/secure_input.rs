//! Opt-in actual Carbon ownership transitions on an awake foreground desktop.
#[cfg(target_os = "macos")]
fn main() {
    if std::env::var_os("SCARS_WINDOW_SECURE_INPUT_TESTS").is_none() {
        println!("Secure-input desktop tests skipped");
        return;
    }
    use scars_window::{ui::*, *};
    use std::{cell::Cell, rc::Rc, time::Duration};
    #[link(name = "Carbon", kind = "framework")]
    unsafe extern "C" {
        fn IsSecureEventInputEnabled() -> bool;
    }
    fn secure() -> bool {
        unsafe { IsSecureEventInputEnabled() }
    }
    let app = Application::named("Secure input transitions").unwrap();
    let passed = Rc::new(Cell::new(false));
    let value = State::new("private".to_owned());
    let readonly = State::new(false);
    let hidden = State::new(false);
    let enabled = State::new(true);
    let window=Window::builder("Secure input transitions").view({let(readonly,hidden,enabled)=(readonly.clone(),hidden.clone(),enabled.clone());move||view!{column{
        password_field("Password",bind:value,key:"password",allow_reveal:true,read_only:readonly.get(),hidden:hidden.get(),enabled:enabled.get());button("Other",key:"other",on_click:||{});
    }}}).build(&app).unwrap();
    window.show();
    app.post({
        let surface = window.surface_clone();
        move || {
            #[allow(deprecated)]
            app_kit::NSApplication::sharedApplication(objc::MainThreadMarker::new().unwrap())
                .activateIgnoringOtherApps(true);
            surface.focus();
        }
    });
    app.post_after(Duration::from_millis(250),{let surface=window.surface_clone();let passed=passed.clone();move||{
        struct Close(Surface);impl Drop for Close{fn drop(&mut self){self.0.close();}}
        let cleanup=Close(surface.clone());
        assert!(app_kit::NSApplication::sharedApplication(objc::MainThreadMarker::new().unwrap()).isActive() && surface.native_window().isKeyWindow(),"Secure-input audit requires an awake foreground desktop");
        assert!(!secure(),"Secure input is already owned outside this test");
        let host=surface.view_host().unwrap();let nodes=host.snapshot().unwrap();
        let password=nodes.iter().find(|n|n.key.as_deref()==Some("password")).unwrap().id;
        let other=nodes.iter().find(|n|n.key.as_deref()==Some("other")).unwrap().id;
        let sync=||surface.invalidate_text_input();
        host.focus(password).unwrap();host.set_active(true);sync();assert!(secure());
        host.set_password_revealed(password,true).unwrap();sync();assert!(secure());
        readonly.set(true);sync();assert!(!secure());readonly.set(false);host.focus(password).unwrap();sync();assert!(secure());
        hidden.set(true);sync();assert!(!secure());hidden.set(false);host.focus(password).unwrap();sync();assert!(secure());
        enabled.set(false);sync();assert!(!secure());enabled.set(true);host.focus(password).unwrap();sync();assert!(secure());
        host.focus(other).unwrap();sync();assert!(!secure());host.focus(password).unwrap();sync();assert!(secure());
        host.set_active(false);sync();assert!(!secure());host.set_active(true);sync();assert!(secure());
        surface.hide().unwrap();assert!(!secure());surface.focus();host.focus(password).unwrap();host.set_active(true);sync();assert!(secure());
        surface.clear_view();assert!(!secure());assert!(!host.text_input().unwrap().is_some_and(|i|i.active));
        surface.set_view(||view!{password_field("Replacement",bind:State::new("value".into()),autofocus:true);}).unwrap();sync();assert!(secure());
        drop(cleanup);assert!(!secure());passed.set(true);
    }});
    app.quit_after(Duration::from_millis(750));
    app.run().unwrap();
    assert!(passed.get());
    println!("Secure-input ownership transitions passed");
}
#[cfg(not(target_os = "macos"))]
fn main() {}
