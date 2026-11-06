use scars_ui::*;
use std::{cell::RefCell, rc::Rc};
struct Metrics;
impl Measure for Metrics {
    fn text(&self, text: &str, _: TextStyle, _: f64) -> TextLayout {
        TextLayout {
            size: Size::new(text.len() as f64 * 7., 18.),
            baseline: 14.,
            lines: vec![],
            placeholders: vec![],
            hit_test: Rc::new(|_| 0),
            range_rects: Rc::new(|_| vec![]),
        }
    }
    fn image(&self, _: &Bitmap) -> Result<Size, Error> {
        Ok(Size::default())
    }
}
fn actions() -> Vec<AlertAction> {
    vec![
        AlertAction::new("cancel", "Cancel").role(AlertActionRole::Cancel),
        AlertAction::new("delete", "Delete").role(AlertActionRole::Destructive),
        AlertAction::new("save", "Save").role(AlertActionRole::Default),
    ]
}
fn alert() -> (Alert, State<Vec<AlertResult>>) {
    let results = State::new(vec![]);
    let a = Alert::new(
        "Save changes?",
        actions(),
        results.handler(|s, r| s.update(|v| v.push(r))),
    )
    .unwrap();
    (a, results)
}
fn layout(host: &Host) {
    host.layout(Size::new(600., 400.), &Metrics).unwrap();
}
fn key(host: &Host, key: Key) {
    assert!(
        host.key(KeyEvent {
            key,
            down: true,
            repeat: false,
            modifiers: Default::default()
        })
        .unwrap()
    );
}
fn button(host: &Host, text: &str) -> NodeId {
    host.snapshot()
        .unwrap()
        .iter()
        .find(|n| n.kind == Kind::Button && n.text == text)
        .unwrap()
        .id
}
#[test]
fn transaction_publishes_before_reentrant_completion_and_observers_release_first() {
    let retained = Rc::new(RefCell::new(None::<Alert>));
    let order = State::new(vec![]);
    let a = Alert::new("Save?", actions(), {
        let retained = retained.clone();
        let order = order.clone();
        move |result| {
            let a = retained.borrow().as_ref().unwrap().clone();
            assert_eq!(a.result(), Some(result));
            assert!(!a.dismiss());
            assert!(!a.choose("delete"));
            order.update(|v| v.push("complete"));
            retained.borrow_mut().take();
        }
    })
    .unwrap();
    retained.replace(Some(a.clone()));
    let _sub = a.observe_completion(order.handler(|s, _| s.update(|v| v.push("release"))));
    assert!(a.choose("save"));
    assert_eq!(order.get(), ["release", "complete"]);
    assert!(!a.cancel());
    assert!(!a.dismiss());
    let late = State::new(0);
    let _late = a.observe_completion(late.handler(|s, _| s.set(s.get() + 1)));
    assert_eq!(late.get(), 1);
}
#[test]
fn roles_validate_and_disabled_actions_cannot_complete() {
    for actions in [
        vec![AlertAction::new("", "Bad")],
        vec![AlertAction::new("a", " ")],
        vec![AlertAction::new("a", "A"), AlertAction::new("a", "B")],
        vec![
            AlertAction::new("a", "A").role(AlertActionRole::Default),
            AlertAction::new("b", "B").role(AlertActionRole::Default),
        ],
    ] {
        assert!(
            Alert::new("Invalid", actions, |_| panic!(
                "invalid transaction must not exist"
            ))
            .is_err()
        );
    }
    let a = Alert::new(
        "Disabled",
        vec![
            AlertAction::new("cancel", "Cancel")
                .role(AlertActionRole::Cancel)
                .enabled(false),
        ],
        |_| {},
    )
    .unwrap();
    assert!(!a.choose("cancel"));
    assert!(!a.choose("missing"));
    assert!(a.cancel());
    assert_eq!(
        a.result(),
        Some(AlertResult {
            action: None,
            reason: AlertReason::Cancel
        })
    );
    let empty = Alert::new("Empty", vec![], |_| {}).unwrap();
    assert!(empty.dismiss());
}
#[test]
fn escape_selects_cancel_return_selects_default_and_completed_controls_are_inert() {
    for (k, expected) in [(Key::Escape, "cancel"), (Key::Enter, "save")] {
        let (a, results) = alert();
        let host = Host::new({
            let a = a.clone();
            move || a.view(View::label("Unsaved work"))
        });
        layout(&host);
        let delete = button(&host, "Delete");
        host.focus(delete).unwrap();
        key(&host, k);
        layout(&host);
        assert_eq!(a.result().unwrap().action.as_deref(), Some(expected));
        assert!(!host.activate(delete).unwrap());
        host.dismiss_overlays(DismissReason::Unmounted);
        assert_eq!(results.get().len(), 1);
    }
    let a = Alert::new(
        "No default",
        vec![AlertAction::new("delete", "Delete").role(AlertActionRole::Destructive)],
        |_| {},
    )
    .unwrap();
    let host = Host::new({
        let a = a.clone();
        move || a.view(View::label("Body"))
    });
    layout(&host);
    host.focus(button(&host, "Delete")).unwrap();
    key(&host, Key::Enter);
    assert!(!a.completed());
    key(&host, Key::Space);
    assert!(a.completed());
}
#[test]
fn modal_isolates_parent_restores_focus_and_survives_outside_click_and_deactivation() {
    let show = State::new(false);
    let (a, results) = alert();
    let host = Host::new({
        let a = a.clone();
        let show = show.clone();
        move || {
            View::layer(vec![
                View::button("Parent", || {}).key("parent"),
                if show.get() {
                    a.view(View::label("Content"))
                } else {
                    View::fragment(vec![])
                },
            ])
        }
    });
    layout(&host);
    let parent = button(&host, "Parent");
    host.focus(parent).unwrap();
    show.set(true);
    layout(&host);
    assert!(!host.activate(parent).unwrap());
    host.pointer(Pointer {
        kind: PointerKind::Down,
        position: Point { x: 2., y: 2. },
        button: 0,
        modifiers: Default::default(),
    })
    .unwrap();
    host.set_active(false);
    layout(&host);
    assert!(!a.completed());
    host.set_active(true);
    key(&host, Key::Escape);
    layout(&host);
    assert_eq!(host.focused(), Some(parent));
    assert_eq!(results.get().len(), 1);
}
#[test]
fn removal_unmount_close_dismiss_and_last_handle_drop_complete_once_with_distinct_reasons() {
    let (a, results) = alert();
    let show = State::new(true);
    let host = Host::new({
        let a = a.clone();
        let show = show.clone();
        move || {
            if show.get() {
                a.view(View::label("Body"))
            } else {
                View::fragment(vec![])
            }
        }
    });
    layout(&host);
    show.set(false);
    layout(&host);
    assert_eq!(results.get()[0].reason, AlertReason::OwnerRemoved);
    drop(host);
    assert_eq!(results.get().len(), 1);
    let (a, results) = alert();
    let host = Host::new({
        let a = a.clone();
        move || a.sheet_view(View::label("Body"))
    });
    layout(&host);
    drop(host);
    assert_eq!(results.get()[0].reason, AlertReason::Unmounted);
    a.dismiss();
    assert_eq!(results.get().len(), 1);
    for reason in [AlertReason::Closed, AlertReason::Dismissed] {
        let (a, results) = alert();
        assert!(a.dismiss_with(reason));
        assert!(!a.cancel());
        assert_eq!(results.get()[0].reason, reason);
    }
    let (a, results) = alert();
    let clone = a.clone();
    drop(a);
    assert!(results.get().is_empty());
    drop(clone);
    assert_eq!(results.get()[0].reason, AlertReason::Destroyed);
}
#[test]
fn ime_return_commits_without_submitting_multiline_keeps_return_and_cancel_discards_preedit() {
    let (a, results) = alert();
    let text = State::new("Name".to_string());
    let host = Host::new({
        let a = a.clone();
        let text = text.clone();
        move || a.view(view! {text_field("Name",bind:text,key:"name");})
    });
    layout(&host);
    let field = host
        .snapshot()
        .unwrap()
        .iter()
        .find(|n| n.key.as_deref() == Some("name"))
        .unwrap()
        .id;
    host.focus(field).unwrap();
    host.mark_text("!", 1..1, None).unwrap();
    key(&host, Key::Enter);
    assert!(!a.completed());
    assert!(text.get().contains('!'));
    host.mark_text("未", 3..3, None).unwrap();
    key(&host, Key::Escape);
    assert!(!a.completed());
    key(&host, Key::Escape);
    assert_eq!(results.get().len(), 1);
    assert!(!text.get().contains('未'));
    let (a, _) = alert();
    let text = State::new(String::new());
    let host = Host::new({
        let a = a.clone();
        let text = text.clone();
        move || a.view(view! {text_editor("Notes",bind:text);})
    });
    layout(&host);
    let field = host
        .snapshot()
        .unwrap()
        .iter()
        .find(|n| n.kind == Kind::TextField)
        .unwrap()
        .id;
    host.focus(field).unwrap();
    key(&host, Key::Enter);
    assert_eq!(text.get(), "\n");
    assert!(!a.completed());
}
#[test]
fn new_transaction_has_new_control_identity_and_render_queries_remain_pure() {
    let (first, results) = alert();
    let current = State::new(first.clone());
    let host = Host::new({
        let current = current.clone();
        move || current.get().view(View::label("Content"))
    });
    layout(&host);
    let old = button(&host, "Save");
    let (second, _) = alert();
    current.set(second.clone());
    layout(&host);
    assert!(!host.activate(old).unwrap());
    assert!(!second.completed());
    assert_eq!(results.get().len(), 1);
    let host = Host::new({
        let second = second.clone();
        move || {
            second.dismiss();
            View::label("Bad")
        }
    });
    assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| host.snapshot())).is_err());
    assert!(!second.completed());
}
