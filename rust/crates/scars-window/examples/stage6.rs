//! Typed composite values and inspection of real mounted controls.
mod common;
use scars_window::ui::*;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let color = State::new(Color::rgba(101, 176, 255, 255));
    let picker = ColorPicker::new(color.binding());
    let date = State::new("2026-11-04".parse::<Date>()?);
    let calendar = DatePicker::new(date.binding());
    let time = State::new("14:30:00".parse::<Time>()?);
    let time_editor = time_draft(time.binding());
    let local = State::new("2026-11-04T14:30:00".parse::<DateTime>()?);
    let local_editor = datetime_draft(local.binding());
    let zoned =
        State::new(resolve_zoned(local.get(), "America/Edmonton", Resolution::Reject)?.timestamp());
    let zone_editor = ZonedPicker::new(zoned.binding(), "America/Edmonton")?;
    let inspector = Inspector::new();
    let inspect = inspector.clone();
    common::run_inspected(
        "Stage 6 · Value lab",
        (1100, 840),
        Some(inspector),
        move || {
            view! {
                column(padding:24,gap:20){
                    label("The value lab",font_size:28,bold:true,shrink:0);
                    label("Typed drafts · explicit time zones · inspectable Rust controls",shrink:0);
                    row(grow:1,gap:20){
                        scroll(width:percent(30.),grow:1,gap:20){
                            section("COLOR",gap:12){embed(picker.view());}
                            separator();embed(value_field("Time · HH:MM:SS",&time_editor));
                            label(format!("Committed time: {}",time.get()));
                        }
                        scroll(width:percent(35.),grow:1,gap:20){
                            section("CALENDAR",gap:12){embed(calendar.view());}
                            separator();embed(value_field("Local date/time · ISO",&local_editor));
                            label(format!("Selected: {}",date.get()));
                        }
                        scroll(width:percent(32.),grow:1,gap:20){
                            embed(inspect.view());
                            section("ZONED TIME",gap:12){embed(zone_editor.view());label(zoned.get().to_string(),wrap:true);}
                        }
                    }
                }
            }
        },
    )
}
