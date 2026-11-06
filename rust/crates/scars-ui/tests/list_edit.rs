use scars_ui::*;
use std::rc::Rc;
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
fn rows(count: usize) -> Vec<ListItem> {
    (0..count)
        .map(|i| ListItem::new(i.to_string(), format!("Row {i}")))
        .collect()
}
fn model(count: usize) -> ListModel {
    ListModel::new(rows(count), 56.)
}
fn layout(h: &Host) {
    h.layout(Size::new(400., 280.), &Metrics).unwrap();
}
fn field(h: &Host) -> Node {
    h.snapshot()
        .unwrap()
        .into_iter()
        .find(|n| n.key.as_deref() == Some("edit-0"))
        .unwrap()
}
fn host(m: &ListModel, value: &State<String>) -> Host {
    Host::new({
        let m = m.clone();
        let value = value.clone();
        move || {
            m.view_with(|item| {
                if item.key == "0" {
                    View::text_field("Draft", value.binding()).key("edit-0")
                } else {
                    View::label(item.label())
                }
            })
            .height(280)
        }
    })
}
#[test]
fn pins_are_shared_bounded_and_release_only_with_the_last_clone() {
    let m = model(20);
    let a = m.pin("0").unwrap();
    let clone = a.clone();
    let again = m.pin("0").unwrap();
    assert!(a == again);
    assert!(m.pin("1").is_err());
    assert!(m.pin("missing").is_err());
    assert_eq!(m.pinned_keys(), ["0"]);
    let mut options = m.list_options();
    options.max_pinned_rows = 0;
    assert!(m.set_options(options).is_err());
    assert_eq!(m.list_options().max_pinned_rows, 1);
    drop(a);
    drop(again);
    assert!(clone.is_valid());
    drop(clone);
    assert!(m.pinned_keys().is_empty());
    options.max_pinned_rows = 2;
    m.set_options(options).unwrap();
    let a = m.pin("0").unwrap();
    let b = m.pin("19").unwrap();
    assert_eq!(m.pinned_keys(), ["0", "19"]);
    drop(m);
    assert!(!a.is_valid());
    assert!(!b.is_valid());
}
#[test]
fn active_edit_keeps_identity_selection_history_and_preedit_offscreen_without_snapping_back() {
    let m = model(1000);
    let _pin = m.pin("0").unwrap();
    let value = State::new("draft".to_string());
    let h = host(&m, &value);
    layout(&h);
    let id = field(&h).id;
    h.focus(id).unwrap();
    h.set_text(id, "changed").unwrap();
    h.select_text(
        id,
        Selection {
            anchor: 2,
            caret: 4,
        },
    )
    .unwrap();
    layout(&h);
    h.mark_text("日本", 6..6, None).unwrap();
    layout(&h);
    let before = h.text_input().unwrap().unwrap();
    assert!(before.marked.is_some());
    let list = h.snapshot().unwrap()[0].id;
    h.scroll_to(list, Point { x: 0., y: 5600. }).unwrap();
    layout(&h);
    assert_eq!(m.offset(), 5600.);
    assert_eq!(field(&h).id, id);
    assert_eq!(h.focused(), Some(id));
    let after = h.text_input().unwrap().unwrap();
    assert_eq!(after.text, before.text);
    assert_eq!(after.selection, before.selection);
    assert_eq!(after.marked, before.marked);
    assert_eq!(value.get(), "changed");
    assert_eq!(field(&h).clip.height, 0.);
    assert!(m.realized_indices().len() <= m.visible_range().len() + 1);
    h.scroll_to(list, Point::default()).unwrap();
    layout(&h);
    h.unmark_text(false).unwrap();
    assert_eq!(
        h.text_input().unwrap().unwrap().selection,
        Selection {
            anchor: 2,
            caret: 4
        }
    );
    h.text_command(TextCommand::Undo, false).unwrap();
    assert_eq!(value.get(), "draft");
    assert!(h.text_input().unwrap().unwrap().can_redo);
}
#[test]
fn release_unrealizes_offscreen_editor_and_leaves_navigation_in_the_collection() {
    let m = model(1000);
    let pin = m.pin("0").unwrap();
    let value = State::new("draft".to_string());
    let h = host(&m, &value);
    layout(&h);
    let id = field(&h).id;
    h.focus(id).unwrap();
    h.set_text(id, "changed").unwrap();
    layout(&h);
    let list = h.snapshot().unwrap()[0].id;
    h.scroll_to(list, Point { x: 0., y: 5600. }).unwrap();
    layout(&h);
    drop(pin);
    layout(&h);
    assert!(!h.snapshot().unwrap().iter().any(|n| n.id == id));
    assert_eq!(h.focused(), Some(list));
    assert!(!h.set_text(id, "stale").unwrap());
    assert_eq!(value.get(), "changed");
    h.scroll_to(list, Point::default()).unwrap();
    layout(&h);
    assert_ne!(field(&h).id, id);
    h.focus(field(&h).id).unwrap();
    assert!(!h.text_input().unwrap().unwrap().can_undo);
}
#[test]
fn removed_and_reinserted_keys_get_fresh_editor_lifetimes_even_between_frames() {
    let m = model(10);
    let pin = m.pin("0").unwrap();
    let value = State::new("draft".to_string());
    let h = host(&m, &value);
    layout(&h);
    let id = field(&h).id;
    h.focus(id).unwrap();
    h.set_text(id, "changed").unwrap();
    h.mark_text("未", 3..3, None).unwrap();
    let token = pin.token();
    m.set_items(rows(10).into_iter().filter(|r| r.key != "0").collect())
        .unwrap();
    m.set_items(rows(10)).unwrap();
    assert!(!pin.is_valid());
    assert_ne!(m.item_token("0"), Some(token));
    layout(&h);
    assert_ne!(field(&h).id, id);
    assert!(!h.set_text(id, "stale").unwrap());
    assert_eq!(value.get(), "changed");
    h.focus(field(&h).id).unwrap();
    let input = h.text_input().unwrap().unwrap();
    assert!(input.marked.is_none());
    assert!(!input.can_undo);
    let fresh = m.pin("0").unwrap();
    drop(pin);
    assert!(fresh.is_valid());
}
#[test]
fn model_replacement_cannot_reuse_an_editor_with_the_same_application_key() {
    let original = model(10);
    let _pin = original.pin("0").unwrap();
    let current = State::new(original.clone());
    let value = State::new("draft".to_string());
    let h = Host::new({
        let current = current.clone();
        let value = value.clone();
        move || {
            current.get().view_with(|item| {
                if item.key == "0" {
                    View::text_field("Draft", value.binding()).key("edit-0")
                } else {
                    View::label(item.label())
                }
            })
        }
    });
    layout(&h);
    let old = field(&h).id;
    h.focus(old).unwrap();
    h.set_text(old, "changed").unwrap();
    current.set(model(10));
    layout(&h);
    assert_ne!(field(&h).id, old);
    assert!(!h.activate(old).unwrap());
    assert!(!h.set_text(old, "stale").unwrap());
    h.focus(field(&h).id).unwrap();
    assert!(!h.text_input().unwrap().unwrap().can_undo);
}
#[test]
fn disabling_invalidates_old_pins_and_their_drop_cannot_release_a_later_edit() {
    let m = model(10);
    let old = m.pin("0").unwrap();
    let mut disabled = rows(10);
    disabled[0].enabled = false;
    m.set_items(disabled).unwrap();
    assert!(!old.is_valid());
    assert!(m.pin("0").is_err());
    m.set_items(rows(10)).unwrap();
    let fresh = m.pin("0").unwrap();
    assert!(!old.is_valid());
    drop(old);
    assert!(fresh.is_valid());
    assert_eq!(m.pinned_keys(), ["0"]);
}
#[test]
fn pinned_rows_keep_indexed_geometry_through_sorting_and_model_context_avoids_a_cycle() {
    let m = model(1000);
    let pin = m.pin("0").unwrap();
    let value = State::new("draft".to_string());
    let h = host(&m, &value);
    layout(&h);
    let id = field(&h).id;
    h.focus(id).unwrap();
    h.set_text(id, "changed").unwrap();
    layout(&h);
    let mut reversed = rows(1000);
    reversed.reverse();
    m.set_items(reversed).unwrap();
    layout(&h);
    assert_eq!(field(&h).id, id);
    assert_eq!(h.focused(), Some(id));
    assert!(pin.is_valid());
    assert!(h.text_input().unwrap().unwrap().can_undo);
    let list = h.snapshot().unwrap()[0].id;
    let info = h.scroll_info(list).unwrap().unwrap();
    assert_eq!(info.content.height, 56000.);
    assert!(m.offset() > 55000.);
    let active = State::new(None::<RowPin>);
    let m = model(2).on_activate_with_model({
        let active = active.clone();
        move |model, key| active.set(Some(model.pin(&key).unwrap()))
    });
    assert!(m.activate("0"));
    let guard = active.get().unwrap();
    assert!(guard.is_valid());
    drop(m);
    assert!(!guard.is_valid());
}

#[test]
fn pin_cleanup_is_safe_during_description_replacement_and_builder_unwinding() {
    use std::cell::RefCell;
    let m = model(1000);
    let held = Rc::new(RefCell::new(Some(m.pin("0").unwrap())));
    m.reveal(100);
    let h = Host::new({
        let m = m.clone();
        let held = held.clone();
        move || {
            let view = m.view().height(280);
            held.borrow_mut().take();
            view
        }
    });
    layout(&h);
    assert!(m.pinned_keys().is_empty());
    assert!(
        !h.snapshot()
            .unwrap()
            .iter()
            .any(|n| n.key.as_deref() == Some("0"))
    );
    let held = Rc::new(RefCell::new(Some(m.pin("0").unwrap())));
    let h = Host::new({
        let held = held.clone();
        move || {
            let _local_pin = held.borrow_mut().take();
            panic!("Builder failed while owning a pin");
        }
    });
    assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| h.snapshot())).is_err());
    assert!(m.pinned_keys().is_empty());
    assert!(m.pin("1").is_ok());
}
