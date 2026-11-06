//! Completion M1: styles and constrained layout. Resize; use --light or --render file.png.
mod common;
use scars_window::ui::*;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let enabled = State::new(true);
    common::run("SCARS · Layout and typography", (820, 740), move || {
        view! {
            column(padding:[24.,32.,24.,32.],gap:20){
                label("Layout, with intent",label_style:LabelStyle::Heading,shrink:0);
                label("Per-edge insets · flex limits · inherited styles · shaped text",label_style:LabelStyle::Secondary,shrink:0);
                panel(padding:Insets::symmetric(16.,20.),gap:12,shrink:0){
                    label("Flexible allocation",label_style:LabelStyle::Section);
                    row(gap:12,height:64,align:Align::Start){
                        button("Capped at 180",basis:100,grow:1,max_width:180,on_click:||{});
                        button("Receives remaining space",basis:100,grow:3,align_self:Align::End,on_click:||{});
                    }
                }
                panel(padding:16,shrink:0){
                rich_text(vec!["Styled ".into(),Span::new("text").bold(true).font_size(22.).into(),
                    " and an ".into(),View::button("inline control",enabled.action(|v|v.update(|v|*v = !*v))).key("inline-toggle").into(),
                    " share the same paragraph. Resize to see the sentence reflow around the control.".into(),
                ]);
            }
            row(gap:20,grow:1){
                    panel(width:percent(50.),padding:[16.,20.,20.,16.],gap:14){
                        label("Typography",label_style:LabelStyle::Section);
                        label("This line is intentionally much longer than the panel and ends with an ellipsis.",text_overflow:TextOverflow::Ellipsis,max_lines:1);
                        label("Two lines of shaped text stay within their assigned space, even when the window gets narrower. The rest is ellipsized.",text_wrap:TextWrap::Wrap,text_overflow:TextOverflow::Ellipsis,max_lines:2,line_height:24);
                        label("Right aligned",text_align:Align::End);
                        label("Straße",uppercase:true,letter_spacing:2);
                        spacer();
                        label("A relative offset keeps its flow slot",position:Position::Relative,left:8,top:2,label_style:LabelStyle::Secondary,text_wrap:TextWrap::Wrap);
                    }
                    panel(grow:1,padding:20,gap:14,accent:0xffe9a34b,density:Density::Compact){
                        label("Inherited accent & density",label_style:LabelStyle::Section);
                        checkbox("Compact control",bind:enabled);
                        switch("Shares the same state",bind:enabled);
                        row(height:42){label("First");separator(orientation:Orientation::Vertical);label("Second");}
                        label("Status uses the inherited accent",label_style:LabelStyle::Status,text_wrap:TextWrap::Wrap);
                        spacer();
                        row(justify:Justify::SpaceAround){button("A",on_click:||{});button("B",on_click:||{});}
                    }
                }
                label("Styled paragraphs and reflow share the portable core. See m1_scroll for nested scrolling.",label_style:LabelStyle::Micro,wrap:true,shrink:0);
            }
        }
    })
}
