use scars_ui::*;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
struct Metrics;
impl Measure for Metrics {
    fn text(&self, s: &str, _: TextStyle, _: f64) -> TextLayout {
        TextLayout {
            size: Size::new(s.len() as f64 * 7., 18.),
            baseline: 14.,
            lines: vec![],
            placeholders: vec![],
            hit_test: Rc::new(|_| 0),
            range_rects: Rc::new(|_| vec![]),
        }
    }
    fn image(&self, _: &Bitmap) -> Result<Size, Error> {
        Ok(Size::default())
    }
}
fn layout(h: &Host) {
    h.layout(Size::new(680., 700.), &Metrics).unwrap();
}
fn field(h: &Host, label: &str) -> Node {
    h.snapshot()
        .unwrap()
        .into_iter()
        .find(|n| n.kind == Kind::TextField && n.label == label)
        .unwrap()
}
fn key(h: &Host, key: Key) {
    h.key(KeyEvent {
        key,
        down: true,
        repeat: false,
        modifiers: Default::default(),
    })
    .unwrap();
}

#[test]
fn hsv_conversion_round_trips_encoded_rgb_and_rejects_invalid_coordinates() {
    for r in [0, 1, 12, 64, 127, 128, 200, 254, 255] {
        for g in [0, 1, 12, 64, 127, 128, 200, 254, 255] {
            for b in [0, 1, 12, 64, 127, 128, 200, 254, 255] {
                let color = Color::rgba(r, g, b, 43);
                assert_eq!(Color::from_hsv(color.hsv(), 43).unwrap(), color);
            }
        }
    }
    assert_eq!(
        Color::from_hsv(Hsv::new(360., 1., 1.).unwrap(), 0).unwrap(),
        Color::rgba(255, 0, 0, 0)
    );
    for hsv in [
        Hsv {
            hue: f64::NAN,
            saturation: 0.,
            value: 0.,
        },
        Hsv {
            hue: 361.,
            saturation: 1.,
            value: 1.,
        },
        Hsv {
            hue: 120.,
            saturation: -0.1,
            value: 1.,
        },
        Hsv {
            hue: 120.,
            saturation: 1.,
            value: f64::INFINITY,
        },
    ] {
        assert!(Color::from_hsv(hsv, 255).is_err());
    }
}

#[test]
fn transaction_shares_rgb_hsv_and_hex_and_publishes_once_on_apply() {
    let source = State::new(Color::rgba(10, 20, 30, 40));
    let writes = Rc::new(Cell::new(0));
    let (read, write, count) = (source.clone(), source.clone(), writes.clone());
    let p = ColorPicker::new(Binding::new(
        move || read.get(),
        move |v| {
            count.set(count.get() + 1);
            write.set(v)
        },
    ));
    assert!(p.set_component(ColorChannel::Red, 100.));
    assert!(p.set_component(ColorChannel::Green, 25.));
    assert_eq!(p.binding().get(), "#64191E28");
    assert_eq!(source.get(), Color::rgba(10, 20, 30, 40));
    assert_eq!(writes.get(), 0);
    assert!(p.set_model(ColorModel::Hsv));
    assert_eq!(p.info().draft, Color::rgba(100, 25, 30, 40));
    assert!(p.commit());
    assert_eq!(source.get(), Color::rgba(100, 25, 30, 40));
    assert_eq!(writes.get(), 1);
    assert!(!p.validation().dirty);
    assert!(p.commit());
    assert_eq!(
        writes.get(),
        1,
        "an untouched Apply does not repeat a setter"
    );
    p.binding().set("#01020380".into());
    assert_eq!(p.info().draft, Color::rgba(1, 2, 3, 128));
    p.cancel();
    assert_eq!(p.info().draft, source.get());
    assert_eq!(writes.get(), 1);
}

#[test]
fn achromatic_edits_keep_hue_and_saturation_intent_and_hidden_alpha() {
    let source = State::new(Color::rgba(128, 128, 128, 77));
    let p = ColorPicker::new(source.binding())
        .model(ColorModel::Hsv)
        .alpha(false);
    assert!(p.set_component(ColorChannel::Hue, 211.123456));
    assert_eq!(p.component(ColorChannel::Hue), 211.123456);
    assert!(p.set_component(ColorChannel::Saturation, 75.));
    let intended = p.info().draft;
    assert!(p.set_component(ColorChannel::Value, 0.));
    assert_eq!(p.info().draft, Color::rgba(0, 0, 0, 77));
    assert_eq!(p.component(ColorChannel::Hue), 211.123456);
    assert_eq!(p.component(ColorChannel::Saturation), 75.);
    assert!(p.set_component(ColorChannel::Value, 128. / 255. * 100.));
    assert_eq!(p.info().draft, intended);
    assert!(p.set_model(ColorModel::Rgb));
    assert!(p.set_model(ColorModel::Hsv));
    assert_eq!(p.info().draft, intended);
    p.binding().set("#123456".into());
    assert_eq!(p.info().draft, Color::rgba(18, 52, 86, 77));
    assert!(p.commit());
    assert_eq!(source.get().channels()[3], 77);
    p.binding().set("#123456FF".into());
    assert_eq!(p.validation().status, ValidationStatus::Invalid);
    assert!(!p.commit());
    assert!(!p.set_component(ColorChannel::Alpha, 255.));
    assert_eq!(source.get().channels()[3], 77);
}

#[test]
fn editing_one_hsv_field_preserves_the_exact_untouched_components_behind_rounded_display() {
    let source = State::new(Color::rgba(128, 128, 128, 77));
    let p = ColorPicker::new(source.binding()).model(ColorModel::Hsv);
    p.set_component(ColorChannel::Hue, 211.123456);
    p.set_component(ColorChannel::Saturation, 73.123456);
    let host = Host::new({
        let p = p.clone();
        move || p.view()
    });
    layout(&host);
    let value = field(&host, "Value percentage");
    host.set_text(value.id, "80").unwrap();
    assert_eq!(p.component(ColorChannel::Hue), 211.123456);
    assert_eq!(p.component(ColorChannel::Saturation), 73.123456);
    assert!(p.commit());
    assert_eq!(
        source.get(),
        Color::from_hsv(Hsv::new(211.123456, 73.123456 / 100., 0.8).unwrap(), 77).unwrap()
    );
}

#[test]
fn invalid_component_text_blocks_other_representations_and_remains_correctable() {
    let source = State::new(Color::rgba(10, 20, 30, 40));
    let p = ColorPicker::new(source.binding());
    let host = Host::new({
        let p = p.clone();
        move || p.view()
    });
    layout(&host);
    let red = field(&host, "Red value");
    host.focus(red.id).unwrap();
    host.set_text(red.id, "256").unwrap();
    layout(&host);
    assert_eq!(p.validation().status, ValidationStatus::OutOfRange);
    assert!(!p.commit());
    assert!(!p.set_model(ColorModel::Hsv));
    assert!(!p.set_component(ColorChannel::Blue, 50.));
    assert!(field(&host, "Hex color").read_only);
    assert_eq!(field(&host, "Red value").value, Value::Text("256".into()));
    host.set_text(red.id, "").unwrap();
    assert_eq!(p.validation().status, ValidationStatus::Incomplete);
    host.set_text(red.id, "12.5").unwrap();
    assert_eq!(p.validation().status, ValidationStatus::Invalid);
    host.set_text(red.id, "12").unwrap();
    assert_eq!(p.info().draft, Color::rgba(12, 20, 30, 40));
    assert_eq!(source.get(), Color::rgba(10, 20, 30, 40));
    let hex = field(&host, "Hex color");
    host.focus(hex.id).unwrap();
    host.set_text(hex.id, "#abc").unwrap();
    layout(&host);
    assert_eq!(p.validation().status, ValidationStatus::Invalid);
    assert!(field(&host, "Red value").read_only);
    host.set_text(hex.id, "#FF0000").unwrap();
    key(&host, Key::Enter);
    assert_eq!(source.get(), Color::rgba(255, 0, 0, 255));
}

#[test]
fn live_preview_rolls_back_only_owned_values_and_apply_finalizes_them() {
    let initial = Color::rgba(10, 20, 30, 40);
    let source = State::new(initial);
    let p = ColorPicker::new(source.binding()).live_preview(true);
    assert!(p.set_component(ColorChannel::Red, 100.));
    assert_eq!(source.get(), Color::rgba(100, 20, 30, 40));
    assert_eq!(p.info().original, initial);
    p.cancel();
    assert_eq!(source.get(), initial);
    p.binding().set("#01020304".into());
    assert_eq!(source.get(), Color::rgba(1, 2, 3, 4));
    assert!(p.commit());
    p.cancel();
    assert_eq!(source.get(), Color::rgba(1, 2, 3, 4));
    assert!(p.set_component(ColorChannel::Blue, 200.));
    let preview = source.get();
    source.set(initial);
    source.set(preview); // equal value with a newer accepted-value revision
    p.cancel();
    assert_eq!(
        source.get(),
        preview,
        "external ABA drops rollback ownership"
    );
    assert!(p.set_component(ColorChannel::Blue, 201.));
    source.set(initial);
    p.cancel();
    assert_eq!(source.get(), initial);
    {
        let p = ColorPicker::new(source.binding()).live_preview(true);
        p.set_component(ColorChannel::Red, 250.);
    }
    assert_eq!(
        source.get(),
        initial,
        "last controller drop requests rollback"
    );
}

#[test]
fn rejected_normalized_and_reentrant_setters_preserve_application_authority() {
    let source = State::new(Color::rgba(10, 20, 30, 40));
    let accept = Rc::new(Cell::new(false));
    let (read, write, flag) = (source.clone(), source.clone(), accept.clone());
    let p = ColorPicker::new(Binding::new(
        move || read.get(),
        move |v| {
            if flag.get() {
                write.set(v)
            }
        },
    ));
    p.binding().set("#AABBCCDD".into());
    assert!(!p.commit());
    assert_eq!(p.validation().status, ValidationStatus::Invalid);
    assert_eq!(p.binding().get(), "#AABBCCDD");
    accept.set(true);
    assert!(p.commit());
    assert_eq!(source.get(), Color::rgba(170, 187, 204, 221));
    let (read, write) = (source.clone(), source.clone());
    let p = ColorPicker::new(Binding::new(
        move || read.get(),
        move |v: Color| {
            let [r, g, b, _] = v.channels();
            write.set(Color::rgba(r, g, b, 255));
        },
    ))
    .live_preview(true);
    p.binding().set("#01020304".into());
    assert_eq!(source.get(), Color::rgba(1, 2, 3, 255));
    assert_eq!(p.binding().get(), "#010203FF");
    p.cancel();
    assert_eq!(
        source.get(),
        Color::rgba(170, 187, 204, 255),
        "rollback reads back normalization"
    );
    let slot: Rc<RefCell<Option<ColorPicker>>> = Rc::new(RefCell::new(None));
    let once = Rc::new(Cell::new(false));
    let (read, write, other, flag) = (source.clone(), source.clone(), slot.clone(), once.clone());
    let p = ColorPicker::new(Binding::new(
        move || read.get(),
        move |v| {
            write.set(v);
            if !flag.replace(true) {
                other
                    .borrow()
                    .as_ref()
                    .unwrap()
                    .binding()
                    .set("#FFEEDDCC".into());
            }
        },
    ))
    .live_preview(true);
    *slot.borrow_mut() = Some(p.clone());
    p.set_component(ColorChannel::Red, 42.);
    assert_eq!(source.get(), Color::rgba(255, 238, 221, 204));
    assert_eq!(p.info().draft, source.get());
    slot.borrow_mut().take();
    p.commit();
}

#[test]
fn marked_text_is_confirmed_before_apply_and_readonly_preserves_selection() {
    let source = State::new(Color::rgba(10, 20, 30, 40));
    let p = ColorPicker::new(source.binding());
    let host = Host::new({
        let p = p.clone();
        move || p.view()
    });
    layout(&host);
    let hex = field(&host, "Hex color");
    host.focus(hex.id).unwrap();
    host.set_text(hex.id, "#AABBCC").unwrap();
    host.mark_text("DD", 2..2, Some(7..7)).unwrap();
    assert!(!host.apply_color(hex.id).unwrap());
    assert_eq!(source.get(), Color::rgba(10, 20, 30, 40));
    key(&host, Key::Enter);
    assert_eq!(source.get(), Color::rgba(10, 20, 30, 40));
    key(&host, Key::Enter);
    assert_eq!(source.get(), Color::rgba(170, 187, 204, 221));
    let p = p.read_only(true);
    let host = Host::new(move || p.view());
    layout(&host);
    let hex = field(&host, "Hex color");
    assert!(host.focus(hex.id).unwrap());
    assert!(
        host.select_text(
            hex.id,
            Selection {
                anchor: 0,
                caret: 3
            }
        )
        .unwrap()
    );
    assert!(!host.set_text(hex.id, "#01020304").unwrap());
    assert!(!host.apply_color(hex.id).unwrap());
    assert_eq!(source.get(), Color::rgba(170, 187, 204, 221));
}

#[test]
fn inline_lifetimes_and_well_dismissal_retire_unfinished_previews() {
    let initial = Color::rgba(10, 20, 30, 40);
    for change in 0..4 {
        let source = State::new(initial);
        let show = State::new(true);
        let enabled = State::new(true);
        let mounted = State::new(true);
        let p = ColorPicker::new(source.binding()).live_preview(true);
        let host = Host::new({
            let p = p.clone();
            let (s, e, m) = (show.clone(), enabled.clone(), mounted.clone());
            move || {
                if m.get() {
                    p.view().visible(s.get()).enabled(e.get())
                } else {
                    View::label("Gone")
                }
            }
        });
        layout(&host);
        p.set_component(ColorChannel::Red, 100.);
        match change {
            0 => show.set(false),
            1 => enabled.set(false),
            2 => mounted.set(false),
            _ => host.set_active(false),
        }
        layout(&host);
        assert_eq!(source.get(), initial);
    }
    let source = State::new(initial);
    let p = ColorPicker::new(source.binding()).live_preview(true);
    {
        let host = Host::new({
            let p = p.clone();
            move || p.view()
        });
        layout(&host);
        p.set_component(ColorChannel::Blue, 200.);
    }
    assert_eq!(
        source.get(),
        initial,
        "host drop rolls back despite a retained controller"
    );
    let popup = Popup::new();
    let p = ColorPicker::new(source.binding()).live_preview(true);
    let host = Host::new({
        let p = p.clone();
        let popup = popup.clone();
        move || {
            let (well, body) = p.well(&popup, "paint");
            View::layer(vec![well, body])
        }
    });
    layout(&host);
    popup.show();
    layout(&host);
    let apply = host
        .snapshot()
        .unwrap()
        .into_iter()
        .find(|n| n.kind == Kind::Button && n.text == "Apply")
        .unwrap();
    assert_eq!(
        apply.clip.height, apply.bounds.height,
        "a fitted color popover exposes Apply without needless scrolling"
    );
    p.set_component(ColorChannel::Blue, 200.);
    layout(&host);
    key(&host, Key::Escape);
    layout(&host);
    assert!(!popup.open.get());
    assert_eq!(source.get(), initial);
    popup.show();
    layout(&host);
    p.set_component(ColorChannel::Blue, 201.);
    layout(&host);
    let apply = host
        .snapshot()
        .unwrap()
        .into_iter()
        .find(|n| n.kind == Kind::Button && n.text == "Apply")
        .unwrap();
    assert!(host.activate(apply.id).unwrap());
    layout(&host);
    assert!(!popup.open.get());
    assert_eq!(source.get(), Color::rgba(10, 20, 201, 40));
}

#[test]
fn theme_collapse_configuration_and_reopened_popup_generations_keep_coherent_ownership() {
    let initial = Color::rgba(10, 20, 30, 40);
    let source = State::new(initial);
    let p = ColorPicker::new(source.binding()).live_preview(true);
    let collapsed = State::new(SplitPane::None);
    let split = Split::new(0.7).collapsed(collapsed.binding());
    let host = Host::new({
        let p = p.clone();
        move || split.view(p.view(), View::label("Other"))
    });
    layout(&host);
    p.set_component(ColorChannel::Red, 100.);
    host.set_theme(Theme::light());
    layout(&host);
    assert_eq!(
        source.get(),
        Color::rgba(100, 20, 30, 40),
        "theme changes preserve the session"
    );
    collapsed.set(SplitPane::First);
    layout(&host);
    assert_eq!(source.get(), initial);
    p.set_component(ColorChannel::Red, 101.);
    let _readonly = p.clone().read_only(true);
    assert_eq!(
        source.get(),
        initial,
        "configuration retires the old session outside construction"
    );
    let popup = Popup::new();
    let p = ColorPicker::new(source.binding()).live_preview(true);
    let host = Host::new({
        let p = p.clone();
        let popup = popup.clone();
        move || {
            let (w, b) = p.well(&popup, "paint");
            View::layer(vec![w, b])
        }
    });
    popup.show();
    layout(&host);
    p.set_component(ColorChannel::Blue, 200.);
    layout(&host);
    popup.dismiss();
    popup.show();
    layout(&host);
    assert!(popup.open.get());
    assert_eq!(
        source.get(),
        initial,
        "the retired presentation cannot carry a preview into its replacement"
    );
    p.set_component(ColorChannel::Blue, 201.);
    layout(&host);
    assert_eq!(source.get(), Color::rgba(10, 20, 201, 40));
    popup.dismiss();
    layout(&host);
    assert_eq!(source.get(), initial);
}

#[test]
fn readonly_toggle_keeps_the_selected_color_model_field_focus_and_selection() {
    let source = State::new(Color::rgba(20, 100, 180, 80));
    let p = ColorPicker::new(source.binding());
    let readonly = State::new(false);
    let host = Host::new({
        let p = p.clone();
        let r = readonly.clone();
        move || p.clone().read_only(r.get()).view()
    });
    layout(&host);
    p.set_model(ColorModel::Hsv);
    layout(&host);
    let hue = field(&host, "Hue value");
    host.focus(hue.id).unwrap();
    let selection = Selection {
        anchor: 0,
        caret: 2,
    };
    host.select_text(hue.id, selection).unwrap();
    readonly.set(true);
    layout(&host);
    let retained = field(&host, "Hue value");
    assert_eq!(retained.id, hue.id);
    assert!(retained.read_only);
    assert_eq!(host.focused(), Some(hue.id));
    assert_eq!(host.text_input().unwrap().unwrap().selection, selection);
    assert_eq!(source.get(), Color::rgba(20, 100, 180, 80));
}
