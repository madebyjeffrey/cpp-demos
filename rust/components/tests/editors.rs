use scars_ui::*;
#[test]
fn undo_redo_retains_selection_and_external_changes_reset_history() {
    let value = State::new(String::new());
    let host = Host::new({
        let value = value.clone();
        move || view! {text_editor("Notes",bind:value);}
    });
    let id = host.snapshot().unwrap().last().unwrap().id;
    host.focus(id).unwrap();
    host.insert_text("First", None).unwrap();
    host.text_command(TextCommand::Enter, false).unwrap();
    host.insert_text("Second", None).unwrap();
    assert_eq!(value.get(), "First\nSecond");
    host.text_command(TextCommand::Undo, false).unwrap();
    assert_eq!(value.get(), "First\n");
    host.text_command(TextCommand::Redo, false).unwrap();
    assert_eq!(value.get(), "First\nSecond");
    value.set("External".into());
    assert!(!host.text_input().unwrap().unwrap().can_undo);
}
#[test]
fn secure_fields_prevent_clipboard_export() {
    let value = State::new("secret 😀".to_owned());
    let host = Host::new(move || view! {password_field("Password",bind:value);});
    let id = host.snapshot().unwrap().last().unwrap().id;
    host.focus(id).unwrap();
    host.text_command(TextCommand::SelectAll, false).unwrap();
    assert!(host.text_input().unwrap().unwrap().secure);
    assert_eq!(host.selected_text().unwrap(), None);
}
#[test]
fn numeric_drafts_keep_invalid_edits_out_of_exact_values() {
    let value = State::new(9_007_199_254_740_993i64);
    let numeric = Numeric::new(value.binding(), i64::MIN, i64::MAX, 1);
    let edit = numeric.draft.binding();
    edit.set("-".into());
    assert!(!numeric.draft.commit());
    assert_eq!(value.get(), 9_007_199_254_740_993);
    edit.set("9007199254740994".into());
    assert!(numeric.draft.commit());
    numeric.step(true);
    assert_eq!(value.get(), 9_007_199_254_740_995);
    value.set(25);
    assert_eq!(edit.get(), "25");
    assert!(numeric.draft.error().is_none());
    let float = Numeric::new(State::new(1.).binding(), 0., 10., 0.5);
    float.draft.binding().set("NaN".into());
    assert!(!float.draft.commit());
}
#[test]
fn deadlines_replace_cancel_and_release_borrows_before_callbacks() {
    use std::{
        cell::Cell,
        rc::Rc,
        time::{Duration, Instant},
    };
    let host = Host::new(|| view! {label("Search");});
    let value = Rc::new(Cell::new(0));
    host.debounce("search", Duration::ZERO, || panic!("obsolete"));
    let changed = value.clone();
    host.debounce("search", Duration::ZERO, move || changed.set(1));
    host.debounce("cancel", Duration::ZERO, || panic!("cancelled"));
    host.cancel_deadline("cancel");
    host.run_due_at(Instant::now() + Duration::from_secs(1));
    assert_eq!(value.get(), 1);
    assert!(host.next_deadline().is_none());
}

#[test]
fn cancelling_ime_preserves_redo_and_empty_deletion_does_not_record_history() {
    let value = State::new(String::new());
    let host = Host::new(move || view! {text_field("Name",bind:value);});
    let id = host.snapshot().unwrap().last().unwrap().id;
    host.focus(id).unwrap();
    host.text_command(TextCommand::Backspace, false).unwrap();
    assert!(!host.text_input().unwrap().unwrap().can_undo);
    host.insert_text("First", None).unwrap();
    host.text_command(TextCommand::Undo, false).unwrap();
    host.mark_text("日", 3..3, None).unwrap();
    host.unmark_text(false).unwrap();
    assert!(host.text_input().unwrap().unwrap().can_redo);
    host.text_command(TextCommand::Redo, false).unwrap();
    assert_eq!(host.text_input().unwrap().unwrap().text, "First");
    host.mark_text("本", 3..3, None).unwrap();
    host.unmark_text(true).unwrap();
    host.text_command(TextCommand::Undo, false).unwrap();
    assert_eq!(host.text_input().unwrap().unwrap().text, "First");
}
