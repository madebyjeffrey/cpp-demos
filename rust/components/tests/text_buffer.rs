use scars_ui::*;
use std::time::{Duration, Instant};
#[test]
fn composition_is_transactional_and_undo_preserves_selection_and_redo() {
    let mut buffer = TextBuffer::new("A👨‍👩‍👧‍👦B");
    buffer.select(Selection {
        anchor: 1,
        caret: 26,
    });
    buffer.mark("日本", 3..6, None);
    assert_eq!(buffer.committed_text(), "A👨‍👩‍👧‍👦B");
    assert_eq!(buffer.text(), "A日本B");
    buffer.mark("日本語", 9..9, None);
    buffer.unmark(true);
    assert_eq!(buffer.text(), "A日本語B");
    assert!(buffer.undo());
    assert_eq!(
        buffer.selection(),
        Selection {
            anchor: 1,
            caret: 26
        }
    );
    buffer.mark("仮", 3..3, None);
    buffer.unmark(false);
    assert!(buffer.can_redo());
    assert!(buffer.redo());
    assert_eq!(buffer.text(), "A日本語B");
    buffer.set_text("External");
    assert!(!buffer.can_undo() && buffer.marked_range().is_none());
}
#[test]
fn typing_coalesces_until_time_selection_or_discrete_edit_breaks_the_group() {
    let mut buffer = TextBuffer::default();
    let now = Instant::now();
    buffer.insert_at("a", now);
    buffer.insert_at("b", now + Duration::from_millis(100));
    assert_eq!(buffer.history_len(), (1, 0));
    buffer.insert_at("c", now + Duration::from_secs(2));
    assert_eq!(buffer.history_len(), (2, 0));
    buffer.replace(3..3, "paste");
    assert!(buffer.undo());
    assert_eq!(buffer.text(), "abc");
    assert!(buffer.undo());
    assert_eq!(buffer.text(), "ab");
    buffer.select(Selection {
        anchor: 1,
        caret: 1,
    });
    buffer.insert("!");
    assert!(!buffer.can_redo());
    buffer.set_options(TextOptions {
        history_limit: 2,
        ..Default::default()
    });
    for _ in 0..10 {
        buffer.replace(0..0, "x");
    }
    assert_eq!(buffer.history_len().0, 2);
    buffer.clear_history();
    assert_eq!(buffer.text(), "xxxxxxxxxxa!b");
}
#[test]
fn limits_count_graphemes_and_readonly_or_no_undo_are_explicit_policies() {
    let mut buffer = TextBuffer::new("A");
    buffer.set_options(TextOptions {
        max_length: 2,
        multiline: false,
        ..Default::default()
    });
    buffer.select(Selection {
        anchor: 1,
        caret: 1,
    });
    buffer.insert("e\u{301}👨‍👩‍👧‍👦\n");
    assert_eq!(buffer.text(), "Ae\u{301}");
    buffer.select(Selection {
        anchor: 1,
        caret: 4,
    });
    buffer.mark("😀😀", 8..8, None);
    buffer.unmark(true);
    assert_eq!(buffer.text(), "A😀");
    buffer.set_options(TextOptions {
        read_only: true,
        ..buffer.options()
    });
    assert!(!buffer.insert("x"));
    assert!(!buffer.undo());
    assert!(!buffer.mark("日", 3..3, None));
    buffer.command(TextCommand::SelectAll, false);
    assert_eq!(buffer.selection().range(), 0..5);
    buffer.set_options(TextOptions {
        read_only: false,
        undo: false,
        ..buffer.options()
    });
    buffer.insert("Z");
    assert!(!buffer.can_undo());
    assert_eq!(buffer.text(), "Z");
    // A combining mark may join an existing grapheme even when the limit is full.
    buffer.select(Selection {
        anchor: 1,
        caret: 1,
    });
    buffer.insert("\u{301}");
    assert_eq!(buffer.text(), "Z\u{301}");
}
#[test]
fn malformed_ranges_snap_outward_and_word_line_commands_keep_unicode_whole() {
    let mut buffer = TextBuffer::new("A😀B\nsecond line");
    buffer.replace(2..4, "!");
    assert_eq!(buffer.text(), "A!B\nsecond line");
    buffer.command(TextCommand::End, false);
    buffer.command(TextCommand::DeleteWordBackward, false);
    assert_eq!(buffer.text(), "A!B\nsecond ");
    buffer.command(TextCommand::LineStart, true);
    assert_eq!(buffer.selection().range(), 4..11);
    buffer.command(TextCommand::Delete, false);
    assert_eq!(buffer.text(), "A!B\n");
    buffer.set_text("a😀");
    buffer.select(Selection {
        anchor: 5,
        caret: 5,
    });
    buffer.command(TextCommand::Transpose, false);
    assert_eq!(buffer.text(), "😀a");
    assert!(text::utf16_offset(buffer.text(), 4).is_some());
    assert!(text::byte_offset(buffer.text(), 1).is_none());
}
#[test]
fn mounted_fields_apply_the_same_limits_readonly_and_history_options() {
    let value = State::new(String::new());
    let readonly = State::new(false);
    let host = Host::new({
        let value = value.clone();
        let readonly = readonly.clone();
        move || view! {text_field("",bind:value,max_length:2,read_only:readonly.get(),history_limit:2);}
    });
    let id = host.snapshot().unwrap().last().unwrap().id;
    host.focus(id).unwrap();
    host.paste_text("😀e\u{301}extra").unwrap();
    assert_eq!(value.get(), "😀e\u{301}");
    host.text_command(TextCommand::Undo, false).unwrap();
    assert_eq!(value.get(), "");
    host.mark_text("日本語", 9..9, None).unwrap();
    host.unmark_text(true).unwrap();
    assert_eq!(value.get(), "日本");
    readonly.set(true);
    assert!(!host.insert_text("x", None).unwrap());
    assert!(!host.text_command(TextCommand::Undo, false).unwrap());
    host.select_text(
        id,
        Selection {
            anchor: 0,
            caret: 6,
        },
    )
    .unwrap();
    assert_eq!(host.selected_text().unwrap().unwrap(), "日本");
}
