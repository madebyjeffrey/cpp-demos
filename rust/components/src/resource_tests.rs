//! Inspect private lifetime state without adding application-facing diagnostics.
use crate::*;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    time::{Duration, Instant},
};

struct Metrics;
impl Measure for Metrics {
    fn text(&self, text: &str, _: TextStyle, _: f64) -> TextLayout {
        TextLayout {
            size: Size::new(text.len() as f64 * 8., 18.),
            baseline: 14.,
            lines: vec![],
            placeholders: vec![],
            hit_test: Rc::new(|_| 0),
            range_rects: Rc::new(|_| vec![]),
        }
    }
    fn image(&self, _: &Bitmap) -> Result<Size, Error> {
        Ok(Size::new(1., 1.))
    }
}
fn layout(host: &Host) {
    host.layout(Size::new(400., 240.), &Metrics).unwrap();
}

#[test]
fn replacement_and_removal_release_editor_history_identities_and_callbacks() {
    let generation = State::new(Some(0usize));
    let value = State::new(String::new());
    let witnesses = Rc::new(RefCell::new(Vec::new()));
    let host = Host::new({
        let (generation, value, witnesses) = (generation.clone(), value.clone(), witnesses.clone());
        move || {
            if let Some(key) = generation.get() {
                let witness = Rc::new(());
                witnesses.borrow_mut().push(Rc::downgrade(&witness));
                View::text_field("Draft", value.binding())
                    .key(key)
                    .on_submit(move || {
                        let _keep = &witness;
                    })
            } else {
                View::label("Removed")
            }
        }
    });
    for step in 0..300 {
        generation.set(Some(step));
        layout(&host);
        let id = host.snapshot().unwrap()[0].id;
        host.focus(id).unwrap();
        host.set_text(id, &format!("Edit {step}")).unwrap();
        layout(&host);
        assert_eq!(host.0.editors.borrow().len(), 1);
        assert_eq!(host.0.mounted.borrow().identities.len(), 1);
        assert!(
            witnesses
                .borrow()
                .iter()
                .filter(|w| w.strong_count() > 0)
                .count()
                <= 1
        );
    }
    let id = host.focused().unwrap();
    for step in 0..1_100 {
        host.set_text(id, &format!("Long session {step}")).unwrap();
    }
    assert_eq!(host.0.editors.borrow()[&id].history_len(), (100, 0));
    generation.set(None);
    layout(&host);
    assert!(host.0.editors.borrow().is_empty());
    assert!(host.0.fields.borrow().is_empty());
    assert!(host.0.scroll.borrow().is_empty());
    assert!(host.focused().is_none());
    assert!(witnesses.borrow().iter().all(|w| w.upgrade().is_none()));
    assert!(!host.set_text(id, "Stale editor").unwrap());
}

#[test]
fn repeated_hidden_inactive_and_removed_controls_cancel_work_and_settle() {
    for mode in 0..3 {
        let shown = State::new(true);
        let hidden = State::new(false);
        let query = State::new(String::new());
        let callbacks = State::new(0usize);
        let host = Host::new({
            let (shown, hidden, query, callbacks) = (
                shown.clone(),
                hidden.clone(),
                query.clone(),
                callbacks.clone(),
            );
            move || {
                view! { column { if shown.get() {
                    column(hidden:hidden.get()) {
                        search_field("Search",bind:query,debounce:Duration::from_secs(60),on_query:callbacks.handler(|n,_:String| n.update(|v| *v += 1)));
                        button("Repeat",repeat:Repeat::default(),key:"repeat",on_click:callbacks.action(|n| n.update(|v| *v += 1)));
                        progress();
                    }
                } } }
            }
        });
        let wakes = Rc::new(Cell::new(0));
        host.on_invalidate({
            let wakes = wakes.clone();
            move || wakes.set(wakes.get() + 1)
        });
        for cycle in 0..50 {
            shown.set(true);
            hidden.set(false);
            host.set_active(true);
            layout(&host);
            let field = host
                .snapshot()
                .unwrap()
                .into_iter()
                .find(|n| n.kind == Kind::TextField)
                .unwrap();
            host.focus(field.id).unwrap();
            host.set_text(field.id, &format!("Query {cycle}")).unwrap();
            layout(&host);
            assert!(host.next_deadline().is_some());
            assert!(host.needs_animation());
            let button = host
                .snapshot()
                .unwrap()
                .into_iter()
                .find(|n| n.key.as_deref() == Some("repeat"))
                .unwrap();
            host.pointer(Pointer {
                kind: PointerKind::Down,
                position: Point {
                    x: button.bounds.x + 5.,
                    y: button.bounds.y + 5.,
                },
                button: 0,
                modifiers: Modifiers::default(),
            })
            .unwrap();
            match mode {
                0 => host.set_active(false),
                1 => hidden.set(true),
                _ => shown.set(false),
            }
            layout(&host);
            let callbacks_before = callbacks.get();
            assert!(host.next_deadline().is_none(), "mode {mode}");
            assert!(!host.needs_animation(), "mode {mode}");
            let baseline = wakes.get();
            for tick in 1..10 {
                host.run_due_at(Instant::now() + Duration::from_secs(120 * tick));
                assert!(host.next_deadline().is_none());
            }
            assert_eq!(wakes.get(), baseline, "idle work woke host in mode {mode}");
            assert_eq!(callbacks.get(), callbacks_before);
        }
    }
}
