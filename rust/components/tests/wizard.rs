use scars_ui::*;
use std::{cell::Cell, rc::Rc};
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
fn layout(h: &Host) {
    h.layout(Size::new(600., 440.), &Metrics).unwrap();
}
fn action(h: &Host, label: &str) -> Node {
    h.snapshot()
        .unwrap()
        .into_iter()
        .find(|n| n.kind == Kind::Button && n.text == label)
        .unwrap()
}
fn keyed(h: &Host, key: &str) -> Node {
    h.snapshot()
        .unwrap()
        .into_iter()
        .find(|n| n.key.as_deref() == Some(key))
        .unwrap()
}
fn steps(name: &State<String>, destination: &State<String>) -> Vec<WizardStep> {
    vec![
        WizardStep::new("name", "Identity").validate({
            let name = name.clone();
            move || {
                if name.get().is_empty() {
                    Err("A name is required.".into())
                } else {
                    Ok(())
                }
            }
        }),
        WizardStep::new("destination", "Destination").validate({
            let destination = destination.clone();
            move || {
                if destination.get().is_empty() {
                    Err("Choose a destination.".into())
                } else {
                    Ok(())
                }
            }
        }),
        WizardStep::new("review", "Review"),
    ]
}
#[test]
fn validation_gates_forward_progress_finish_rechecks_prior_steps_and_callbacks_report_accepted_changes()
 {
    let name = State::new(String::new());
    let destination = State::new(String::new());
    let current = State::new("name".to_owned());
    let changes = State::new(vec![]);
    let finished = State::new(0);
    let cancelled = State::new(0);
    let w = Wizard::bound(current.binding(), steps(&name, &destination))
        .unwrap()
        .on_change(changes.handler(|s, c: WizardChange| s.update(|v| v.push(c))))
        .on_finish(finished.action(|s| s.update(|n| *n += 1)))
        .on_cancel(cancelled.action(|s| s.update(|n| *n += 1)));
    assert_eq!(w.current(), Some("name".into()));
    assert!(!w.info().can_back);
    assert!(!w.next());
    assert!(!w.go_to("review"));
    assert!(!w.finish());
    name.set("Kestrel".into());
    assert!(w.next());
    assert_eq!(
        changes.get(),
        vec![WizardChange {
            from: "name".into(),
            to: "destination".into()
        }]
    );
    assert!(!w.next());
    assert!(w.back());
    assert_eq!(name.get(), "Kestrel");
    assert!(w.next());
    destination.set("outer".into());
    assert!(w.next());
    assert!(w.finish());
    assert_eq!(finished.get(), 1);
    name.set(String::new());
    let info = w.info();
    assert!(!info.can_advance);
    assert_eq!(info.issue.unwrap().step_id, "name");
    assert!(!w.finish());
    assert_eq!(finished.get(), 1);
    assert!(w.go_to("name"));
    assert!(w.cancel());
    assert_eq!(cancelled.get(), 1);
    assert_eq!(w.current(), Some("name".into()));
    assert!(!w.go_to("missing"));
    assert!(!w.go_to("name"));
}
#[test]
fn back_review_and_final_validation_policies_are_independent() {
    let name = State::new(String::new());
    let destination = State::new("ok".into());
    let current = State::new("review".to_owned());
    let finished = State::new(0);
    let w = Wizard::bound(current.binding(), steps(&name, &destination))
        .unwrap()
        .on_finish(finished.action(|s| s.update(|n| *n += 1)));
    let locked = w
        .clone()
        .options(WizardOptions {
            allow_back: false,
            ..Default::default()
        })
        .unwrap();
    assert!(!locked.back());
    assert!(!locked.go_to("name"));
    let no_review = w
        .clone()
        .options(WizardOptions {
            review_previous: false,
            ..Default::default()
        })
        .unwrap();
    assert!(!no_review.go_to("name"));
    assert!(no_review.back());
    current.set("review".into());
    let current_only = w
        .options(WizardOptions {
            validate_all_on_finish: false,
            ..Default::default()
        })
        .unwrap();
    assert!(current_only.finish());
    assert_eq!(finished.get(), 1);
    let no_finish = Wizard::new("review", vec![WizardStep::new("review", "Review")]).unwrap();
    assert!(!no_finish.info().can_advance);
    assert!(!no_finish.finish());
    assert!(!no_finish.cancel());
}
#[test]
fn binding_rejection_normalization_and_reentrant_callbacks_are_safe() {
    let rejected = State::new(0);
    let w = Wizard::bound(
        Binding::new(|| "a".to_owned(), |_| {}),
        vec![WizardStep::new("a", "A"), WizardStep::new("b", "B")],
    )
    .unwrap()
    .on_change(rejected.handler(|s, _: WizardChange| s.update(|n| *n += 1)));
    assert!(!w.next());
    assert_eq!(rejected.get(), 0);
    let source = State::new("a".to_owned());
    let changes = State::new(vec![]);
    let w = Wizard::bound(
        Binding::new(
            {
                let source = source.clone();
                move || source.get()
            },
            {
                let source = source.clone();
                move |_| source.set("c".into())
            },
        ),
        vec![
            WizardStep::new("a", "A"),
            WizardStep::new("b", "B"),
            WizardStep::new("c", "C"),
        ],
    )
    .unwrap()
    .on_change(changes.handler(|s, c: WizardChange| s.update(|v| v.push(c))));
    assert!(w.next());
    assert_eq!(changes.get()[0].to, "c");
    let base = Wizard::new(
        "a",
        vec![
            WizardStep::new("a", "A"),
            WizardStep::new("b", "B"),
            WizardStep::new("c", "C"),
        ],
    )
    .unwrap();
    let nested = base.clone();
    let w = base.on_change(move |_| assert!(!nested.next()));
    assert!(w.next());
    assert_eq!(w.current(), Some("b".into()));
}
#[test]
fn empty_invalid_changed_and_reordered_steps_keep_stable_ids_without_render_time_repairs() {
    let current = State::new("b".to_owned());
    let w = Wizard::bound(
        current.binding(),
        vec![WizardStep::new("a", "A"), WizardStep::new("b", "B")],
    )
    .unwrap();
    w.set_steps(vec![
        WizardStep::new("b", "Renamed"),
        WizardStep::new("a", "A"),
    ])
    .unwrap();
    assert_eq!(w.index(), Some(0));
    assert_eq!(w.current(), Some("b".into()));
    w.set_steps(vec![WizardStep::new("a", "A")]).unwrap();
    assert_eq!(w.current(), Some("a".into()));
    assert_eq!(current.get(), "b");
    assert!(w.go_to("a"));
    assert_eq!(current.get(), "a");
    w.set_steps(vec![]).unwrap();
    assert!(w.current().is_none());
    assert!(w.index().is_none());
    assert!(!w.next());
    assert!(!w.back());
    assert!(!w.finish());
    let h = Host::new({
        let w = w.clone();
        move || w.navigation()
    });
    layout(&h);
    assert!(h.snapshot().unwrap().iter().any(|n| n.text == "No steps"));
    assert!(
        !h.snapshot()
            .unwrap()
            .iter()
            .any(|n| n.text.contains("1 of 0"))
    );
    assert!(
        w.set_steps(vec![
            WizardStep::new("same", "A"),
            WizardStep::new("same", "B")
        ])
        .is_err()
    );
    assert!(w.current().is_none());
    assert!(Wizard::new("a", vec![WizardStep::new("", "A")]).is_err());
    assert!(
        w.options(WizardOptions {
            next_label: String::new(),
            ..Default::default()
        })
        .is_err()
    );
}
#[test]
fn reactive_validation_updates_controls_and_stale_live_actions_cannot_skip_or_submit_another_step()
{
    let valid = State::new(false);
    let finished = State::new(0);
    let w = Wizard::new(
        "a",
        vec![
            WizardStep::new("a", "A").validate({
                let valid = valid.clone();
                move || {
                    if valid.get() {
                        Ok(())
                    } else {
                        Err("Required".into())
                    }
                }
            }),
            WizardStep::new("b", "B"),
        ],
    )
    .unwrap()
    .on_finish(finished.action(|s| s.update(|n| *n += 1)));
    let h = Host::new({
        let w = w.clone();
        move || w.navigation()
    });
    layout(&h);
    assert!(!action(&h, "Next").enabled);
    valid.set(true);
    layout(&h);
    let old = action(&h, "Next").id;
    assert!(action(&h, "Next").enabled);
    assert!(h.activate(old).unwrap());
    layout(&h);
    assert!(!h.activate(old).unwrap());
    assert_eq!(finished.get(), 0);
    assert!(h.activate(action(&h, "Finish").id).unwrap());
    assert_eq!(finished.get(), 1);
    let source = Rc::new(std::cell::RefCell::new("a".to_owned()));
    let live = Rc::new(Cell::new(true));
    let w = Wizard::bound(
        Binding::new(
            {
                let source = source.clone();
                move || source.borrow().clone()
            },
            {
                let source = source.clone();
                move |s| *source.borrow_mut() = s
            },
        ),
        vec![
            WizardStep::new("a", "A").validate({
                let live = live.clone();
                move || {
                    if live.get() {
                        Ok(())
                    } else {
                        Err("Now invalid".into())
                    }
                }
            }),
            WizardStep::new("b", "B"),
        ],
    )
    .unwrap()
    .on_finish(finished.action(|s| s.update(|n| *n += 1)));
    let h = Host::new({
        let w = w.clone();
        move || w.navigation()
    });
    layout(&h);
    let old = action(&h, "Next").id;
    live.set(false);
    assert!(!h.activate(old).unwrap());
    layout(&h);
    assert!(!action(&h, "Next").enabled);
    live.set(true);
    let enabled = Host::new({
        let w = w.clone();
        move || w.navigation()
    });
    layout(&enabled);
    let old = action(&enabled, "Next").id;
    assert!(action(&enabled, "Next").enabled);
    *source.borrow_mut() = "b".into();
    assert!(!enabled.activate(old).unwrap());
    assert_eq!(finished.get(), 1);
}
#[test]
fn retained_pages_keep_undo_selection_scroll_and_focus_through_next_back_and_step_reordering() {
    let w = Wizard::new(
        "a",
        vec![
            WizardStep::new("a", "Editor"),
            WizardStep::new("b", "Review"),
        ],
    )
    .unwrap()
    .on_finish(|| {});
    let text = State::new("draft".to_owned());
    let h = Host::new({
        let w = w.clone();
        let text = text.clone();
        move || {
            w.view(vec![("a",view!{column{ text_field("Draft",bind:text,key:"draft");scroll(height:100,key:"scroll"){label("Long",height:600);}}}),("b",View::label("Review page"))])
        }
    });
    layout(&h);
    let id = keyed(&h, "draft").id;
    h.focus(id).unwrap();
    h.set_text(id, "edited").unwrap();
    h.select_text(
        id,
        Selection {
            anchor: 1,
            caret: 4,
        },
    )
    .unwrap();
    layout(&h);
    h.scroll_to(keyed(&h, "scroll").id, Point { x: 0., y: 50. })
        .unwrap();
    layout(&h);
    h.focus(action(&h, "Next").id).unwrap();
    assert!(h.activate(action(&h, "Next").id).unwrap());
    layout(&h);
    assert!(keyed(&h, "draft").hidden);
    assert_eq!(h.focused(), Some(action(&h, "Finish").id));
    assert!(h.activate(action(&h, "Back").id).unwrap());
    layout(&h);
    assert_eq!(h.focused(), Some(id));
    assert_eq!(
        h.text_input().unwrap().unwrap().selection,
        Selection {
            anchor: 1,
            caret: 4
        }
    );
    assert_eq!(keyed(&h, "scroll").scroll.unwrap().offset.y, 50.);
    assert!(h.text_command(TextCommand::Undo, false).unwrap());
    assert_eq!(text.get(), "draft");
    w.set_steps(vec![
        WizardStep::new("b", "Review"),
        WizardStep::new("a", "Renamed editor"),
    ])
    .unwrap();
    layout(&h);
    assert_eq!(keyed(&h, "draft").id, id);
    assert_eq!(w.current(), Some("a".into()));
}
#[test]
fn editor_commit_is_revalidated_and_cancel_is_independent_of_invalid_data() {
    let name = State::new("ok".to_owned());
    let cancelled = State::new(0);
    let w = Wizard::new(
        "name",
        vec![
            WizardStep::new("name", "Name").validate({
                let name = name.clone();
                move || {
                    if name.get().is_empty() {
                        Err("Required".into())
                    } else {
                        Ok(())
                    }
                }
            }),
            WizardStep::new("review", "Review"),
        ],
    )
    .unwrap()
    .on_cancel(cancelled.action(|s| s.update(|n| *n += 1)));
    let h = Host::new({
        let w = w.clone();
        let name = name.clone();
        move || {
            w.view(vec![
                ("name", View::text_field("Name", name.binding()).key("name")),
                ("review", View::label("Review")),
            ])
        }
    });
    layout(&h);
    let id = keyed(&h, "name").id;
    h.focus(id).unwrap();
    h.select_text(
        id,
        Selection {
            anchor: 0,
            caret: 2,
        },
    )
    .unwrap();
    h.mark_text("", 0..0, None).unwrap();
    assert!(!h.activate(action(&h, "Next").id).unwrap());
    layout(&h);
    assert_eq!(name.get(), "");
    assert_eq!(w.current(), Some("name".into()));
    assert!(h.activate(action(&h, "Cancel").id).unwrap());
    assert_eq!(cancelled.get(), 1);
}
#[test]
fn validators_are_read_only_and_failed_definition_changes_leave_the_flow_intact() {
    let value = State::new(0);
    let w = Wizard::new(
        "a",
        vec![WizardStep::new("a", "A").validate({
            let value = value.clone();
            move || {
                value.set(1);
                Ok(())
            }
        })],
    )
    .unwrap();
    assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| w.info())).is_err());
    assert_eq!(value.get(), 0);
    value.set(2);
    let w = Wizard::new(
        "a",
        vec![WizardStep::new("a", "A"), WizardStep::new("b", "B")],
    )
    .unwrap();
    let h = Host::new({
        let w = w.clone();
        move || {
            w.view(vec![
                ("a", View::label("A")),
                ("a", View::label("Duplicate")),
            ])
        }
    });
    assert!(h.layout(Size::new(600., 440.), &Metrics).is_err());
    let illegal = Host::new({
        let w = w.clone();
        move || {
            w.set_steps(vec![]).unwrap();
            w.navigation()
        }
    });
    assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| illegal.refresh())).is_err());
    assert_eq!(w.current(), Some("a".into()));
}
#[test]
fn legacy_and_macro_navigation_handle_empty_pages_and_policy_visibility() {
    let index = State::new(50);
    let h = Host::new({
        let index = index.clone();
        move || wizard(&index, vec![])
    });
    layout(&h);
    assert!(h.snapshot().unwrap().iter().any(|n| n.text == "No steps"));
    assert!(!action(&h, "Next").enabled);
    let w = Wizard::new(
        "a",
        vec![WizardStep::new("a", "A"), WizardStep::new("b", "B")],
    )
    .unwrap()
    .options(WizardOptions {
        show_steps: false,
        show_status: false,
        ..Default::default()
    })
    .unwrap();
    let h = Host::new(
        move || view! {wizard_view(&w,vec![("a",View::label("A")),("b",View::label("B"))]);},
    );
    layout(&h);
    assert!(
        !h.snapshot()
            .unwrap()
            .iter()
            .any(|n| matches!(n.key.as_deref(), Some("$steps" | "$status")))
    );
    assert!(action(&h, "Next").enabled);
}
