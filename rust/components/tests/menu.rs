use scars_ui::*;
use std::{
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
        Ok(Size::default())
    }
}
fn layout(host: &Host) {
    host.layout(Size::new(640., 480.), &Metrics).unwrap();
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
    layout(host);
}
fn item(host: &Host, label: &str) -> Node {
    host.snapshot()
        .unwrap()
        .into_iter()
        .find(|n| n.menu_item.is_some() && n.text == label)
        .unwrap()
}
fn focused(host: &Host) -> String {
    host.snapshot()
        .unwrap()
        .into_iter()
        .find(|n| Some(n.id) == host.focused())
        .unwrap()
        .text
}
fn tree(menu: Menu, items: impl Fn() -> Vec<MenuAction> + 'static) -> Host {
    Host::new(move || {
        View::layer(vec![
            View::button("Open", || {}).key("open"),
            menu.view("open", items()),
        ])
    })
}
fn point(n: &Node) -> Point {
    Point {
        x: n.bounds.x + 5.,
        y: n.bounds.y + n.bounds.height / 2.,
    }
}
fn pointer(host: &Host, kind: PointerKind, position: Point) {
    host.pointer(Pointer {
        kind,
        position,
        button: 0,
        modifiers: Default::default(),
    })
    .unwrap();
    layout(host);
}
#[test]
fn arrows_skip_disabled_and_separators_activation_toggles_bindings_and_restores_anchor() {
    let menu = Menu::new();
    let checked = State::new(false);
    let radio = State::new("a".to_string());
    let host = tree(menu.clone(), {
        let checked = checked.clone();
        let radio = radio.clone();
        move || {
            vec![
                MenuAction::new("Disabled", || {}).enabled(false),
                MenuAction::separator(),
                MenuAction::check("Check", checked.binding()).shortcut("⌘K"),
                MenuAction::radio("B", radio.binding(), "b"),
            ]
        }
    });
    layout(&host);
    key(&host, Key::Tab);
    let anchor = host.focused();
    menu.show();
    layout(&host);
    assert_eq!(focused(&host), "Check");
    assert_eq!(item(&host, "Check").menu_item.unwrap().checked, Some(false));
    key(&host, Key::Up);
    assert_eq!(focused(&host), "B");
    key(&host, Key::Enter);
    assert_eq!(radio.get(), "b");
    assert!(!menu.open.get());
    assert_eq!(host.focused(), anchor);
    menu.show();
    layout(&host);
    key(&host, Key::Space);
    assert!(checked.get());
    assert!(!menu.open.get());
}
#[test]
fn submenus_open_close_navigate_at_their_own_level_and_escape_one_level_at_a_time() {
    let menu = Menu::new();
    let count = State::new(0);
    let host = tree(menu.clone(), {
        let count = count.clone();
        move || {
            vec![
                MenuAction::submenu(
                    "Export",
                    vec![
                        MenuAction::new("Disabled", || {}).enabled(false),
                        MenuAction::submenu(
                            "Images",
                            vec![MenuAction::new(
                                "PNG",
                                count.action(|n| n.update(|x| *x += 1)),
                            )],
                        ),
                    ],
                ),
                MenuAction::new("Other", || {}),
            ]
        }
    });
    layout(&host);
    menu.show();
    layout(&host);
    key(&host, Key::Right);
    assert_eq!(focused(&host), "Images");
    key(&host, Key::Right);
    assert_eq!(focused(&host), "PNG");
    key(&host, Key::Escape);
    assert_eq!(focused(&host), "Images");
    assert!(menu.open.get());
    assert!(!host.snapshot().unwrap().iter().any(|n| n.text == "PNG"));
    key(&host, Key::Left);
    assert_eq!(focused(&host), "Export");
    key(&host, Key::Down);
    assert_eq!(focused(&host), "Other");
    key(&host, Key::Home);
    key(&host, Key::Enter);
    key(&host, Key::Enter);
    key(&host, Key::Enter);
    assert_eq!(count.get(), 1);
    assert!(!menu.open.get());
}
#[test]
fn typeahead_matches_prefix_cycles_letters_and_expires_without_a_timer() {
    let menu = Menu::new();
    let host = tree(menu.clone(), || {
        vec![
            MenuAction::new("Apple", || {}),
            MenuAction::new("Apricot", || {}),
            MenuAction::new("Banana", || {}),
            MenuAction::new("Blueberry", || {}).enabled(false),
        ]
    });
    menu.show();
    layout(&host);
    let now = Instant::now();
    host.menu_text_at("a", now).unwrap();
    assert_eq!(focused(&host), "Apple");
    host.menu_text_at("a", now + Duration::from_millis(100))
        .unwrap();
    assert_eq!(focused(&host), "Apricot");
    host.menu_text_at("p", now + Duration::from_millis(200))
        .unwrap();
    assert_eq!(focused(&host), "Apricot");
    host.menu_text_at("B", now + Duration::from_secs(2))
        .unwrap();
    assert_eq!(focused(&host), "Banana");
    assert!(host.next_deadline().is_none());
    key(&host, Key::Tab);
    assert!(!menu.open.get());
    assert!(!host.menu_text("a").unwrap());
}
#[test]
fn pointer_hover_opens_submenus_click_activates_and_outside_press_dismisses() {
    let menu = Menu::new();
    let count = State::new(0);
    let host = tree(menu.clone(), {
        let count = count.clone();
        move || {
            vec![MenuAction::submenu(
                "More",
                vec![MenuAction::new(
                    "Run",
                    count.action(|n| n.update(|x| *x += 1)),
                )],
            )]
        }
    });
    menu.show();
    layout(&host);
    pointer(&host, PointerKind::Move, point(&item(&host, "More")));
    let leaf = item(&host, "Run");
    pointer(&host, PointerKind::Down, point(&leaf));
    pointer(&host, PointerKind::Up, point(&leaf));
    assert_eq!(count.get(), 1);
    assert!(!menu.open.get());
    menu.show();
    layout(&host);
    pointer(&host, PointerKind::Down, Point { x: 600., y: 450. });
    assert!(!menu.open.get());
    assert_eq!(count.get(), 1);
}
#[test]
fn branch_changes_repair_focus_and_deactivation_and_lost_anchor_dismiss() {
    let menu = Menu::new();
    let enabled = State::new(true);
    let visible = State::new(true);
    let host = Host::new({
        let menu = menu.clone();
        let enabled = enabled.clone();
        let visible = visible.clone();
        move || {
            View::layer(vec![
                View::button("Open", || {})
                    .key("open")
                    .visible(visible.get()),
                menu.view(
                    "open",
                    vec![
                        MenuAction::submenu("More", vec![MenuAction::new("Leaf", || {})])
                            .enabled(enabled.get()),
                        MenuAction::new("Other", || {}),
                    ],
                ),
            ])
        }
    });
    menu.show();
    layout(&host);
    key(&host, Key::Right);
    let stale = item(&host, "Leaf").id;
    enabled.set(false);
    layout(&host);
    assert_eq!(focused(&host), "Other");
    assert!(!host.activate(stale).unwrap());
    host.set_active(false);
    layout(&host);
    assert!(!menu.open.get());
    host.set_active(true);
    menu.show();
    visible.set(false);
    layout(&host);
    assert!(!menu.open.get());
}
#[test]
fn split_action_remembers_leaf_by_key_and_uses_safe_fallback_for_disabled_removed_choice() {
    let split = SplitAction::new();
    let chosen = State::new(String::new());
    let second = State::new(true);
    let host = Host::new({
        let split = split.clone();
        let chosen = chosen.clone();
        let second = second.clone();
        move || {
            let mut items = vec![
                MenuAction::new("First", chosen.action(|s| s.set("first".into()))).key("first"),
            ];
            if second.get() {
                items.push(MenuAction::submenu(
                    "More",
                    vec![
                        MenuAction::new("Second", chosen.action(|s| s.set("second".into())))
                            .key("second"),
                    ],
                ));
            }
            let (button, overlay) = split.view("split", items);
            View::layer(vec![button, overlay])
        }
    });
    layout(&host);
    split.menu.show();
    layout(&host);
    key(&host, Key::End);
    key(&host, Key::Right);
    key(&host, Key::Enter);
    assert_eq!(chosen.get(), "second");
    assert_eq!(split.selected(), Some(vec!["More".into(), "second".into()]));
    let primary = |host: &Host| {
        host.snapshot()
            .unwrap()
            .into_iter()
            .find(|n| n.key.as_deref() == Some("$primary"))
            .unwrap()
    };
    assert_eq!(primary(&host).text, "Second");
    chosen.set(String::new());
    host.activate(primary(&host).id).unwrap();
    assert_eq!(chosen.get(), "second");
    second.set(false);
    layout(&host);
    assert_eq!(primary(&host).text, "First");
    assert_eq!(split.selected().unwrap()[1], "second");
    second.set(true);
    layout(&host);
    assert_eq!(primary(&host).text, "Second");
}
#[test]
fn menu_keys_validate_before_replacing_the_tree() {
    let menu = Menu::new();
    menu.show();
    let host = tree(menu, || {
        vec![
            MenuAction::new("Same", || {}),
            MenuAction::new("Same", || {}),
        ]
    });
    assert!(host.refresh().unwrap_err().0.contains("unique"));
}
#[test]
fn empty_submenu_closes_back_to_its_owner_and_edges_fit_small_viewports() {
    let menu = Menu::new();
    let host = Host::new({
        let menu = menu.clone();
        move || {
            View::layer(vec![
                View::button("Anchor", || {})
                    .key("open")
                    .position(Position::Absolute)
                    .left(310)
                    .top(160)
                    .width(40)
                    .height(30),
                menu.view(
                    "open",
                    vec![
                        MenuAction::submenu("First", vec![]),
                        MenuAction::submenu("Empty", vec![]),
                    ],
                ),
            ])
        }
    });
    menu.show();
    host.layout(Size::new(360., 240.), &Metrics).unwrap();
    host.key(KeyEvent {
        key: Key::End,
        down: true,
        repeat: false,
        modifiers: Default::default(),
    })
    .unwrap();
    host.key(KeyEvent {
        key: Key::Right,
        down: true,
        repeat: false,
        modifiers: Default::default(),
    })
    .unwrap();
    host.layout(Size::new(360., 240.), &Metrics).unwrap();
    assert!(item(&host, "Empty").menu_item.unwrap().expanded);
    let panels: Vec<_> = host
        .snapshot()
        .unwrap()
        .into_iter()
        .filter(|n| n.menu_panel)
        .collect();
    assert_eq!(panels.len(), 2);
    for p in panels {
        assert!(
            p.bounds.x >= 0.
                && p.bounds.y >= 0.
                && p.bounds.x + p.bounds.width <= 360.
                && p.bounds.y + p.bounds.height <= 240.
        );
    }
    host.key(KeyEvent {
        key: Key::Escape,
        down: true,
        repeat: false,
        modifiers: Default::default(),
    })
    .unwrap();
    host.layout(Size::new(360., 240.), &Metrics).unwrap();
    assert!(menu.open.get());
    assert_eq!(focused(&host), "Empty");
    assert!(!item(&host, "Empty").menu_item.unwrap().expanded);
}
#[test]
fn keyboard_reveals_long_menu_items_and_callbacks_can_remove_the_menu() {
    let menu = Menu::new();
    let visible = State::new(true);
    let host = Host::new({
        let menu = menu.clone();
        let visible = visible.clone();
        move || {
            View::layer(vec![
                View::button("Open", || {}).key("open"),
                if visible.get() {
                    menu.view(
                        "open",
                        (0..50)
                            .map(|i| {
                                MenuAction::new(
                                    format!("Item {i}"),
                                    visible.action(|s| s.set(false)),
                                )
                            })
                            .collect(),
                    )
                } else {
                    View::label("Done")
                },
            ])
        }
    });
    menu.show();
    layout(&host);
    key(&host, Key::End);
    assert_eq!(focused(&host), "Item 49");
    let last = item(&host, "Item 49");
    assert!(last.clip.height > 0.);
    assert!(last.clip.contains(point(&last)));
    key(&host, Key::Enter);
    assert!(!visible.get());
    assert!(!menu.open.get());
}
#[test]
fn dragging_between_menu_rows_activates_only_the_release_target() {
    let menu = Menu::new();
    let count = State::new(0);
    let host = tree(menu.clone(), {
        let count = count.clone();
        move || {
            vec![
                MenuAction::new("First", count.action(|s| s.set(1))),
                MenuAction::new("Second", count.action(|s| s.set(2))),
            ]
        }
    });
    menu.show();
    layout(&host);
    pointer(&host, PointerKind::Down, point(&item(&host, "First")));
    pointer(&host, PointerKind::Move, point(&item(&host, "Second")));
    pointer(&host, PointerKind::Up, point(&item(&host, "Second")));
    assert_eq!(count.get(), 2);
    assert!(!menu.open.get());
    menu.show();
    layout(&host);
    pointer(&host, PointerKind::Down, point(&item(&host, "First")));
    pointer(&host, PointerKind::Move, Point { x: 600., y: 450. });
    pointer(&host, PointerKind::Up, Point { x: 600., y: 450. });
    assert_eq!(count.get(), 2);
    assert!(!menu.open.get());
}
