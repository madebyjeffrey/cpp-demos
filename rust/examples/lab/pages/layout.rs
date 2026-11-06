use super::*;
pub fn view(m: &Lab) -> View {
    let image_modes = view! {row(wrap:true,gap:12){
        for (title,mode) in [("Fit",ContentMode::Fit),("Fill",ContentMode::Fill),("Stretch",ContentMode::Stretch),("Center",ContentMode::Center)] {
            column(gap:4){label(title,label_style:LabelStyle::Micro);image(m.image.clone(),content_mode:mode,width:100,height:76,radius:8);}
        }
    }};
    grid(vec![
        card(
            m,
            "typography",
            "Typography and rich text",
            view! {column(gap:12){
                label("Heading",label_style:LabelStyle::Heading);
                label("A deliberately long single line ends with an ellipsis when space is constrained.",text_overflow:TextOverflow::Ellipsis,max_lines:1);
                label(m.notes.get(),wrap:true);
                rich_text(vec![Span::new("Styled ").bold(true).into(),"inline ".into(),View::button("control",m.checked.action(|s|s.update(|v| *v = !*v))).into()," in a shaped paragraph.".into()]);
                label("Straße",uppercase:true,letter_spacing:2,text_align:Align::End);
            }},
        ),
        card(
            m,
            "boxes",
            "Layout and grouping",
            view! {column(gap:12){
                section("Section"){label("Section → group → panel",wrap:true);}
                group_box("Group"){panel(padding:[4.,12.,8.,16.]){label("Per-edge padding",wrap:true);}}
                block{label("A block flow");inline(gap:6){for word in ["keyed","wrapping","inline"]{button(word,key:word,on_click:||{});}}}
                row(height:40){label("Leading");spacer();separator(orientation:Orientation::Vertical);label("Trailing");}
                layer(height:64){label("Layered positioning",height:24,position:Position::Absolute,left:8,top:8);label("Relative offset",height:24,align_self:Align::Start,position:Position::Relative,left:8,top:32,label_style:LabelStyle::Secondary);}
            }},
        ),
        card(m, "images", "Image content modes", image_modes),
        card(
            m,
            "styles",
            "Inherited styles and flex",
            view! {column(gap:12){
                panel(density:Density::Compact,accent:0xffa48236,margin:4){
                    label("Inherited accent and density",label_style:LabelStyle::Status,wrap:true);
                    checkbox("Compact control",bind:m.checked);
                }
                row(height:54,gap:8,justify:Justify::SpaceBetween,align:Align::Center){
                    button("Capped",basis:70,grow:1,max_width:120,align_self:Align::End,on_click:||{});
                    button("Flexible",basis:80,grow:2,min_width:0,on_click:||{});
                }
                label("A bounded multiline paragraph uses an explicit font size, line height, wrapping and ellipsis.",font_size:17,bold:true,line_height:22,text_wrap:TextWrap::BreakWord,text_overflow:TextOverflow::Ellipsis,max_lines:2);
            }},
        ),
        card(
            m,
            "scroll",
            "Two-axis scrolling",
            view! {scroll(height:160,scroll_axes:ScrollAxes::Both){
                column(width:560){for i in 0..12 {label(format!("Row {i} · wide content can scroll horizontally as well"));}}
            }},
        ),
    ])
}
