#[path = "../examples/todo/model.rs"]
mod model;
use model::{Todo, Todos, decode, encode};
#[test]
fn todo_actions_filters_and_stale_bindings_follow_keys() {
    let model = Todos::load(None).unwrap();
    model.input.set("  First task  ".into());
    model.add();
    model.input.set("Second".into());
    model.add();
    let first = model.items.get()[0].id;
    let binding = model.done(first);
    binding.set(true);
    assert_eq!(model.remaining(), 1);
    model.filter.set("active".into());
    assert_eq!(model.visible().len(), 1);
    model.begin_edit(first);
    model.draft.set("Renamed".into());
    model.commit_edit();
    assert_eq!(model.items.get()[0].title, "Renamed");
    model.remove(first);
    binding.set(false);
    assert_eq!(model.items.get().len(), 1);
    model.all_done().set(true);
    model.clear_completed();
    assert!(model.items.get().is_empty());
    model.input.set("   ".into());
    model.add();
    assert!(model.items.get().is_empty());
}
#[test]
fn storage_round_trips_unicode_escapes_and_rejects_corruption() {
    let items = vec![Todo {
        id: 9,
        title: "A\tB\\C\n😀".into(),
        done: true,
    }];
    assert_eq!(decode(&encode(&items)).unwrap(), items);
    for bad in [
        "bad",
        "SCARS-TODO-1\n1\t0\tx\n1\t1\ty\n",
        "SCARS-TODO-1\n1\t0\tbad\\q\n",
    ] {
        assert!(decode(bad).is_err());
    }
}
#[test]
fn atomic_save_can_be_reloaded_and_cancel_keeps_original() {
    let dir = std::env::temp_dir().join(format!("scars-todo-test-{}", std::process::id()));
    let path = dir.join("tasks.tsv");
    let _ = std::fs::remove_dir_all(&dir);
    let model = Todos::load(Some(path.clone())).unwrap();
    model.input.set("Saved 😀".into());
    model.add();
    let loaded = Todos::load(Some(path)).unwrap();
    assert_eq!(loaded.items.get(), model.items.get());
    loaded.begin_edit(loaded.items.get()[0].id);
    loaded.draft.set("Discard".into());
    loaded.cancel_edit();
    assert_eq!(loaded.items.get()[0].title, "Saved 😀");
    let _ = model.action(|m| {
        m.remaining();
    });
    let _ = Todos::preview();
    std::fs::remove_dir_all(dir).unwrap();
}

#[path = "../examples/todo/view.rs"]
mod todo_view;
#[test]
fn real_todo_view_submits_and_focuses_inline_edit_then_cancels_or_saves() {
    use scars_window::ui::*;
    let model = Todos::load(None).unwrap();
    let host = Host::new({
        let model = model.clone();
        move || todo_view::view(&model)
    });
    let renderer = Renderer::default();
    let layout = || host.layout(Size::new(760., 720.), &renderer).unwrap();
    layout();
    host.insert_text("Buy milk", None).unwrap();
    host.text_command(TextCommand::Enter, false).unwrap();
    layout();
    assert_eq!(model.items.get().len(), 1);
    let edit = || {
        host.snapshot()
            .unwrap()
            .into_iter()
            .find(|n| n.text == "Edit")
            .unwrap()
            .id
    };
    host.activate(edit()).unwrap();
    layout();
    assert_eq!(host.text_input().unwrap().unwrap().text, "Buy milk");
    host.text_command(TextCommand::SelectAll, false).unwrap();
    host.insert_text("Discard", None).unwrap();
    host.text_command(TextCommand::Escape, false).unwrap();
    layout();
    assert_eq!(model.items.get()[0].title, "Buy milk");
    host.activate(edit()).unwrap();
    layout();
    host.text_command(TextCommand::SelectAll, false).unwrap();
    host.insert_text("Buy oat milk", None).unwrap();
    host.text_command(TextCommand::Enter, false).unwrap();
    layout();
    assert_eq!(model.items.get()[0].title, "Buy oat milk");
    assert_eq!(model.editing.get(), None);
}
