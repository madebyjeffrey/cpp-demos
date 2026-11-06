//! A live mounted outline; application code supplies ordinary controls and state.
mod common;
use scars_window::ui::*;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let inspector = Inspector::new();
    let inspect = inspector.clone();
    let name = State::new("Project Atlas".to_owned());
    let copies = Numeric::new(State::new(3i64).binding(), 0, 10, 1);
    let visible = State::new(true);
    let compact = State::new(false);
    let popup = Popup::new();
    let rows = ListModel::new(
        (0..10_000)
            .map(|i| ListItem::new(format!("r{i}"), format!("Item {i}")))
            .collect(),
        28.,
    );
    if std::env::args().any(|a| a == "--popover") {
        popup.show();
    }
    common::run_inspected(
        "SCARS · Mounted inspector",
        (1120, 900),
        Some(inspector),
        move || {
            let p = popup.clone();
            let close = popup.clone();
            let validation = Validation {
                status: if name.get().is_empty() {
                    ValidationStatus::Incomplete
                } else {
                    ValidationStatus::Valid
                },
                message: name.get().is_empty().then(|| "Enter a project name".into()),
                dirty: name.get().is_empty(),
            };
            view! {layer{
                row(padding:20,gap:20){
                    scroll(grow:1,min_width:0,basis:0){column(gap:14){
                        label("Mounted inspector",label_style:LabelStyle::Heading);
                        label("Browse with arrows or pick a control. Selection updates live; inspect hidden/disabled nodes, validation, focus, popup scope and virtual windows. Picking does not activate the target.",wrap:true);
                        text_field("Project name",bind:name,accessible_label:"Project name",validation:validation,key:"name");
                        embed(numeric_field("Copies",&copies),key:"copies");
                        checkbox("Compact layout",bind:compact,key:"compact");
                        checkbox("Show preview",bind:visible);
                        button("Options",on_click:move||p.show(),key:"options");
                        panel(hidden:!visible.get(),key:"preview"){label("Preview content",enabled:!compact.get());}
                        label("10,000 items; only the viewport and overscan are mounted.",wrap:true);
                        embed(rows.view(),height:200,key:"items");
                    }}
                    embed(inspect.view(),width:400,shrink:0);
                }
                embed(popup.popover("options",View::column(vec![View::label("Options popup"),View::button("Close",move || close.dismiss())])));
            }}
        },
    )
}
