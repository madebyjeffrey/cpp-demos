//! Small component catalog; use --light or resize the native window for palette/layout checks.
mod common;
use scars_window::ui::*;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let checked = State::new(CheckState::Indeterminate);
    let enabled = State::new(true);
    let text = State::new("Portable Rust controls".to_owned());
    let level = State::new(0.5);
    let selected = State::new("a".to_owned());
    common::run("SCARS · Component lab", (820, 720), move || {
        view! {
            column(padding:28,gap:20){
                label("Component lab",font_size:30,bold:true,shrink:0);
                switch("Enable controls",bind:enabled,shrink:0);
                scroll(grow:1){
                    panel(padding:20,gap:14,enabled:enabled.get()){
                        text_field("Text",bind:text);tri_checkbox("Three-state checkbox",bind:checked);
                        segmented(bind:selected,options:vec![OptionItem::new("a","Alpha"),OptionItem::new("b","Beta"),OptionItem::new("c","Gamma")]);
                        slider(bind:level,min:0,max:1,step:0.01);progress(bind:level);
                        row{button("Primary",on_click:||{});button("Disabled",enabled:false,on_click:||{});}
                        inline(gap:8){label("Wrapping inline layout:");for word in ["reactive","portable","declarative","native IME"]{button(word,on_click:||{});}}
                        text_editor("Notes",bind:text,height:150);
                    }
                }
                label("Use the titlebar theme button, or launch with --light. Resize to check constraints.",wrap:true,shrink:0);
            }
        }
    })
}
