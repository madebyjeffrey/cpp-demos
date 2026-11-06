//! Visible component work must settle after hiding, deactivation and unmount.
use super::support::run_for;
use scars_window::{Application, Window, skia, ui::*};
use std::{
    cell::Cell,
    rc::Rc,
    time::{Duration, Instant},
};

pub fn run(app: &Application) {
    for mode in ["hidden", "inactive", "unmounted"] {
        let hidden = State::new(false);
        let query = State::new(String::new());
        let calls = State::new(0usize);
        let frames = Rc::new(Cell::new(0usize));
        let window = Window::builder(format!("Resource lifecycle: {mode}"))
            .size(520., 400.)
            .on_draw({ let frames = frames.clone(); move |_, frame| {
                frames.set(frames.get() + 1);
                frame.canvas().clear(skia::Color::BLACK);
            } })
            .view({
                let (hidden, query, calls) = (hidden.clone(), query.clone(), calls.clone());
                move || view! { column(padding:20,hidden:hidden.get()) {
                    search_field("Search",bind:query,debounce:Duration::from_secs(60),on_query:calls.handler(|n,_:String| n.update(|v| *v += 1)));
                    button("Repeat",key:"repeat",repeat:Repeat::default(),on_click:calls.action(|n| n.update(|v| *v += 1)));
                    progress();
                } }
            })
            .build(app).unwrap();
        window.show();
        run_for(app, Duration::from_millis(300));
        assert!(
            frames.get() > 1,
            "{mode}: visible animation did not deliver Metal frames"
        );
        let host = window.view_host().unwrap();
        assert!(host.needs_animation());
        let field = host
            .snapshot()
            .unwrap()
            .into_iter()
            .find(|n| n.kind == Kind::TextField)
            .unwrap();
        host.focus(field.id).unwrap();
        host.insert_text("Pending query", None).unwrap();
        // Let the native redraw callback rebuild and lay out the invalidated tree
        // before querying geometry to synthesize the captured pointer press.
        run_for(app, Duration::from_millis(80));
        let button = host
            .snapshot()
            .unwrap()
            .into_iter()
            .find(|n| n.key.as_deref() == Some("repeat"))
            .unwrap();
        let point = Point {
            x: button.bounds.x + 5.,
            y: button.bounds.y + 5.,
        };
        assert_eq!(
            host.hit_test(point).unwrap(),
            Some(button.id),
            "{mode}: button {:?} outside hit area {:?} clipped by {:?}",
            button.bounds,
            button.bounds,
            button.clip,
        );
        let hit = host
            .pointer(Pointer {
                kind: PointerKind::Down,
                position: point,
                button: 0,
                modifiers: Modifiers::default(),
            })
            .unwrap();
        assert!(
            hit,
            "{mode}: repeat control did not receive the pointer press"
        );
        assert!(host.next_deadline().is_some());
        match mode {
            "hidden" => hidden.set(true),
            "inactive" => window.hide().unwrap(),
            "unmounted" => window.clear_view(),
            _ => unreachable!(),
        }
        run_for(app, Duration::from_millis(200));
        assert!(
            host.next_deadline().is_none(),
            "{mode}: work remains scheduled"
        );
        assert!(
            !host.needs_animation(),
            "{mode}: animation remains scheduled"
        );
        let baseline = frames.get();
        let callbacks = calls.get();
        host.run_due_at(Instant::now() + Duration::from_secs(120));
        run_for(app, Duration::from_millis(200));
        assert_eq!(
            frames.get(),
            baseline,
            "{mode}: idle display work did not settle"
        );
        assert_eq!(calls.get(), callbacks, "{mode}: cancelled work ran later");
        window.close();
        println!(
            "Resource lifecycle {mode}: {baseline} frames before settling; zero idle frames/callbacks"
        );
    }
}
