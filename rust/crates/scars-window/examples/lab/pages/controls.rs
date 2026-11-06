use super::*;
pub fn view(m: &Lab) -> View {
    let choices = vec![
        OptionItem::new("a", "Alpha"),
        OptionItem::new("b", "Beta"),
        OptionItem {
            id: "c".into(),
            label: "Unavailable".into(),
            enabled: false,
        },
    ];
    grid(vec![
        card(
            m,
            "actions",
            "Buttons and progress",
            view! {column(gap:12){
                row(wrap:true){
                    button("Run",on_click:m.status.action(|s|s.set("Action ran".into())),key:"run");
                    command_button("Command",on_click:m.status.action(|s|s.set("Command ran".into())));
                    button("Disabled",enabled:false,on_click:||{});
                }
                toggle("Toggle button",bind:m.checked);
                progress(bind:m.level,accessible_label:"Progress");
                progress(accessible_label:"Indeterminate progress");
            }},
        ),
        card(
            m,
            "choices",
            "Bound choices",
            view! {column(gap:12){
                checkbox("Checkbox",bind:m.checked);
                tri_checkbox("Three states",bind:m.tri);
                switch("Switch",bind:m.checked);
                radio_group("Choose one",bind:m.choice,options:choices.clone());
                segmented(bind:m.choice,options:choices);
            }},
        ),
        card(
            m,
            "slider",
            "Slider",
            view! {column(gap:12){
                labeled("Level · arrows 0.05, pages 0.25",View::slider(m.level.binding(),0.,1.,0.05).page_step(0.25).read_only(m.readonly.get()).key("level"));
                label(format!("{:.0}%",m.level.get()*100.));
                label("Home/End reach bounds; Page Up/Down use an independent step.",wrap:true);
                slider(bind:m.level,min:0,max:1,step:0.05,enabled:false,accessible_label:"Disabled level");
            }},
        ),
        card(
            m,
            "custom",
            "Stateful custom components",
            view! {column(gap:16){
                counter("Downloads",&m.counters[0],key:"downloads");
                separator();
                counter("Uploads",&m.counters[1],key:"uploads");
            }},
        ),
    ])
}
