//! Configurable popovers, nested return focus, tooltips and attached context menus.
mod common;
use scars_window::ui::*;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let status =
        State::new("Open the details popover or secondary-click the document card.".to_owned());
    let popup = Popup::new()
        .on_dismiss(status.handler(|s, reason| s.set(format!("Details closed: {reason:?}"))));
    let nested = Popup::new().options(OverlayOptions {
        placement: OverlayPlacement::Right,
        width: Some(220.),
        margin: 8.,
        ..Default::default()
    });
    let context = Menu::new().options(OverlayOptions {
        margin: 8.,
        ..Default::default()
    });
    let placement = State::new("below".to_owned());
    let preserve = State::new(false);
    let note = State::new("A retained draft".to_owned());
    if std::env::args().any(|a| a == "--render") {
        popup.show();
    }
    common::run("SCARS · Overlay policies", (900, 640), move || {
        let placement_value = match placement.get().as_str() {
            "above" => OverlayPlacement::Above,
            "left" => OverlayPlacement::Left,
            "right" => OverlayPlacement::Right,
            _ => OverlayPlacement::Below,
        };
        let details = popup.clone().options(OverlayOptions {
            placement: placement_value,
            focus: if preserve.get() {
                OverlayFocus::Preserve
            } else {
                OverlayFocus::Content
            },
            width: Some(340.),
            max_height: 290.,
            margin: 12.,
            gap: 10.,
            ..Default::default()
        });
        let body = view! {column(padding:32,gap:20){
            label("Overlays with a return address",label_style:LabelStyle::Heading);
            label("Position, focus and dismissal are independent policies.",label_style:LabelStyle::Secondary);
            segmented(bind:placement,options:vec![OptionItem::new("below","Below"),OptionItem::new("above","Above"),OptionItem::new("left","Left"),OptionItem::new("right","Right")]);
            checkbox("Keep the invoking control focused",bind:preserve);
            row{embed(details.button("Details…","details").tooltip("Open the details. Try nested popovers, a short scroll area, or changing placement.").tooltip_options(TooltipOptions{delay:std::time::Duration::from_millis(350),max_width:230.,..Default::default()}));}
            embed(View::panel(vec![View::label("Document card").bold(true),View::label("Secondary-click or Control-click anywhere here. Focus the card action and use Shift-F10." ).wrap(true),View::button("Card action",status.action(|s|s.set("Ordinary card action".into())))]).padding(20).context_menu(&context,vec![MenuAction::new("Duplicate",status.action(|s|s.set("Duplicated document".into()))),MenuAction::submenu("Share",vec![MenuAction::new("Copy link",status.action(|s|s.set("Copied link".into())))]) ]));
            spacer(grow:1);
            label(status.get(),wrap:true,label_style:LabelStyle::Status);
        }};
        let content = view! {panel(padding:18,gap:12){
            label("Document details",label_style:LabelStyle::Section);
            text_field("Notes",bind:note,key:"notes");
            embed(nested.button("More options…","nested"));
            scroll(height:110,gap:6){for i in 1..=8{button(format!("Detail {i}"),on_click:||{},shrink:0);}}
            label("Escape closes the top overlay. Marked text cancels first.",wrap:true,label_style:LabelStyle::Micro);
        }};
        View::layer(vec![body,details.popover("details",content),nested.popover("nested",view!{panel(padding:18){label("A nested scope",bold:true);button("Inside",on_click:||{});label("Escape returns focus to More options.",wrap:true);}})])
    })
}
