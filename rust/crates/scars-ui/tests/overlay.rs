use scars_ui::*;
use std::{
    cell::RefCell,
    rc::Rc,
    time::{Duration, Instant},
};
struct Metrics;
impl Measure for Metrics {
    fn text(&self, text: &str, _: TextStyle, width: f64) -> TextLayout {
        let natural = text.chars().count() as f64 * 8.;
        let width = if width.is_finite() {
            width.max(1.)
        } else {
            natural.max(1.)
        };
        TextLayout {
            size: Size::new(natural.min(width), (natural / width).ceil().max(1.) * 18.),
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
    h.layout(Size::new(640., 480.), &Metrics).unwrap();
}
fn keyed(h: &Host, key: &str) -> Node {
    h.snapshot()
        .unwrap()
        .into_iter()
        .find(|n| n.key.as_deref() == Some(key))
        .unwrap()
}
fn focused(h: &Host) -> Option<String> {
    h.snapshot()
        .unwrap()
        .into_iter()
        .find(|n| Some(n.id) == h.focused())
        .map(|n| n.text)
}
fn key(h: &Host, key: Key) {
    h.key(KeyEvent {
        key,
        down: true,
        repeat: false,
        modifiers: Default::default(),
    })
    .unwrap();
    layout(h);
}
fn pointer(h: &Host, kind: PointerKind, position: Point, button: i32, control: bool) {
    h.pointer(Pointer {
        kind,
        position,
        button,
        modifiers: Modifiers {
            control,
            ..Default::default()
        },
    })
    .unwrap();
    layout(h);
}
fn center(n: &Node) -> Point {
    Point {
        x: n.bounds.x + n.bounds.width / 2.,
        y: n.bounds.y + n.bounds.height / 2.,
    }
}
fn log_popup(log: &Rc<RefCell<Vec<DismissReason>>>) -> Popup {
    let log = log.clone();
    Popup::new().on_dismiss(move |reason| log.borrow_mut().push(reason))
}
#[test]
fn nested_sibling_portals_restore_each_invoking_control_and_close_parent_descendants() {
    let parent = Popup::new();
    let child = Popup::new();
    let menu = Menu::new();
    let text = State::new("draft".to_owned());
    let h = Host::new({
        let parent = parent.clone();
        let child = child.clone();
        let menu = menu.clone();
        move || {
            View::layer(vec![
                parent.button("Open", "open"),
                parent.popover(
                    "open",
                    View::panel(vec![
                        View::text_field("Draft", text.binding()).key("draft"),
                        child.button("Nested", "nested"),
                        menu.button("Menu", "menu"),
                    ]),
                ),
                child.popover("nested", View::button("Child", || {}).key("child")),
                menu.view("menu", vec![MenuAction::new("Run", || {})]),
            ])
        }
    });
    layout(&h);
    h.focus(keyed(&h, "open").id).unwrap();
    parent.show();
    layout(&h);
    assert_eq!(focused(&h), Some("Draft".into()));
    h.focus(keyed(&h, "nested").id).unwrap();
    child.show();
    layout(&h);
    assert_eq!(focused(&h), Some("Child".into()));
    key(&h, Key::Escape);
    assert!(!child.open.get());
    assert!(parent.open.get());
    assert_eq!(focused(&h), Some("Nested".into()));
    h.focus(keyed(&h, "menu").id).unwrap();
    menu.show();
    layout(&h);
    key(&h, Key::Enter);
    assert_eq!(focused(&h), Some("Menu".into()));
    assert!(parent.open.get());
    h.focus(keyed(&h, "nested").id).unwrap();
    child.show();
    layout(&h);
    parent.dismiss();
    layout(&h);
    assert!(!child.open.get());
    assert_eq!(focused(&h), Some("Open".into()));
    parent.show();
    layout(&h);
    assert!(!child.open.get());
}
#[test]
fn placement_sizing_and_portal_clipping_fit_all_sides_and_tiny_resizes() {
    for placement in [
        OverlayPlacement::Below,
        OverlayPlacement::Above,
        OverlayPlacement::Left,
        OverlayPlacement::Right,
    ] {
        let popup = Popup::new().options(OverlayOptions {
            placement,
            width: Some(220.),
            max_height: 100.,
            gap: 9.,
            margin: 8.,
            ..Default::default()
        });
        popup.show();
        let h = Host::new(move || {
            View::layer(vec![
                View::scroll(vec![View::button("Anchor", || {}).key("anchor")]).height(45),
                popup.popover(
                    "anchor",
                    View::column(
                        (0..20)
                            .map(|i| {
                                View::button(format!("Item {i}"), || {})
                                    .key(i.to_string())
                                    .shrink(0.)
                            })
                            .collect(),
                    )
                    .gap(2.),
                ),
            ])
        });
        h.layout(Size::new(300., 220.), &Metrics).unwrap();
        let scopes = h.snapshot().unwrap();
        let root = scopes.iter().find(|n| n.kind == Kind::Modal).unwrap();
        let panel = scopes.iter().find(|n| n.parent == Some(root.id)).unwrap();
        assert_eq!(panel.bounds.width, 220.);
        assert_eq!(panel.bounds.height, 100.);
        assert_eq!(panel.bounds, panel.clip);
        assert!(panel.bounds.y >= 8.);
        assert!(panel.bounds.y + panel.bounds.height > 45.);
        let last = keyed(&h, "19");
        h.focus(last.id).unwrap();
        h.layout(Size::new(300., 220.), &Metrics).unwrap();
        assert!(keyed(&h, "19").clip.height > 0.);
        assert_eq!(keyed(&h, "anchor").bounds.y, 0.);
        h.layout(Size::new(30., 20.), &Metrics).unwrap();
        for n in h
            .snapshot()
            .unwrap()
            .iter()
            .filter(|n| n.kind == Kind::Modal || n.parent == Some(root.id))
        {
            assert!(
                n.clip.x >= 0.
                    && n.clip.y >= 0.
                    && n.clip.x + n.clip.width <= 30.
                    && n.clip.y + n.clip.height <= 20.
            );
        }
    }
}
#[test]
fn preserve_focus_allows_editor_input_and_policies_consume_outside_click_without_activation() {
    let popup = Popup::new().options(OverlayOptions {
        focus: OverlayFocus::Preserve,
        dismiss_on_escape: false,
        dismiss_on_outside: false,
        dismiss_on_deactivate: false,
        ..Default::default()
    });
    let text = State::new(String::new());
    let clicks = State::new(0);
    let h = Host::new({
        let popup = popup.clone();
        let text = text.clone();
        let clicks = clicks.clone();
        move || {
            View::layer(vec![
                View::column(vec![
                    View::text_field("Owner", text.binding()).key("owner"),
                    View::button("Behind", clicks.action(|s| s.update(|n| *n += 1))).key("behind"),
                ]),
                popup.popover("owner", View::button("Suggestion", || {}).key("suggestion")),
            ])
        }
    });
    layout(&h);
    h.focus(keyed(&h, "owner").id).unwrap();
    popup.show();
    layout(&h);
    assert_eq!(focused(&h), Some("Owner".into()));
    h.insert_text("new", None).unwrap();
    assert_eq!(text.get(), "new");
    key(&h, Key::Escape);
    assert!(popup.open.get());
    let p = center(&keyed(&h, "behind"));
    pointer(&h, PointerKind::Down, p, 0, false);
    pointer(&h, PointerKind::Up, p, 0, false);
    assert!(popup.open.get());
    assert_eq!(clicks.get(), 0);
    let p = center(&keyed(&h, "suggestion"));
    pointer(&h, PointerKind::Down, p, 0, false);
    pointer(&h, PointerKind::Up, p, 0, false);
    assert_eq!(focused(&h), Some("Suggestion".into()));
    popup.dismiss();
    layout(&h);
    assert_eq!(focused(&h), Some("Owner".into()));
    popup.show();
    layout(&h);
    h.set_active(false);
    layout(&h);
    assert!(popup.open.get());
    h.set_active(true);
}
#[test]
fn dismissal_reports_once_for_external_close_outside_escape_owner_loss_deactivation_and_drop() {
    let log = Rc::new(RefCell::new(vec![]));
    let popup = log_popup(&log);
    let present = State::new(true);
    let enabled = State::new(true);
    let include = State::new(true);
    let h = Host::new({
        let popup = popup.clone();
        let present = present.clone();
        let enabled = enabled.clone();
        let include = include.clone();
        move || {
            View::layer(vec![
                View::button("Anchor", || {})
                    .key("anchor")
                    .visible(present.get())
                    .enabled(enabled.get()),
                if include.get() {
                    popup.popover("anchor", View::button("Body", || {}))
                } else {
                    View::fragment(vec![])
                },
            ])
        }
    });
    layout(&h);
    popup.show();
    layout(&h);
    popup.dismiss();
    layout(&h);
    assert_eq!(*log.borrow(), [DismissReason::Programmatic]);
    layout(&h);
    popup.show();
    layout(&h);
    key(&h, Key::Escape);
    assert_eq!(log.borrow().last(), Some(&DismissReason::Escape));
    popup.show();
    layout(&h);
    pointer(&h, PointerKind::Down, Point { x: 600., y: 450. }, 0, false);
    pointer(&h, PointerKind::Up, Point { x: 600., y: 450. }, 0, false);
    assert_eq!(log.borrow().last(), Some(&DismissReason::Outside));
    popup.show();
    layout(&h);
    enabled.set(false);
    layout(&h);
    assert!(!popup.open.get());
    assert_eq!(log.borrow().last(), Some(&DismissReason::OwnerRemoved));
    enabled.set(true);
    popup.show();
    layout(&h);
    present.set(false);
    layout(&h);
    assert!(!popup.open.get());
    present.set(true);
    popup.show();
    layout(&h);
    include.set(false);
    layout(&h);
    assert!(!popup.open.get());
    assert_eq!(log.borrow().last(), Some(&DismissReason::OwnerRemoved));
    include.set(true);
    popup.show();
    layout(&h);
    h.set_active(false);
    layout(&h);
    assert!(!popup.open.get());
    assert_eq!(log.borrow().last(), Some(&DismissReason::Deactivated));
    h.set_active(true);
    popup.show();
    layout(&h);
    drop(h);
    assert!(!popup.open.get());
    assert_eq!(
        *log.borrow(),
        [
            DismissReason::Programmatic,
            DismissReason::Escape,
            DismissReason::Outside,
            DismissReason::OwnerRemoved,
            DismissReason::OwnerRemoved,
            DismissReason::OwnerRemoved,
            DismissReason::Deactivated,
            DismissReason::Unmounted
        ]
    );
}
#[test]
fn escape_cancels_preedit_before_dismissing_the_popover_and_close_callbacks_can_reopen() {
    let popup = Popup::new();
    let text = State::new("original".to_owned());
    let h = Host::new({
        let popup = popup.clone();
        let text = text.clone();
        move || {
            View::layer(vec![
                popup.button("Open", "open"),
                popup.popover("open", View::text_field("Text", text.binding())),
            ])
        }
    });
    layout(&h);
    popup.show();
    layout(&h);
    h.mark_text("日本", 0.."日本".len(), None).unwrap();
    key(&h, Key::Escape);
    assert!(popup.open.get());
    assert!(h.text_input().unwrap().unwrap().marked.is_none());
    assert_eq!(text.get(), "original");
    key(&h, Key::Escape);
    assert!(!popup.open.get());
    let popup = Popup::new();
    let count = State::new(0);
    let popup = popup.clone().on_dismiss({
        let popup = popup.clone();
        let count = count.clone();
        move |_| {
            count.update(|n| *n += 1);
            if count.get() == 1 {
                popup.show();
            }
        }
    });
    let h = Host::new({
        let popup = popup.clone();
        move || {
            View::layer(vec![
                popup.button("Open", "open"),
                popup.popover("open", View::button("Body", || {})),
            ])
        }
    });
    popup.show();
    layout(&h);
    key(&h, Key::Escape);
    assert!(popup.open.get());
    assert!(h.active_modal().is_some());
    key(&h, Key::Escape);
    assert!(!popup.open.get());
    assert_eq!(count.get(), 2);
}
#[test]
fn context_menu_uses_nearest_owner_actual_hit_control_click_keyboard_and_owner_lifetime() {
    let menu = Menu::new();
    let count = State::new(0);
    let visible = State::new(true);
    let h = Host::new({
        let menu = menu.clone();
        let count = count.clone();
        let visible = visible.clone();
        move || {
            View::column(vec![
                View::panel(vec![View::button("Child", || {}).key("child")])
                    .key("owner")
                    .visible(visible.get())
                    .context_menu(
                        &menu,
                        vec![MenuAction::new(
                            "Run",
                            count.action(|s| s.update(|n| *n += 1)),
                        )],
                    ),
                View::button("Other", || {}).key("other"),
            ])
        }
    });
    layout(&h);
    pointer(&h, PointerKind::Down, center(&keyed(&h, "other")), 1, false);
    assert!(!menu.open.get());
    let p = center(&keyed(&h, "child"));
    pointer(&h, PointerKind::Down, p, 1, false);
    assert!(menu.open.get());
    let root = h
        .snapshot()
        .unwrap()
        .into_iter()
        .find(|n| n.menu_root)
        .unwrap();
    assert_eq!(root.overlay_point, Some(p));
    key(&h, Key::Enter);
    assert_eq!(count.get(), 1);
    pointer(&h, PointerKind::Down, p, 0, true);
    pointer(&h, PointerKind::Up, p, 0, false);
    assert!(menu.open.get());
    key(&h, Key::Escape);
    h.focus(keyed(&h, "child").id).unwrap();
    h.key(KeyEvent {
        key: Key::F10,
        down: true,
        repeat: false,
        modifiers: Modifiers {
            shift: true,
            ..Default::default()
        },
    })
    .unwrap();
    layout(&h);
    assert!(menu.open.get());
    visible.set(false);
    layout(&h);
    assert!(!menu.open.get());
}
#[test]
fn tooltip_delay_keyboard_help_suppression_options_geometry_and_owner_removal_settle() {
    let log = Rc::new(RefCell::new(vec![]));
    let visible = State::new(true);
    let text =
        State::new("A long tooltip with enough words to wrap across several lines".to_owned());
    let h = Host::new({
        let log = log.clone();
        let visible = visible.clone();
        let text = text.clone();
        move || {
            View::button("Help", || {})
                .key("help")
                .visible(visible.get())
                .tooltip(text.get())
                .tooltip_options(TooltipOptions {
                    delay: Duration::from_millis(100),
                    max_width: 140.,
                    placement: OverlayPlacement::Right,
                    margin: 8.,
                    ..Default::default()
                })
                .on_tooltip_dismiss({
                    let log = log.clone();
                    move |reason| log.borrow_mut().push(reason)
                })
        }
    });
    layout(&h);
    assert_eq!(keyed(&h, "help").help, text.get());
    h.focus(keyed(&h, "help").id).unwrap();
    let now = Instant::now();
    h.advance_overlays_at(now);
    assert!(h.next_deadline().is_some());
    assert!(h.tooltip_presentation(&Metrics).unwrap().is_none());
    h.advance_overlays_at(now + Duration::from_millis(101));
    let tip = h.tooltip_presentation(&Metrics).unwrap().unwrap();
    assert!(tip.bounds.width <= 140.);
    assert!(tip.bounds.height > 30.);
    assert!(h.next_deadline().is_none());
    key(&h, Key::Escape);
    assert!(h.tooltip().is_none());
    assert_eq!(*log.borrow(), [DismissReason::Escape]);
    assert!(h.focused().is_some());
    h.blur();
    h.focus(keyed(&h, "help").id).unwrap();
    h.advance_overlays_at(Instant::now() + Duration::from_secs(1));
    assert!(h.tooltip().is_some());
    visible.set(false);
    layout(&h);
    assert!(h.tooltip().is_none());
    assert_eq!(log.borrow().last(), Some(&DismissReason::OwnerRemoved));
    assert!(h.next_deadline().is_none());
}
#[test]
fn fully_clipped_owner_dismisses_without_scrolling_back_and_invalid_options_preserve_old_tree() {
    let popup = Popup::new();
    let h = Host::new({
        let popup = popup.clone();
        move || {
            View::layer(vec![
                View::scroll(vec![
                    View::button("Owner", || {}).key("owner").shrink(0.),
                    View::spacer().height(600).shrink(0.),
                    View::button("Tail", || {}).shrink(0.),
                ])
                .height(80)
                .key("scroll"),
                popup.popover("owner", View::button("Body", || {})),
            ])
        }
    });
    layout(&h);
    popup.show();
    layout(&h);
    h.scroll_to(keyed(&h, "scroll").id, Point { x: 0., y: 500. })
        .unwrap();
    layout(&h);
    assert!(!popup.open.get());
    assert!(
        h.scroll_info(keyed(&h, "scroll").id)
            .unwrap()
            .unwrap()
            .offset
            .y
            >= 500.
    );
    assert_eq!(keyed(&h, "owner").clip.height, 0.);
    let options = State::new(OverlayOptions::default());
    let popup = Popup::new();
    popup.show();
    let h = Host::new({
        let options = options.clone();
        move || {
            View::layer(vec![
                View::button("Owner", || {}).key("owner"),
                popup
                    .clone()
                    .options(options.get())
                    .popover("owner", View::label("Body")),
            ])
        }
    });
    layout(&h);
    options.set(OverlayOptions {
        width: Some(f64::NAN),
        ..Default::default()
    });
    assert!(h.layout(Size::new(640., 480.), &Metrics).is_err());
    options.set(OverlayOptions::default());
    layout(&h);
    assert!(h.active_modal().is_some());
}
#[test]
fn tooltip_hover_click_suppression_reentry_and_noninteractive_owner_have_no_idle_wakes() {
    let clicks = State::new(0);
    let h = Host::new({
        let clicks = clicks.clone();
        move || {
            View::column(vec![
                View::button("Help", clicks.action(|s| s.update(|n| *n += 1)))
                    .key("help")
                    .tooltip("Click me")
                    .tooltip_options(TooltipOptions {
                        delay: Duration::ZERO,
                        ..Default::default()
                    }),
                View::label("Label help")
                    .key("label")
                    .tooltip("Also on labels")
                    .tooltip_options(TooltipOptions {
                        delay: Duration::ZERO,
                        ..Default::default()
                    }),
            ])
        }
    });
    layout(&h);
    let p = center(&keyed(&h, "help"));
    pointer(&h, PointerKind::Move, p, 0, false);
    assert!(h.tooltip().is_some());
    assert!(h.next_deadline().is_none());
    pointer(&h, PointerKind::Down, p, 0, false);
    pointer(&h, PointerKind::Up, p, 0, false);
    assert_eq!(clicks.get(), 1);
    assert!(h.tooltip().is_none());
    assert!(h.next_deadline().is_none());
    pointer(&h, PointerKind::Move, center(&keyed(&h, "label")), 0, false);
    assert_eq!(h.tooltip().unwrap().0, "Also on labels");
    pointer(&h, PointerKind::Move, p, 0, false);
    assert_eq!(h.tooltip().unwrap().0, "Click me");
}
#[test]
fn close_then_show_before_rebuild_does_not_dismiss_the_new_presentation() {
    let log = Rc::new(RefCell::new(vec![]));
    let popup = log_popup(&log);
    let h = Host::new({
        let popup = popup.clone();
        move || {
            View::layer(vec![
                popup.button("Open", "open"),
                popup.popover("open", View::button("Body", || {})),
            ])
        }
    });
    popup.show();
    layout(&h);
    let old = h.active_modal();
    popup.dismiss();
    popup.show();
    layout(&h);
    assert!(popup.open.get());
    assert_ne!(h.active_modal(), old);
    assert_eq!(*log.borrow(), [DismissReason::Programmatic]);
}

#[test]
fn escape_can_dismiss_a_refreshed_overlay_before_its_first_layout() {
    let log = Rc::new(RefCell::new(vec![]));
    let popup = log_popup(&log);
    let h = Host::new({
        let popup = popup.clone();
        move || {
            View::layer(vec![
                popup.button("Open", "open"),
                popup.popover("open", View::button("Body", || {})),
            ])
        }
    });
    layout(&h);
    popup.show();
    h.snapshot().unwrap();
    h.key(KeyEvent {
        key: Key::Escape,
        down: true,
        repeat: false,
        modifiers: Default::default(),
    })
    .unwrap();
    layout(&h);
    assert!(!popup.open.get());
    assert_eq!(*log.borrow(), [DismissReason::Escape]);
    assert!(h.active_modal().is_none());
}

#[test]
fn attached_context_menu_keeps_its_owner_when_a_later_modifier_changes_the_key() {
    let menu = Menu::new();
    let h = Host::new({
        let menu = menu.clone();
        move || {
            View::label("Target")
                .context_menu(&menu, vec![MenuAction::new("Run", || {})])
                .key("later")
        }
    });
    layout(&h);
    assert!(h.show_context_menu(keyed(&h, "later").id, None).unwrap());
    layout(&h);
    assert!(menu.open.get());
    let scope = h
        .snapshot()
        .unwrap()
        .into_iter()
        .find(|n| n.menu_root)
        .unwrap();
    assert_eq!(scope.overlay_owner, Some(keyed(&h, "later").id));
}
