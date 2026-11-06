use scars_ui::*;
use std::{
    cell::RefCell,
    rc::Rc,
    time::{Duration, Instant},
};
fn field(host: &Host) -> Node {
    host.snapshot()
        .unwrap()
        .into_iter()
        .find(|n| n.kind == Kind::TextField)
        .unwrap()
}
#[test]
fn search_debounces_committed_queries_and_submit_flushes_before_callback() {
    let query = State::new(String::new());
    let events = Rc::new(RefCell::new(Vec::new()));
    let host = Host::new({
        let query = query.clone();
        let events = events.clone();
        move || {
            let changed = events.clone();
            let submit = events.clone();
            view! {search_field("Search",bind:query,debounce:Duration::from_secs(60),on_query:move|q|changed.borrow_mut().push(q),on_submit:move||submit.borrow_mut().push("submit".into()));}
        }
    });
    host.focus(field(&host).id).unwrap();
    host.insert_text("r", None).unwrap();
    host.insert_text("u", None).unwrap();
    host.run_due();
    assert!(events.borrow().is_empty());
    host.mark_text("日", 3..3, None).unwrap();
    assert_eq!(query.get(), "ru");
    host.unmark_text(true).unwrap();
    assert_eq!(query.get(), "ru日");
    host.text_command(TextCommand::Enter, false).unwrap();
    assert_eq!(*events.borrow(), ["ru日", "submit"]);
    assert!(host.next_deadline().is_none());
    host.run_due_at(Instant::now() + Duration::from_secs(120));
    assert_eq!(events.borrow().len(), 2);
}
#[test]
fn pending_search_is_cancelled_by_external_replace_hide_remove_disable_and_deactivate() {
    for mode in 0..7 {
        let value = State::new(String::new());
        let visible = State::new(true);
        let enabled = State::new(true);
        let hidden = State::new(false);
        let readonly = State::new(false);
        let events = Rc::new(RefCell::new(Vec::new()));
        let host = Host::new({
            let readonly = readonly.clone();
            let (value, visible, enabled, hidden, events) = (
                value.clone(),
                visible.clone(),
                enabled.clone(),
                hidden.clone(),
                events.clone(),
            );
            move || {
                let events = events.clone();
                view! {column{if visible.get(){search_field("Search",bind:value,key:"search",enabled:enabled.get(),hidden:hidden.get(),read_only:readonly.get(),debounce:Duration::from_millis(20),on_query:move|q|events.borrow_mut().push(q));}}}
            }
        });
        host.focus(field(&host).id).unwrap();
        host.insert_text("pending", None).unwrap();
        match mode {
            0 => value.set("external".into()),
            1 => visible.set(false),
            2 => enabled.set(false),
            3 => hidden.set(true),
            4 => host.set_active(false),
            5 => host.blur(),
            _ => readonly.set(true),
        }
        assert!(host.next_deadline().is_none());
        host.run_due_at(Instant::now() + Duration::from_secs(1));
        assert!(events.borrow().is_empty());
    }
}
#[test]
fn clear_and_escape_publish_empty_query_restore_focus_and_respect_readonly() {
    let value = State::new("query".to_owned());
    let cancelled = State::new(0);
    let readonly = State::new(false);
    let events = Rc::new(RefCell::new(Vec::new()));
    let host = Host::new({
        let (value, cancelled, readonly, events) = (
            value.clone(),
            cancelled.clone(),
            readonly.clone(),
            events.clone(),
        );
        move || {
            let events = events.clone();
            view! {search_field("Search",bind:value,key:"search",read_only:readonly.get(),on_query:move|q|events.borrow_mut().push(q),on_cancel:cancelled.action(|n|n.update(|n|*n+=1)));}
        }
    });
    let id = field(&host).id;
    let clear = host
        .snapshot()
        .unwrap()
        .into_iter()
        .find(|n| n.label == "Clear search")
        .unwrap()
        .id;
    host.activate(clear).unwrap();
    host.run_due();
    assert_eq!(*events.borrow(), [""]);
    assert_eq!(value.get(), "");
    assert_eq!(host.focused(), Some(id));
    host.text_command(TextCommand::Undo, false).unwrap();
    assert_eq!(value.get(), "query");
    host.text_command(TextCommand::Escape, false).unwrap();
    host.run_due();
    assert_eq!(value.get(), "");
    assert_eq!(cancelled.get(), 1);
    value.set("readonly".into());
    readonly.set(true);
    assert!(!host.clear_search(id).unwrap());
    assert_eq!(value.get(), "readonly");
    assert!(!host.activate(clear).unwrap());
}
#[test]
fn query_callback_can_remove_all_owners_without_running_stale_callbacks() {
    let visible = State::new(true);
    let first = State::new(String::new());
    let second = State::new(String::new());
    let events = State::new(0);
    let host = Host::new({
        let (visible, first, second, events) = (
            visible.clone(),
            first.clone(),
            second.clone(),
            events.clone(),
        );
        move || {
            let (v1, v2, e1, e2) = (
                visible.clone(),
                visible.clone(),
                events.clone(),
                events.clone(),
            );
            view! {column{if visible.get(){
                search_field("First",bind:first,key:"a",on_query:move|_|{e1.update(|n|*n+=1);v1.set(false);});
                search_field("Second",bind:second,key:"b",on_query:move|_|{e2.update(|n|*n+=1);v2.set(false);});
            }}}
        }
    });
    let fields: Vec<_> = host
        .snapshot()
        .unwrap()
        .into_iter()
        .filter(|n| n.kind == Kind::TextField)
        .map(|n| n.id)
        .collect();
    for id in fields {
        host.set_text(id, "query").unwrap();
    }
    host.run_due();
    assert_eq!(events.get(), 1);
    assert!(host.next_deadline().is_none());
}
#[test]
fn password_reveal_is_explicit_sensitive_and_remasked_on_focus_loss_or_deactivation() {
    let text = State::new(String::new());
    let allow = State::new(false);
    let host = Host::new({
        let text = text.clone();
        let allow = allow.clone();
        move || view! {column{password_field("Password",bind:text,key:"password",allow_reveal:allow.get(),max_length:2);button("Other",key:"other",on_click:||{});}}
    });
    let id = field(&host).id;
    host.focus(id).unwrap();
    assert!(!host.set_password_revealed(id, true).unwrap());
    host.insert_text("😀e\u{301}extra", None).unwrap();
    assert_eq!(text.get(), "😀e\u{301}");
    assert!(!host.text_input().unwrap().unwrap().can_undo);
    allow.set(true);
    assert!(host.set_password_revealed(id, true).unwrap());
    assert!(field(&host).revealed);
    host.text_command(TextCommand::SelectAll, false).unwrap();
    assert_eq!(host.selected_text().unwrap(), None);
    assert!(host.text_input().unwrap().unwrap().secure);
    assert!(host.text_command(TextCommand::Escape, false).unwrap());
    assert!(!field(&host).revealed);
    host.set_password_revealed(id, true).unwrap();
    host.set_active(false);
    assert!(!host.set_password_revealed(id, true).unwrap());
    assert!(!field(&host).revealed);
    host.set_active(true);
    host.set_password_revealed(id, true).unwrap();
    let other = host
        .snapshot()
        .unwrap()
        .into_iter()
        .find(|n| n.key.as_deref() == Some("other"))
        .unwrap()
        .id;
    host.focus(other).unwrap();
    assert!(!field(&host).revealed);
}
#[test]
fn submit_flushes_only_its_owner_and_a_query_can_cancel_submission_by_removing_it() {
    let first = State::new(String::new());
    let second = State::new(String::new());
    let events = State::new(Vec::<String>::new());
    let visible = State::new(true);
    let host = Host::new({
        let (first, second, events, visible) = (
            first.clone(),
            second.clone(),
            events.clone(),
            visible.clone(),
        );
        move || {
            let (events1, events2, events3, visible) = (
                events.clone(),
                events.clone(),
                events.clone(),
                visible.clone(),
            );
            view! {column{
                if visible.get(){search_field("First",bind:first,key:"first",debounce:Duration::from_secs(60),on_query:move|q|{events1.update(|v|v.push(q.clone()));if q=="remove"{visible.set(false);}},on_submit:events2.action(|v|v.update(|v|v.push("submit".into()))));}
                search_field("Second",bind:second,key:"second",debounce:Duration::from_secs(60),on_query:events3.handler(|v,q|v.update(|v|v.push(q))));
            }}
        }
    });
    let nodes = host.snapshot().unwrap();
    let a = nodes
        .iter()
        .find(|n| n.key.as_deref() == Some("first"))
        .unwrap()
        .id;
    let b = nodes
        .iter()
        .find(|n| n.key.as_deref() == Some("second"))
        .unwrap()
        .id;
    host.set_text(a, "first").unwrap();
    host.set_text(b, "second").unwrap();
    host.focus(a).unwrap();
    host.text_command(TextCommand::Enter, false).unwrap();
    assert_eq!(events.get(), ["first", "submit"]);
    host.run_due_at(Instant::now() + Duration::from_secs(120));
    assert_eq!(events.get(), ["first", "submit", "second"]);
    host.set_text(a, "remove").unwrap();
    host.text_command(TextCommand::Enter, false).unwrap();
    assert_eq!(events.get(), ["first", "submit", "second", "remove"]);
}
