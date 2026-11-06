use scars_ui::{Binding, Host, Kind, State, View, view};
use std::{
    cell::{Cell, RefCell},
    panic::{AssertUnwindSafe, catch_unwind},
    rc::Rc,
};

#[test]
fn subscriptions_follow_conditional_reads_and_writes_coalesce() {
    let choose_left = State::new(true);
    let left = State::new(1);
    let right = State::new(10);
    let host = Host::new({
        let (choose_left, left, right) = (choose_left.clone(), left.clone(), right.clone());
        move || view! { label((if choose_left.get() { left.get() } else { right.get() }).to_string()); }
    });
    let wakes = Rc::new(Cell::new(0));
    host.on_invalidate({
        let wakes = wakes.clone();
        move || wakes.set(wakes.get() + 1)
    });
    assert!(host.refresh().unwrap());
    left.set(1);
    assert!(!host.refresh().unwrap());
    left.set(2);
    left.set(3);
    assert_eq!(wakes.get(), 1);
    assert!(host.refresh().unwrap());
    choose_left.set(false);
    host.refresh().unwrap();
    left.set(4);
    assert!(
        !host.refresh().unwrap(),
        "Removed dependencies must stop waking a host"
    );
    right.set(11);
    assert!(host.refresh().unwrap());
    let before = wakes.get();
    drop(host);
    right.set(12);
    assert_eq!(wakes.get(), before, "State must not retain a dropped host");
}

#[test]
fn macro_reordering_preserves_keys_and_removed_actions_become_inert() {
    fn heading() -> View {
        View::label("Heading")
    }
    let items = State::new(vec!["a", "b"]);
    let clicks = State::new(0);
    let host = Host::new({
        let (items, clicks) = (items.clone(), clicks.clone());
        move || {
            view! {
                column {
                    heading();
                    for item in items.get() {
                        button(item, key: item, on_click: {
                            let clicks = clicks.clone();
                            move || clicks.update(|n| *n += 1)
                        });
                    }
                    if clicks.get() > 1 { label("many"); }
                    else if clicks.get() == 1 { label("one"); }
                    else { label("none"); }
                }
            }
        }
    });
    let initial = host.snapshot().unwrap();
    let a = initial
        .iter()
        .find(|n| n.key.as_deref() == Some("a"))
        .unwrap()
        .id;
    items.set(vec!["b", "a"]);
    assert_eq!(
        host.snapshot()
            .unwrap()
            .iter()
            .find(|n| n.key.as_deref() == Some("a"))
            .unwrap()
            .id,
        a
    );
    assert!(host.activate(a).unwrap());
    assert!(host.snapshot().unwrap().iter().any(|n| n.text == "one"));
    items.set(vec!["b"]);
    assert!(!host.activate(a).unwrap());
    items.set(vec!["a", "b"]);
    assert_ne!(
        host.snapshot()
            .unwrap()
            .iter()
            .find(|n| n.key.as_deref() == Some("a"))
            .unwrap()
            .id,
        a
    );
    let other = Host::new(|| view! { button("other", on_click: || {}); });
    let foreign = other
        .snapshot()
        .unwrap()
        .iter()
        .find(|n| n.kind == Kind::Button)
        .unwrap()
        .id;
    assert!(!host.activate(foreign).unwrap());
}

#[test]
fn failed_and_panicking_builds_restore_tracking_and_keep_identity() {
    let mode = State::new(0);
    let host = Host::new({
        let mode = mode.clone();
        move || match mode.get() {
            1 => view! { row { label("bad", key: "x"); label("duplicate", key: "x"); } },
            2 => {
                mode.set(0);
                unreachable!()
            } // Writes inside a builder must be rejected.
            _ => view! { label("retained", key: "good"); },
        }
    });
    let initial = host.snapshot().unwrap();
    let generation = host.generation();
    mode.set(1);
    assert!(host.refresh().unwrap_err().0.contains("Duplicate"));
    assert_eq!(host.generation(), generation);
    mode.set(2);
    assert!(catch_unwind(AssertUnwindSafe(|| host.refresh())).is_err());
    mode.set(0); // The failed construction must not leave the thread in build mode.
    let next = host.snapshot().unwrap();
    assert_eq!(initial.last().unwrap().id, next.last().unwrap().id);
}

#[test]
fn descriptions_bindings_and_action_callbacks_do_not_share_mutable_borrows() {
    let value = State::new(0);
    let binding = Binding::new(
        {
            let value = value.clone();
            move || value.get()
        },
        {
            let value = value.clone();
            move |v: i32| value.set(v.clamp(0, 10))
        },
    );
    binding.set(100);
    assert_eq!(binding.get(), 10);
    let base = View::label("base");
    let keyed = base.clone().key("changed");
    let host = Host::new(move || View::column(vec![base.clone(), keyed.clone()]));
    let nodes = host.snapshot().unwrap();
    assert_eq!(nodes[1].key, None);
    assert_eq!(nodes[2].key.as_deref(), Some("changed"));

    let slot = Rc::new(RefCell::new(None::<Host>));
    let weak = Rc::downgrade(&slot);
    let action = Host::new(move || {
        let weak = weak.clone();
        view! { button("query", on_click: move || {
            let target = weak.upgrade().unwrap().borrow().clone().unwrap();
            target.snapshot().unwrap();
        }); }
    });
    *slot.borrow_mut() = Some(action.clone());
    let id = action.snapshot().unwrap().last().unwrap().id;
    assert!(action.activate(id).unwrap());
    // Generated local names must not shadow application expressions.
    let __scars_children = "caller variable";
    let hygiene = Host::new(move || view! { label(__scars_children); });
    assert_eq!(
        hygiene.snapshot().unwrap().last().unwrap().text,
        "caller variable"
    );
}
