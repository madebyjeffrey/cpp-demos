use scars_ui::*;
use std::{
    cell::Cell,
    rc::Rc,
    time::{Duration, Instant},
};
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
fn items(count: usize) -> Vec<ListItem> {
    (0..count)
        .map(|i| ListItem::new(format!("id-{i}"), format!("Item {i:06}")))
        .collect()
}
fn model() -> ListModel {
    let mut rows = items(10);
    rows[2].enabled = false;
    rows[5].enabled = false;
    ListModel::new(rows, 30.).multiple()
}
fn layout(h: &Host) {
    h.layout(Size::new(400., 240.), &Metrics).unwrap();
}
fn host(m: &ListModel) -> Host {
    Host::new({
        let m = m.clone();
        move || m.view().height(240)
    })
}
fn key(h: &Host, key: Key, modifiers: Modifiers) {
    assert!(
        h.key(KeyEvent {
            key,
            down: true,
            repeat: false,
            modifiers
        })
        .unwrap()
    );
    layout(h);
}
fn selected(m: &ListModel) -> Vec<String> {
    m.selected().into_iter().collect()
}
fn ids(values: &[usize]) -> Vec<String> {
    values.iter().map(|i| format!("id-{i}")).collect()
}
fn cursor(h: &Host) -> String {
    h.snapshot()
        .unwrap()
        .iter()
        .find(|n| Some(n.id) == h.focused())
        .unwrap()
        .key
        .clone()
        .unwrap()
}
#[test]
fn selection_distinguishes_anchor_ranges_toggle_addition_and_focus_only() {
    let m = model();
    assert!(m.select("id-1", false));
    assert!(m.select_with("id-6", ListSelection::Range));
    assert_eq!(selected(&m), ids(&[1, 3, 4, 6]));
    m.select_with("id-3", ListSelection::Range);
    assert_eq!(selected(&m), ids(&[1, 3]));
    assert_eq!(m.anchor().as_deref(), Some("id-1"));
    m.select_with("id-7", ListSelection::Toggle);
    m.select_with("id-4", ListSelection::AddRange);
    assert_eq!(selected(&m), ids(&[1, 3, 4, 6, 7]));
    m.select_with("id-0", ListSelection::Focus);
    assert_eq!(selected(&m), ids(&[1, 3, 4, 6, 7]));
    assert!(!m.select_with("id-2", ListSelection::Replace));
    assert!(!m.select("missing", false));
    assert!(m.select_all());
    assert_eq!(m.selected().len(), 8);
    m.clear_selection();
    assert!(m.selected().is_empty());
    let clone = m.clone();
    m.set_options(ListOptions::default()).unwrap();
    assert!(clone.select_with("id-9", ListSelection::Toggle));
    clone.select_with("id-1", ListSelection::Range);
    assert_eq!(selected(&m), ids(&[1]));
}
#[test]
fn keyboard_skips_disabled_rows_uses_viewport_pages_and_keeps_one_tab_stop() {
    let m = model();
    let h = Host::new({
        let m = m.clone();
        move || {
            View::column(vec![
                View::button("Before", || {}).key("before"),
                m.view().height(90),
                View::button("After", || {}).key("after"),
            ])
        }
    });
    layout(&h);
    key(&h, Key::Tab, Default::default());
    assert_eq!(cursor(&h), "before");
    key(&h, Key::Tab, Default::default());
    assert_eq!(cursor(&h), "id-0");
    key(&h, Key::Down, Default::default());
    assert_eq!(cursor(&h), "id-1");
    key(
        &h,
        Key::Down,
        Modifiers {
            shift: true,
            ..Default::default()
        },
    );
    assert_eq!(cursor(&h), "id-3");
    assert_eq!(selected(&m), ids(&[1, 3]));
    key(&h, Key::PageDown, Default::default());
    assert_eq!(cursor(&h), "id-6");
    key(
        &h,
        Key::Up,
        Modifiers {
            command: true,
            ..Default::default()
        },
    );
    assert_eq!(cursor(&h), "id-4");
    assert_eq!(selected(&m), ids(&[6]));
    key(&h, Key::End, Default::default());
    assert_eq!(cursor(&h), "id-9");
    assert!(m.offset() > 0.);
    key(&h, Key::Tab, Default::default());
    assert_eq!(cursor(&h), "after");
    key(
        &h,
        Key::Tab,
        Modifiers {
            shift: true,
            ..Default::default()
        },
    );
    assert_eq!(cursor(&h), "id-9");
    key(
        &h,
        Key::A,
        Modifiers {
            command: true,
            ..Default::default()
        },
    );
    assert_eq!(m.selected().len(), 8);
}
#[test]
fn pointer_modifiers_distinguish_shift_and_command_and_double_click_activates_once() {
    let calls = State::new(vec![]);
    let m = model().on_activate(calls.handler(|s, key| s.update(|v| v.push(key))));
    let h = host(&m);
    layout(&h);
    let click = |index, modifiers, clicks| {
        let row = h
            .snapshot()
            .unwrap()
            .iter()
            .find(|n| n.key.as_deref() == Some(&format!("id-{index}")))
            .unwrap()
            .clone();
        for kind in [PointerKind::Down, PointerKind::Up] {
            h.pointer_click(
                Pointer {
                    kind,
                    position: Point {
                        x: row.bounds.x + 2.,
                        y: row.bounds.y + 10.,
                    },
                    button: 0,
                    modifiers,
                },
                clicks,
            )
            .unwrap();
        }
        layout(&h);
    };
    click(1, Default::default(), 1);
    click(
        4,
        Modifiers {
            shift: true,
            ..Default::default()
        },
        1,
    );
    assert_eq!(selected(&m), ids(&[1, 3, 4]));
    click(
        6,
        Modifiers {
            command: true,
            ..Default::default()
        },
        1,
    );
    assert_eq!(selected(&m), ids(&[1, 3, 4, 6]));
    assert!(calls.get().is_empty());
    click(6, Default::default(), 2);
    assert_eq!(calls.get(), ids(&[6]));
    key(&h, Key::Enter, Default::default());
    assert_eq!(calls.get(), ids(&[6, 6]));
}
#[test]
fn typeahead_is_unicode_case_insensitive_cycles_and_expires_without_editing_children() {
    let m = ListModel::new(
        vec![
            ListItem::new("e1", "Éclair"),
            ListItem::new("off", "Éden"),
            ListItem::new("e2", "Étoile"),
            ListItem::new("b", "Beta"),
        ],
        30.,
    );
    let mut rows = (0..m.len()).map(|i| m.item(i).unwrap()).collect::<Vec<_>>();
    rows[1].enabled = false;
    m.set_items(rows).unwrap();
    let h = host(&m);
    layout(&h);
    key(&h, Key::Tab, Default::default());
    let now = Instant::now();
    assert!(h.collection_text_at("é", now).unwrap());
    layout(&h);
    assert_eq!(cursor(&h), "e2");
    h.collection_text_at("é", now + Duration::from_millis(20))
        .unwrap();
    layout(&h);
    assert_eq!(cursor(&h), "e1");
    h.collection_text_at("b", now + Duration::from_secs(2))
        .unwrap();
    layout(&h);
    assert_eq!(cursor(&h), "b");
    assert!(!h.collection_text("\n").unwrap());
    h.collection_text_at("zz", now + Duration::from_secs(4))
        .unwrap();
    assert_eq!(m.cursor().as_deref(), Some("b"));
    let text = State::new(String::new());
    let h = Host::new(move || m.view_with(|_| view! {text_field("Value",bind:text);}));
    layout(&h);
    let field = h
        .snapshot()
        .unwrap()
        .iter()
        .find(|n| n.kind == Kind::TextField)
        .unwrap()
        .id;
    h.focus(field).unwrap();
    assert!(!h.collection_text("é").unwrap());
}
#[test]
fn keys_and_focus_follow_reordering_and_replacement_but_validation_is_atomic() {
    let m = ListModel::new(items(200), 30.).multiple();
    let h = host(&m);
    layout(&h);
    let list = h.snapshot().unwrap()[0].id;
    h.select_collection_item(list, 2, false).unwrap();
    layout(&h);
    let token = m.item_token("id-2");
    let mut rows = items(200);
    rows.reverse();
    m.set_items(rows).unwrap();
    layout(&h);
    assert_eq!(cursor(&h), "id-2");
    assert_eq!(m.cursor().as_deref(), Some("id-2"));
    assert_eq!(m.item_token("id-2"), token);
    assert!(m.offset() > 5000.);
    assert!(
        m.set_items(vec![
            ListItem::new("x", "X"),
            ListItem::new("x", "Duplicate")
        ])
        .is_err()
    );
    assert_eq!(m.len(), 200);
    assert_eq!(selected(&m), ids(&[2]));
    let rows = items(200).into_iter().filter(|r| r.key != "id-2").collect();
    m.set_items(rows).unwrap();
    layout(&h);
    assert!(m.selected().is_empty());
    assert_ne!(cursor(&h), "id-2");
    let view = Host::new({
        let m = m.clone();
        move || {
            m.set_items(items(2)).unwrap();
            View::label("Invalid")
        }
    });
    assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| view.snapshot())).is_err());
    assert_eq!(m.len(), 199);
}
#[test]
fn configurable_overscan_stays_bounded_and_wheel_scrolling_does_not_snap_back() {
    let m = ListModel::new(items(100_000), 30.)
        .options(ListOptions {
            overscan_rows: 0,
            ..Default::default()
        })
        .unwrap();
    let h = host(&m);
    layout(&h);
    let list = h.snapshot().unwrap()[0].id;
    h.select_collection_item(list, 0, false).unwrap();
    layout(&h);
    h.scroll_to(list, Point { x: 0., y: 9000. }).unwrap();
    layout(&h);
    assert_eq!(m.offset(), 9000.);
    assert_eq!(m.visible_range(), 300..308);
    assert!(h.snapshot().unwrap().len() < 25);
    key(&h, Key::Down, Default::default());
    assert_eq!(cursor(&h), "id-1");
    assert!(m.offset() < 100.);
    let mut options = m.list_options();
    options.overscan_rows = 7;
    m.set_options(options).unwrap();
    layout(&h);
    assert!(m.visible_range().len() <= 8 + 14);
    assert!(h.snapshot().unwrap().len() < 70);
    options.visible_rows = 0;
    assert!(m.set_options(options).is_err());
    assert_eq!(m.list_options().visible_rows, 5);
}
#[test]
fn activation_allows_replacement_and_recursive_activation_is_guarded() {
    let held = Rc::new(std::cell::RefCell::new(None::<ListModel>));
    let calls = Rc::new(Cell::new(0));
    let m = model().on_activate({
        let held = held.clone();
        let calls = calls.clone();
        move |key| {
            calls.set(calls.get() + 1);
            let m = held.borrow().as_ref().unwrap().clone();
            assert!(!m.activate(&key));
            m.set_items(vec![]).unwrap();
        }
    });
    held.replace(Some(m.clone()));
    assert!(!m.activate("id-2"));
    assert!(m.activate("id-1"));
    assert_eq!(calls.get(), 1);
    assert!(!m.activate("id-1"));
    held.borrow_mut().take();
    let all = ListModel::new(
        vec![ListItem {
            key: "disabled".into(),
            cells: vec!["Disabled".into()],
            enabled: false,
        }],
        30.,
    );
    let h = host(&all);
    layout(&h);
    assert!(!h.focus_next(false).unwrap());
    assert!(!all.select("disabled", false));
}
