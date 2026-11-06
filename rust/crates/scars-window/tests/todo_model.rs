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
fn external_items_do_not_collide_with_new_keys_or_revive_deleted_bindings() {
    let model = Todos::load(None).unwrap();
    model.items.set(vec![Todo {
        id: 1,
        title: "Imported".into(),
        done: false,
    }]);
    model.input.set("Added".into());
    model.add();
    assert_eq!(
        model.items.get().iter().map(|t| t.id).collect::<Vec<_>>(),
        [1, 2]
    );
    let stale = model.done(2);
    model.remove(2);
    model.input.set("Replacement".into());
    model.add();
    stale.set(true);
    assert!(!model.items.get()[1].done);
    assert_eq!(model.items.get()[1].id, 3);
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

#[test]
fn themed_todo_view_filters_persists_and_reflects_external_values_without_losing_keys() {
    use scars_window::ui::*;
    for (theme, palette) in [("dark", Theme::default()), ("light", Theme::light())] {
        let directory =
            std::env::temp_dir().join(format!("scars-todo-ui-{}-{theme}", std::process::id()));
        let path = directory.join("tasks.tsv");
        let model = Todos::load(Some(path.clone())).unwrap();
        let host = Host::new({
            let model = model.clone();
            move || todo_view::view(&model)
        });
        host.set_theme(palette);
        let renderer = Renderer::default();
        let layout = || host.layout(Size::new(760., 720.), &renderer).unwrap();
        let find = |label: &str| {
            host.snapshot()
                .unwrap()
                .into_iter()
                .find(|n| n.label == label)
                .unwrap_or_else(|| panic!("Missing {label}"))
        };
        layout();
        let input = host.text_input().unwrap().unwrap().id;
        model.input.set("External draft 👩🏽‍💻".into());
        layout();
        assert_eq!(host.text_input().unwrap().unwrap().id, input);
        assert_eq!(host.text_input().unwrap().unwrap().text, model.input.get());
        host.activate(find("Add task").id).unwrap();
        layout();
        let id = model.items.get()[0].id;
        let checkbox = find("Complete External draft 👩🏽‍💻").id;
        model.items.update(|items| {
            items[0].title = "Externally renamed".into();
            items[0].done = true;
        });
        layout();
        let changed = find("Complete Externally renamed");
        assert_eq!(changed.id, checkbox);
        assert_eq!(changed.value, Value::Bool(true));
        host.activate(find("Active").id).unwrap();
        layout();
        assert_eq!(model.filter.get(), "active");
        assert!(!host.snapshot().unwrap().iter().any(|n| n.id == checkbox));
        host.activate(find("Completed").id).unwrap();
        layout();
        assert_eq!(model.visible()[0].id, id);
        host.activate(find("Edit Externally renamed").id).unwrap();
        layout();
        host.text_command(TextCommand::SelectAll, false).unwrap();
        host.insert_text("Saved title", None).unwrap();
        host.activate(find("Save").id).unwrap();
        layout();
        assert_eq!(
            Todos::load(Some(path.clone())).unwrap().items.get(),
            model.items.get()
        );
        host.activate(find("Clear completed").id).unwrap();
        layout();
        assert!(model.items.get().is_empty());
        assert!(Todos::load(Some(path)).unwrap().items.get().is_empty());
        std::fs::remove_dir_all(directory).unwrap();
    }
}

#[test]
fn failed_save_keeps_edits_visible_and_recovers_when_storage_becomes_available() {
    use scars_window::ui::*;
    let directory =
        std::env::temp_dir().join(format!("scars-todo-recovery-{}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    let parent = directory.join("storage");
    let path = parent.join("tasks.tsv");
    let model = Todos::load(Some(path.clone())).unwrap();
    std::fs::write(&parent, "Blocks directory creation").unwrap();
    model.input.set("Keep this change".into());
    model.add();
    let host = Host::new({
        let model = model.clone();
        move || todo_view::view(&model)
    });
    let renderer = Renderer::default();
    host.layout(Size::new(760., 720.), &renderer).unwrap();
    assert_eq!(model.items.get()[0].title, "Keep this change");
    assert!(
        host.scene()
            .unwrap()
            .iter()
            .any(|n| n.text.starts_with("Changes are in memory; saving failed:"))
    );
    std::fs::remove_file(parent).unwrap();
    model.rename(model.items.get()[0].id, "Recovered save");
    host.layout(Size::new(760., 720.), &renderer).unwrap();
    assert!(model.error.get().is_none());
    assert_eq!(
        Todos::load(Some(path)).unwrap().items.get(),
        model.items.get()
    );
    std::fs::remove_dir_all(directory).unwrap();
}
