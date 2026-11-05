//! Editing and typed drafts. Enter commits numeric edits; +/- steps exact values.
mod common;
use scars_window::ui::*;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let search = State::new(String::new());
    let password = State::new("correct horse".to_owned());
    let notes=State::new("A native IME, a portable editor.\nTry emoji, accents, or Japanese.\nCommand-Z undoes an edit.".to_owned());
    let count = State::new(42i64);
    let number = Numeric::new(count.binding(), 0, 1000, 1);
    common::run("Stage 3 · Editing", (650, 650), move || {
        view! {
            column(padding:28,gap:18) {
                label("Words & values",font_size:28,bold:true);
                search_field("Search notes…",bind:search,accessible_label:"Search notes");
                label(format!("{} matches",if search.get().is_empty() { 0 } else { notes.get().matches(&search.get()).count() }));
                section("PRIVATE",gap:10) { password_field("Password",bind:password); }
                section("NOTES",gap:10,grow:1) {text_editor("Write something…",bind:notes,grow:1);}
                numeric_field("Copies · ASCII integer",&number);
                label(format!("Committed: {} · Enter applies a valid draft",count.get()));
            }
        }
    })
}
