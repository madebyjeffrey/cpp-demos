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
fn items() -> Vec<OptionItem> {
    vec![
        OptionItem::new("a", "Apple"),
        OptionItem {
            id: "off".into(),
            label: "Apricot".into(),
            enabled: false,
        },
        OptionItem::new("b", "Banana"),
        OptionItem::new("c", "Crème"),
        OptionItem::new("d", "Pineapple"),
    ]
}
fn host(c: &ComboBox) -> Host {
    let c = c.clone();
    Host::new(move || {
        View::column(vec![
            View::button("Before", || {}).key("before"),
            c.view("Choose fruit"),
            View::button("After", || {}).key("after"),
        ])
    })
}
fn layout(h: &Host) {
    h.layout(Size::new(500., 400.), &Metrics).unwrap();
}
fn field(h: &Host) -> NodeId {
    h.snapshot()
        .unwrap()
        .iter()
        .find(|n| n.combo.is_some())
        .unwrap()
        .id
}
fn key(h: &Host, key: Key) {
    assert!(
        h.key(KeyEvent {
            key,
            down: true,
            repeat: false,
            modifiers: Default::default()
        })
        .unwrap()
    );
    layout(h);
}
fn focus(h: &Host) {
    layout(h);
    h.focus(field(h)).unwrap();
}
#[test]
fn query_and_choice_are_independent_cancel_restores_session_and_callbacks_see_published_values() {
    let queries = State::new(vec![]);
    let accepted = State::new(vec![]);
    let canceled = Rc::new(Cell::new(0));
    let c = ComboBox::new(items())
        .unwrap()
        .on_query(queries.handler(|s, q| s.update(|v| v.push(q))))
        .on_accept(accepted.handler(|s, v| s.update(|a| a.push(v))))
        .on_cancel({
            let n = canceled.clone();
            move || n.set(n.get() + 1)
        });
    c.set_selection(Some("b")).unwrap();
    let h = host(&c);
    focus(&h);
    h.set_text(field(&h), "AP").unwrap();
    layout(&h);
    assert_eq!(c.selection().as_deref(), Some("b"));
    assert_eq!(c.info().matches, 3);
    assert_eq!(c.highlighted().as_deref(), Some("a"));
    assert_eq!(queries.get(), ["AP"]);
    key(&h, Key::Down);
    assert_eq!(c.highlighted().as_deref(), Some("d"));
    assert_eq!(h.focused(), Some(field(&h)));
    key(&h, Key::Enter);
    assert_eq!(c.selection().as_deref(), Some("d"));
    assert_eq!(c.query(), "Pineapple");
    assert!(!c.is_open());
    assert_eq!(
        accepted.get(),
        [ComboAccept {
            selection: Some("d".into()),
            text: "Pineapple".into()
        }]
    );
    h.set_text(field(&h), "new").unwrap();
    layout(&h);
    key(&h, Key::Escape);
    assert_eq!(c.query(), "Pineapple");
    assert_eq!(c.selection().as_deref(), Some("d"));
    assert_eq!(canceled.get(), 1);
    assert!(!c.cancel());
    c.set_query("external").unwrap();
    assert_eq!(queries.get(), ["AP", "new"]);
}
#[test]
fn selection_and_free_text_filter_modes_loading_empty_and_read_only() {
    let c = ComboBox::with_options(
        items(),
        ComboBoxOptions {
            filter: ComboBoxFilter::Prefix,
            auto_highlight: false,
            ..Default::default()
        },
    )
    .unwrap();
    c.set_query("p").unwrap();
    assert_eq!(c.info().matches, 1);
    assert_eq!(c.highlighted(), None);
    c.show();
    assert!(!c.accept());
    assert!(c.accept_key("d"));
    c.set_loading(true);
    c.show();
    assert!(!c.accept_key("d"));
    let h = host(&c);
    focus(&h);
    assert!(h.snapshot().unwrap().iter().any(|n| n.text == "Loading…"));
    c.set_loading(false);
    c.set_query("none").unwrap();
    layout(&h);
    assert!(h.snapshot().unwrap().iter().any(|n| n.text == "No matches"));
    assert!(!c.accept());
    c.set_read_only(true);
    layout(&h);
    assert!(!c.is_open());
    assert!(!c.show());
    assert!(!h.set_text(field(&h), "x").unwrap());
    let free = ComboBox::with_options(
        items(),
        ComboBoxOptions {
            mode: ComboBoxMode::FreeText,
            filter: ComboBoxFilter::None,
            auto_highlight: false,
            ..Default::default()
        },
    )
    .unwrap();
    free.set_query("Custom").unwrap();
    free.show();
    assert_eq!(free.info().matches, 5);
    assert!(free.accept());
    assert_eq!(free.query(), "Custom");
    assert_eq!(free.selection(), None);
}
#[test]
fn keys_skip_disabled_and_tab_leaves_field_while_composition_commits_before_accepting() {
    let c = ComboBox::new(items()).unwrap();
    let h = host(&c);
    focus(&h);
    key(&h, Key::Down);
    assert_eq!(c.highlighted().as_deref(), Some("a"));
    key(&h, Key::Down);
    assert_eq!(c.highlighted().as_deref(), Some("b"));
    key(&h, Key::End);
    assert_eq!(c.highlighted().as_deref(), Some("d"));
    key(&h, Key::Home);
    assert_eq!(c.highlighted().as_deref(), Some("a"));
    h.mark_text("Cr", 2..2, Some(0..0)).unwrap();
    key(&h, Key::Enter);
    assert!(c.is_open());
    assert_eq!(c.selection(), None);
    assert_eq!(c.query(), "Cr");
    assert_eq!(c.highlighted().as_deref(), Some("c"));
    key(&h, Key::Enter);
    assert_eq!(c.selection().as_deref(), Some("c"));
    key(&h, Key::Down);
    key(&h, Key::Tab);
    assert!(!c.is_open());
    let after = h
        .snapshot()
        .unwrap()
        .iter()
        .find(|n| n.key.as_deref() == Some("after"))
        .unwrap()
        .id;
    assert_eq!(h.focused(), Some(after));
    h.focus(field(&h)).unwrap();
    assert!(h.text_command(TextCommand::Down, false).unwrap());
    layout(&h);
    assert!(c.is_open());
    assert!(h.text_command(TextCommand::Tab, false).unwrap());
    assert_eq!(h.focused(), Some(after));
}
#[test]
fn replacement_validates_atomically_preserves_keys_and_does_not_accept_retired_rows() {
    let c = ComboBox::new(items()).unwrap();
    c.set_selection(Some("b")).unwrap();
    c.set_query("").unwrap();
    c.show();
    let h = host(&c);
    focus(&h);
    assert_eq!(c.highlighted().as_deref(), Some("b"));
    assert!(
        c.set_items(vec![
            OptionItem::new("x", "one"),
            OptionItem::new("x", "two")
        ])
        .is_err()
    );
    assert_eq!(c.info().matches, 5);
    c.set_items(vec![
        OptionItem::new("b", "Renamed"),
        OptionItem::new("z", "Zed"),
    ])
    .unwrap();
    assert_eq!(c.selection().as_deref(), Some("b"));
    assert_eq!(c.highlighted().as_deref(), Some("b"));
    c.set_items(vec![]).unwrap();
    assert_eq!(c.selection(), None);
    assert_eq!(c.highlighted(), None);
    assert!(!c.accept_key("b"));
    assert!(
        ComboBox::with_options(
            items(),
            ComboBoxOptions {
                row_height: 0.,
                ..Default::default()
            }
        )
        .is_err()
    );
    assert!(
        ComboBox::with_options(
            items(),
            ComboBoxOptions {
                max_height: f64::INFINITY,
                ..Default::default()
            }
        )
        .is_err()
    );
}
#[test]
fn large_popup_realizes_a_bounded_range_and_indexed_press_accepts_beyond_old_cap() {
    let c = ComboBox::new(
        (0..100_000)
            .map(|i| OptionItem::new(i.to_string(), format!("Choice {i:06}")))
            .collect(),
    )
    .unwrap();
    let h = host(&c);
    focus(&h);
    key(&h, Key::Down);
    assert_eq!(c.info().matches, 100_000);
    assert!(h.snapshot().unwrap().len() < 80);
    key(&h, Key::End);
    assert_eq!(c.highlighted().as_deref(), Some("99999"));
    assert!(h.snapshot().unwrap().len() < 80);
    let list = h
        .snapshot()
        .unwrap()
        .iter()
        .find(|n| n.collection.is_some() && n.combo_popup.is_some())
        .unwrap()
        .id;
    assert!(h.select_collection_item(list, 54321, false).unwrap());
    layout(&h);
    assert_eq!(c.selection().as_deref(), Some("54321"));
    assert_eq!(h.focused(), Some(field(&h)));
    assert!(!c.is_open());
    assert!(!h.select_collection_item(list, 99999, false).unwrap());
}
#[test]
fn query_callbacks_can_replace_results_and_owner_removal_closes_once() {
    let cancels = Rc::new(Cell::new(0));
    let c = ComboBox::new(items())
        .unwrap()
        .on_query_with_model(|c, q| {
            c.set_loading(false);
            c.set_items(vec![OptionItem::new("result", q)]).unwrap();
        })
        .on_cancel({
            let n = cancels.clone();
            move || n.set(n.get() + 1)
        });
    let visible = State::new(true);
    let h = Host::new({
        let visible = visible.clone();
        let c = c.clone();
        move || {
            if visible.get() {
                c.view("query")
            } else {
                View::label("gone")
            }
        }
    });
    focus(&h);
    h.set_text(field(&h), "Reply").unwrap();
    layout(&h);
    assert_eq!(c.info().matches, 1);
    assert_eq!(c.highlighted().as_deref(), Some("result"));
    visible.set(false);
    layout(&h);
    assert!(!c.is_open());
    assert_eq!(cancels.get(), 1);
    drop(h);
    assert_eq!(cancels.get(), 1);
}
#[test]
fn pointer_accepts_once_outside_and_blur_cancel_and_popup_geometry_tracks_owner() {
    let cancels = Rc::new(Cell::new(0));
    let accepts = Rc::new(Cell::new(0));
    let c = ComboBox::with_options(
        items(),
        ComboBoxOptions {
            max_length: 3,
            ..Default::default()
        },
    )
    .unwrap()
    .on_cancel({
        let n = cancels.clone();
        move || n.set(n.get() + 1)
    })
    .on_accept({
        let n = accepts.clone();
        move |_| n.set(n.get() + 1)
    });
    let h = host(&c);
    focus(&h);
    key(&h, Key::Down);
    let nodes = h.snapshot().unwrap();
    let owner = nodes.iter().find(|n| n.combo.is_some()).unwrap();
    let list = nodes.iter().find(|n| n.collection.is_some()).unwrap();
    assert_eq!(list.bounds.width, owner.bounds.width);
    let r = nodes
        .iter()
        .find(|n| n.kind == Kind::CollectionRow && n.key.as_deref() == Some("b"))
        .unwrap();
    for kind in [PointerKind::Down, PointerKind::Up] {
        h.pointer(Pointer {
            kind,
            position: Point {
                x: r.bounds.x + 10.,
                y: r.bounds.y + 10.,
            },
            button: 0,
            modifiers: Default::default(),
        })
        .unwrap();
    }
    layout(&h);
    assert_eq!(c.selection().as_deref(), Some("b"));
    assert_eq!(accepts.get(), 1);
    assert_eq!(c.query(), "Banana");
    h.set_text(field(&h), "Apple").unwrap();
    layout(&h);
    assert_eq!(c.query(), "App");
    assert!(c.is_open());
    h.mark_text("X", 1..1, None).unwrap();
    key(&h, Key::Escape);
    assert!(c.is_open());
    assert_eq!(cancels.get(), 0);
    key(&h, Key::Escape);
    assert_eq!(c.query(), "Banana");
    assert_eq!(cancels.get(), 1);
    key(&h, Key::Down);
    h.pointer(Pointer {
        kind: PointerKind::Down,
        position: Point { x: 499., y: 399. },
        button: 0,
        modifiers: Default::default(),
    })
    .unwrap();
    layout(&h);
    assert!(!c.is_open());
    assert_eq!(cancels.get(), 2);
    key(&h, Key::Down);
    h.blur();
    layout(&h);
    assert!(!c.is_open());
    assert_eq!(cancels.get(), 3);
}
