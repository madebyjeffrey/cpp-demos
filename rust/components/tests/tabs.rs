use scars_ui::*;
use std::{cell::Cell, rc::Rc};
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
fn layout(h: &Host) {
    h.layout(Size::new(500., 320.), &Metrics).unwrap();
}
fn keyed(h: &Host, key: &str) -> Node {
    h.snapshot()
        .unwrap()
        .into_iter()
        .find(|n| n.key.as_deref() == Some(key))
        .unwrap()
}
fn header(h: &Host, id: &str) -> Node {
    h.snapshot()
        .unwrap()
        .into_iter()
        .find(|n| n.kind == Kind::Tab && n.tab_id.as_deref() == Some(id))
        .unwrap()
}
fn root(h: &Host) -> Node {
    h.snapshot()
        .unwrap()
        .into_iter()
        .find(|n| n.kind == Kind::Tabs)
        .unwrap()
}
fn key(h: &Host, key: Key, control: bool, shift: bool) -> bool {
    let result = h
        .key(KeyEvent {
            key,
            down: true,
            repeat: false,
            modifiers: Modifiers {
                control,
                shift,
                ..Default::default()
            },
        })
        .unwrap();
    layout(h);
    result
}
#[test]
fn page_switch_retains_editor_history_selection_scroll_and_remembered_focus() {
    let tabs = Tabs::new("a");
    let value = State::new("start".to_owned());
    let h = Host::new({
        let tabs = tabs.clone();
        let value = value.clone();
        move || {
            tabs.view(vec![
        Tab::new("a","Alpha",view!{column{ text_field("Name",bind:value,key:"field");scroll(height:80,key:"scroll"){label("Long",height:600);}}}),
        Tab::new("b","Beta",View::button("Second control",||{}).key("second")),
    ])
        }
    });
    layout(&h);
    let field = keyed(&h, "field").id;
    let scroll = keyed(&h, "scroll").id;
    h.focus(field).unwrap();
    h.set_text(field, "edited").unwrap();
    h.select_text(
        field,
        Selection {
            anchor: 1,
            caret: 4,
        },
    )
    .unwrap();
    layout(&h);
    h.scroll_to(scroll, Point { x: 0., y: 50. }).unwrap();
    layout(&h);
    tabs.select("b");
    layout(&h);
    assert!(keyed(&h, "field").hidden);
    assert_eq!(keyed(&h, "field").bounds.height, 0.);
    assert!(!h.focus(field).unwrap());
    assert!(!h.set_text(field, "hidden edit").unwrap());
    h.focus(keyed(&h, "second").id).unwrap();
    tabs.select("a");
    layout(&h);
    assert_eq!(keyed(&h, "field").id, field);
    assert_eq!(h.focused(), Some(field));
    assert_eq!(
        h.text_input().unwrap().unwrap().selection,
        Selection {
            anchor: 1,
            caret: 4
        }
    );
    assert_eq!(h.scroll_info(scroll).unwrap().unwrap().offset.y, 50.);
    assert!(h.text_command(TextCommand::Undo, false).unwrap());
    assert_eq!(value.get(), "start");
    h.select_tab(root(&h).id, "b").unwrap();
    layout(&h);
    assert_eq!(h.focused(), Some(header(&h, "b").id));
    assert!(key(&h, Key::Tab, false, false));
    assert_eq!(h.focused(), Some(keyed(&h, "second").id));
    h.set_active(false);
    tabs.select("a");
    layout(&h);
    assert!(h.focused().is_none());
    h.set_active(true);
    layout(&h);
    assert_eq!(h.focused(), Some(field));
}
#[test]
fn lazy_pages_only_build_after_visit_and_removal_discards_their_retained_state() {
    let tabs = Tabs::new("a");
    let include = State::new(true);
    let builds = Rc::new(Cell::new(0));
    let value = State::new("start".to_owned());
    let h = Host::new({
        let tabs = tabs.clone();
        let include = include.clone();
        let builds = builds.clone();
        let value = value.clone();
        move || {
            let mut pages = vec![Tab::new("a", "Alpha", View::label("First"))];
            if include.get() {
                pages.push(Tab::lazy("b", "Beta", {
                    let builds = builds.clone();
                    let value = value.clone();
                    move || {
                        builds.set(builds.get() + 1);
                        View::text_field("Draft", value.binding()).key("draft")
                    }
                }));
            }
            tabs.view(pages)
        }
    });
    layout(&h);
    assert_eq!(builds.get(), 0);
    assert_eq!(root(&h).tabs.unwrap().visited, 1);
    tabs.select("b");
    layout(&h);
    let old = keyed(&h, "draft").id;
    h.focus(old).unwrap();
    h.set_text(old, "edited").unwrap();
    tabs.select("a");
    layout(&h);
    assert!(builds.get() > 0);
    assert_eq!(root(&h).tabs.unwrap().visited, 2);
    value.set("external".into());
    layout(&h);
    tabs.select("b");
    layout(&h);
    h.focus(old).unwrap();
    assert_eq!(h.text_input().unwrap().unwrap().text, "external");
    include.set(false);
    layout(&h);
    assert!(!h.activate(old).unwrap());
    include.set(true);
    tabs.select("b");
    layout(&h);
    let new = keyed(&h, "draft").id;
    assert_ne!(old, new);
    h.focus(new).unwrap();
    assert!(!h.text_input().unwrap().unwrap().can_undo);
}
#[test]
fn reordering_preserves_page_ids_and_empty_disabled_invalid_choices_are_safe() {
    let tabs = Tabs::new("b");
    let reverse = State::new(false);
    let enabled = State::new(true);
    let empty = State::new(false);
    let h = Host::new({
        let tabs = tabs.clone();
        let reverse = reverse.clone();
        let enabled = enabled.clone();
        let empty = empty.clone();
        move || {
            let mut pages = if empty.get() {
                vec![]
            } else {
                vec![
                    Tab::new("a", "Alpha", View::button("A", || {}).key("a-field")),
                    Tab::new("b", "Beta", View::button("B", || {}).key("b-field"))
                        .enabled(enabled.get()),
                ]
            };
            if reverse.get() {
                pages.reverse();
            }
            tabs.view(pages)
        }
    });
    layout(&h);
    let field = keyed(&h, "b-field").id;
    reverse.set(true);
    layout(&h);
    assert_eq!(keyed(&h, "b-field").id, field);
    h.focus(field).unwrap();
    enabled.set(false);
    layout(&h);
    let info = root(&h).tabs.unwrap();
    assert_eq!(info.requested, "b");
    assert_eq!(info.selected, Some("a".into()));
    assert_eq!(tabs.selected(), "b");
    assert_eq!(h.focused(), Some(header(&h, "a").id));
    assert!(!h.activate(header(&h, "b").id).unwrap());
    enabled.set(true);
    layout(&h);
    assert_eq!(root(&h).tabs.unwrap().selected, Some("b".into()));
    assert_eq!(keyed(&h, "b-field").id, field);
    empty.set(true);
    layout(&h);
    assert!(root(&h).tabs.unwrap().selected.is_none());
    assert!(h.focused().is_none());
}
#[test]
fn keyboard_roves_one_header_stop_skips_disabled_and_control_tab_targets_nearest_group() {
    let outer = Tabs::new("a");
    let inner = Tabs::new("one");
    let h = Host::new({
        let outer = outer.clone();
        let inner = inner.clone();
        move || {
            outer.view(vec![
                Tab::new(
                    "a",
                    "Alpha",
                    inner.view(vec![
                        Tab::new(
                            "one",
                            "One",
                            View::button("Inner control", || {}).key("inside"),
                        ),
                        Tab::new("two", "Two", View::button("Other inner", || {})),
                    ]),
                ),
                Tab::new("disabled", "Unavailable", View::label("Disabled")).enabled(false),
                Tab::new("b", "Beta", View::button("Outer second", || {})),
            ])
        }
    });
    layout(&h);
    h.focus(keyed(&h, "inside").id).unwrap();
    key(&h, Key::Tab, true, false);
    assert_eq!(inner.selected(), "two");
    assert_eq!(outer.selected(), "a");
    h.focus(header(&h, "a").id).unwrap();
    key(&h, Key::Right, false, false);
    assert_eq!(outer.selected(), "b");
    assert_eq!(h.focused(), Some(header(&h, "b").id));
    key(&h, Key::Right, false, false);
    assert_eq!(outer.selected(), "a");
    key(&h, Key::End, false, false);
    assert_eq!(outer.selected(), "b");
    key(&h, Key::Home, false, false);
    assert_eq!(outer.selected(), "a");
    key(&h, Key::Tab, true, true);
    assert_eq!(outer.selected(), "b");
    h.focus(header(&h, "a").id).unwrap();
    key(&h, Key::Right, false, false);
    assert_eq!(outer.selected(), "b");
    let outside = Host::new(|| View::button("Outside", || {}));
    layout(&outside);
    outside.focus(outside.snapshot().unwrap()[0].id).unwrap();
    assert!(!key(&outside, Key::Tab, true, false));
}
#[test]
fn external_switch_cancels_preedit_explicit_selection_commits_and_rechecks_target() {
    let tabs = Tabs::new("a");
    let text = State::new("value".to_owned());
    let available = State::new(true);
    let binding = Binding::new(
        {
            let text = text.clone();
            move || text.get()
        },
        {
            let text = text.clone();
            let available = available.clone();
            move |s| {
                text.set(s);
                available.set(false);
            }
        },
    );
    let h = Host::new({
        let tabs = tabs.clone();
        let binding = binding.clone();
        let available = available.clone();
        move || {
            tabs.view(vec![
                Tab::new(
                    "a",
                    "Alpha",
                    View::text_field("Draft", binding.clone()).key("draft"),
                ),
                Tab::new("b", "Beta", View::label("B")).enabled(available.get()),
            ])
        }
    });
    layout(&h);
    let id = keyed(&h, "draft").id;
    h.focus(id).unwrap();
    h.mark_text("仮", 0.."仮".len(), None).unwrap();
    tabs.select("b");
    layout(&h);
    tabs.select("a");
    layout(&h);
    h.focus(id).unwrap();
    assert_eq!(text.get(), "value");
    assert!(h.text_input().unwrap().unwrap().marked.is_none());
    h.select_text(
        id,
        Selection {
            anchor: 5,
            caret: 5,
        },
    )
    .unwrap();
    h.mark_text("!", 0..1, None).unwrap();
    assert!(h.activate(header(&h, "b").id).unwrap());
    layout(&h);
    assert!(!available.get());
    assert_eq!(tabs.selected(), "a");
    assert_eq!(text.get(), "value!");
}
#[test]
fn hidden_page_cancels_search_work_and_closes_its_popover_without_forgetting_editor_state() {
    let tabs = Tabs::new("a");
    let popup = Popup::new();
    let query = State::new(String::new());
    let calls = State::new(0);
    let h = Host::new({
        let tabs = tabs.clone();
        let popup = popup.clone();
        let query = query.clone();
        let calls = calls.clone();
        move || {
            tabs.view(vec![
                Tab::new(
                    "a",
                    "Alpha",
                    View::layer(vec![
                        View::search_field("Search", query.binding())
                            .key("query")
                            .on_query(calls.handler(|s, _: String| s.update(|n| *n += 1))),
                        popup.popover("query", View::button("Inside", || {})),
                    ]),
                ),
                Tab::new("b", "Beta", View::button("B", || {})),
            ])
        }
    });
    layout(&h);
    let id = keyed(&h, "query").id;
    h.focus(id).unwrap();
    h.insert_text("query", None).unwrap();
    assert!(h.next_deadline().is_some());
    tabs.select("b");
    layout(&h);
    assert!(h.next_deadline().is_none());
    assert_eq!(calls.get(), 0);
    tabs.select("a");
    layout(&h);
    popup.show();
    layout(&h);
    assert!(popup.open.get());
    tabs.select("b");
    layout(&h);
    assert!(!popup.open.get());
    assert_eq!(keyed(&h, "query").id, id);
}
#[test]
fn rejected_selection_and_failed_page_builds_keep_the_previous_page_usable() {
    let selected = State::new("a".to_owned());
    let fail = State::new(false);
    let tabs = Tabs::bound(Binding::new(
        {
            let selected = selected.clone();
            move || selected.get()
        },
        |_| {},
    ));
    let h = Host::new({
        let tabs = tabs.clone();
        move || {
            tabs.view(vec![
                Tab::new("a", "Alpha", View::button("A", || {})),
                Tab::new("b", "Beta", View::label("B")),
            ])
        }
    });
    layout(&h);
    h.activate(header(&h, "b").id).unwrap();
    layout(&h);
    assert_eq!(root(&h).tabs.unwrap().selected, Some("a".into()));
    assert_eq!(h.focused(), Some(header(&h, "a").id));
    let tabs = Tabs::new("a");
    let h = Host::new({
        let tabs = tabs.clone();
        let fail = fail.clone();
        move || {
            tabs.view(vec![
                Tab::new("a", "Alpha", View::button("A", || {}).key("first")),
                Tab::lazy("b", "Beta", {
                    let fail = fail.clone();
                    move || {
                        if fail.get() {
                            View::column(vec![
                                View::label("Duplicate").key("x"),
                                View::label("Duplicate").key("x"),
                            ])
                        } else {
                            View::label("Good")
                        }
                    }
                }),
            ])
        }
    });
    layout(&h);
    let before = keyed(&h, "first").id;
    fail.set(true);
    tabs.select("b");
    assert!(h.layout(Size::new(500., 320.), &Metrics).is_err());
    fail.set(false);
    tabs.select("a");
    layout(&h);
    assert_eq!(keyed(&h, "first").id, before);
    let h = Host::new(|| {
        Tabs::new("a").view(vec![
            Tab::new("same", "A", View::label("A")),
            Tab::new("same", "B", View::label("B")),
        ])
    });
    assert!(h.refresh().unwrap_err().0.contains("unique"));
}
#[test]
fn overflowing_headers_reveal_focus_across_resize_and_keep_the_page_out_of_strip_scrolling() {
    let tabs = Tabs::new("0");
    let reverse = State::new(false);
    let h = Host::new({
        let tabs = tabs.clone();
        let reverse = reverse.clone();
        move || {
            let mut pages: Vec<_> = (0..10)
                .map(|i| {
                    Tab::new(
                        i.to_string(),
                        format!("Long tab {i}"),
                        View::button("Content", || {}),
                    )
                })
                .collect();
            if reverse.get() {
                pages.reverse();
            }
            tabs.view(pages)
        }
    });
    h.layout(Size::new(180., 160.), &Metrics).unwrap();
    h.focus(header(&h, "9").id).unwrap();
    h.layout(Size::new(180., 160.), &Metrics).unwrap();
    let last = header(&h, "9");
    assert!(last.clip.width > 0.);
    assert!(last.bounds.x >= 0. && last.bounds.x + last.bounds.width <= 180.001);
    reverse.set(true);
    h.layout(Size::new(180., 160.), &Metrics).unwrap();
    assert!(header(&h, "9").bounds.x >= 0.);
    h.layout(Size::new(90., 160.), &Metrics).unwrap();
    let last = header(&h, "9");
    assert!(last.clip.width > 0.);
    assert!(last.bounds.x >= 0. && last.bounds.x + last.bounds.width <= 90.001);
    h.key(KeyEvent {
        key: Key::Home,
        down: true,
        repeat: false,
        modifiers: Default::default(),
    })
    .unwrap();
    h.layout(Size::new(90., 160.), &Metrics).unwrap();
    let strip = keyed(&h, "$strip").scroll.unwrap();
    assert_eq!(strip.offset.x, 0.);
    assert!(
        h.scroll_at(Point { x: 20., y: 20. }, Point { x: 100., y: 0. })
            .unwrap()
    );
    h.layout(Size::new(90., 160.), &Metrics).unwrap();
    assert!(keyed(&h, "$strip").scroll.unwrap().offset.x > 0.);
    tabs.select("4");
    h.layout(Size::new(90., 160.), &Metrics).unwrap();
    assert!(header(&h, "4").clip.width > 0.);
    h.layout(Size::new(0., 0.), &Metrics).unwrap();
    let split_tabs = Tabs::new("a");
    let h = Host::new(move || {
        View::row(vec![
            split_tabs
                .view(vec![Tab::new(
                    "a",
                    "A very long tab title that needs ellipsis",
                    View::label("Body"),
                )])
                .basis(0)
                .grow(1.),
            View::label("").width(300),
        ])
    });
    h.layout(Size::new(500., 160.), &Metrics).unwrap();
    let group = root(&h);
    let tab = header(&h, "a");
    assert_eq!(group.bounds.width, 192.);
    assert!(tab.bounds.width <= group.bounds.width);
}
#[test]
fn legacy_helper_and_macro_keep_identity_and_context_menu_can_attach_to_the_group() {
    let selected = State::new("a".to_owned());
    let value = State::new(String::new());
    let h = Host::new({
        let selected = selected.clone();
        move || {
            tabs(
                &selected,
                vec![
                    (
                        "a".into(),
                        "Alpha".into(),
                        View::text_field("Draft", value.binding()).key("draft"),
                    ),
                    ("b".into(), "Beta".into(), View::label("B")),
                ],
            )
        }
    });
    layout(&h);
    let id = keyed(&h, "draft").id;
    selected.set("b".into());
    layout(&h);
    selected.set("a".into());
    layout(&h);
    assert_eq!(keyed(&h, "draft").id, id);
    let tabs = Tabs::new("a");
    let menu = Menu::new();
    let h = Host::new({
        let menu = menu.clone();
        move || view! {embed(tab_view(&tabs,vec![Tab::new("a","Alpha",View::label("A"))]).context_menu(&menu,vec![MenuAction::new("Run",||{})]),key:"group");}
    });
    layout(&h);
    assert!(h.show_context_menu(keyed(&h, "group").id, None).unwrap());
    layout(&h);
    assert!(menu.open.get());
}
#[test]
fn a_hidden_virtual_list_keeps_its_visible_range_and_row_editor_history() {
    let tabs = Tabs::new("a");
    let list = ListModel::new(
        (0..100)
            .map(|i| ListItem::new(i.to_string(), format!("Row {i}")))
            .collect(),
        40.,
    );
    let value = State::new("draft".to_owned());
    let h = Host::new({
        let tabs = tabs.clone();
        let list = list.clone();
        let value = value.clone();
        move || {
            tabs.view(vec![
                Tab::new(
                    "a",
                    "Alpha",
                    list.view_with(|item| {
                        View::text_field(item.label(), value.binding())
                            .key(format!("edit-{}", item.key))
                    })
                    .height(200),
                ),
                Tab::new("b", "Beta", View::label("Other")),
            ])
        }
    });
    layout(&h);
    let id = keyed(&h, "edit-4").id;
    h.focus(id).unwrap();
    h.set_text(id, "edited").unwrap();
    layout(&h);
    let before = list.visible_range();
    tabs.select("b");
    layout(&h);
    assert_eq!(list.visible_range(), before);
    tabs.select("a");
    layout(&h);
    assert_eq!(keyed(&h, "edit-4").id, id);
    h.focus(id).unwrap();
    assert!(h.text_command(TextCommand::Undo, false).unwrap());
    assert_eq!(value.get(), "draft");
}
