//! Completion M1: nested two-axis scrolling, overlay scrollbars and input reveal.
mod common;
use scars_window::ui::*;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let name = State::new(String::from("Tab to reveal this field"));
    common::run("SCARS · Scrolling", (820, 600), move || {
        view! {
            column(padding:24,gap:16){
                label("Room to move",label_style:LabelStyle::Heading);
                label("Scroll in both directions. Drag either scrollbar. The nested list passes unused wheel motion outward.",wrap:true,label_style:LabelStyle::Secondary);
                scroll(grow:1,scroll_axes:ScrollAxes::Both,gap:20){
                    row(width:1100,gap:24,padding:20,align:Align::Start){
                        panel(width:320,padding:16,gap:12){
                            label("Nested vertical list",label_style:LabelStyle::Section);
                            scroll(height:260,gap:6){for i in 1..=30{button(format!("Item {i}"),key:format!("{i}"),on_click:||{});}}
                        }
                        panel(width:360,padding:20,gap:12){
                            label("Across the canvas",label_style:LabelStyle::Heading,wrap:true);
                            label("An ordinary portable component tree can be wider than its viewport.",wrap:true);
                            text_field("Name",bind:name,key:"far-field");
                        }
                        panel(width:300,height:300,padding:20){label("The far edge",label_style:LabelStyle::Heading);}
                    }
                    column(width:1100,padding:24,gap:14){
                        for i in 1..=8{label(format!("Canvas row {i}"),height:40);}
                        button("Bottom of the canvas",key:"bottom",on_click:||{});
                    }
                }
                label("Default scroll remains vertical; choose ScrollAxes::Horizontal or Both when needed.",label_style:LabelStyle::Micro,wrap:true);
            }
        }
    })
}
