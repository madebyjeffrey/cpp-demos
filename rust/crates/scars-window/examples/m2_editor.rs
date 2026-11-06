//! Completion M2: shared buffer, visual navigation, selection and field policies.
mod common;
use scars_window::ui::*;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let notes=State::new("Wrapped text has visual lines, even without newlines. Move up and down to keep your horizontal caret position; Command-Left/Right moves within a visual line, Command-Up/Down moves through the document.\n\nDouble-click a word or triple-click a line, then drag to extend the selection. Try emoji 👨‍👩‍👧‍👦, accents e\u{301}, and Japanese 日本語.\n\nTyping coalesces until a pause or explicit action. Paste and composition form separate undo steps.".to_owned());
    let short = State::new(String::from("Two 😀"));
    let readonly = State::new(false);
    common::run("SCARS · Text editing", (720, 620), move || {
        view! {
            column(padding:28,gap:16){
                label("A shared editing model",label_style:LabelStyle::Heading);
                label("Wrapped navigation · word/line gestures · composition · bounded history",label_style:LabelStyle::Secondary,wrap:true);
                checkbox("Read only",bind:readonly);
                text_editor("Write something…",bind:notes,key:"notes",read_only:readonly.get(),history_limit:100,grow:1);
                label("A limit of 12 graphemes keeps combined emoji and accents whole.",label_style:LabelStyle::Secondary,wrap:true);
                text_field("Up to twelve graphemes",bind:short,max_length:12,read_only:readonly.get());
                label(format!("{} / 12 graphemes",text::grapheme_boundaries(&short.get()).len().saturating_sub(1)),label_style:LabelStyle::Micro);
            }
        }
    })
}
