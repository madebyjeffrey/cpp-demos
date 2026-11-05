//! No window, native backend or renderer: `cargo run -p scars-ui --example text_buffer`.
use scars_ui::{Selection, TextBuffer};
fn main() {
    let mut text = TextBuffer::new("Hello Rust");
    text.select(Selection {
        anchor: 6,
        caret: 10,
    });
    text.mark("日本", 6..6, None);
    println!(
        "Committed: {} · composing: {}",
        text.committed_text(),
        text.text()
    );
    text.unmark(true);
    println!("Accepted: {}", text.text());
    text.undo();
    println!("Undone: {}", text.text());
}
