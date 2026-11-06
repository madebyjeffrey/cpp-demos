//! Shared color drafts, retained hue intent and explicit live-preview rollback.
mod common;
use scars_window::ui::*;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let value = State::new(Color::rgba(68, 136, 204, 128));
    let live_value = State::new(Color::rgba(204, 102, 68, 180));
    let readonly = State::new(false);
    let alpha = State::new(true);
    let picker = ColorPicker::new(value.binding());
    let live = ColorPicker::new(live_value.binding())
        .live_preview(true)
        .model(ColorModel::Hsv);
    let popup = Popup::new();
    if std::env::args().any(|arg| arg == "--well") {
        popup.show();
    }
    let well = ColorPicker::new(value.binding());
    common::run("SCARS · Color policies", (1040, 820), move || {
        let (swatch, popover) = well
            .clone()
            .alpha(alpha.get())
            .read_only(readonly.get())
            .well(&popup, "paint");
        view! {
            layer {
                scroll { column(padding:24,gap:16) {
                    label("Color policies",label_style:LabelStyle::Heading);
                    label("RGB/HSV share one draft. Apply commits; Cancel discards. Live preview publishes valid edits and rolls back only values it still owns. Alpha is straight sRGB; gray/black retain hue intent.",wrap:true);
                    row {
                        checkbox("Read only",bind:readonly);
                        checkbox("Alpha channel",bind:alpha);
                        embed(swatch);
                        label(format!("Applied: {}",value.get()));
                    }
                    row(gap:24) {
                        panel(grow:1,basis:0,min_width:0) {
                            label("Transactional",bold:true);
                            embed(picker.clone().alpha(alpha.get()).read_only(readonly.get()).view());
                        }
                        panel(grow:1,basis:0,min_width:0) {
                            label("Live preview",bold:true);
                            embed(live.clone().alpha(alpha.get()).read_only(readonly.get()).view());
                            label(format!("Application sees: {}",live_value.get()),wrap:true);
                        }
                    }
                    button("External update",on_click:value.action(|v|v.set(Color::rgba(90,190,125,96))));
                } }
                embed(popover);
            }
        }
    })
}
