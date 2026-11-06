use scars_ui::*;
use std::rc::Rc;
struct Metrics;
impl Measure for Metrics {
    fn text(&self, text: &str, style: TextStyle, _: f64) -> TextLayout {
        TextLayout {
            lines: Vec::new(),
            placeholders: Vec::new(),
            size: Size::new(text.chars().count() as f64 * 8., style.size + 4.),
            baseline: style.size,
            hit_test: Rc::new(|_| 0),
            range_rects: Rc::new(|_| vec![Rect::new(0., 0., 1., 19.)]),
        }
    }
    fn image(&self, _: &Bitmap) -> Result<Size, Error> {
        Ok(Size::new(10., 10.))
    }
}
fn key(host: &Host, key: Key) {
    host.key(KeyEvent {
        key,
        down: true,
        repeat: false,
        modifiers: Modifiers::default(),
    })
    .unwrap();
}
fn node(host: &Host, kind: Kind) -> Node {
    host.snapshot()
        .unwrap()
        .into_iter()
        .find(|n| n.kind == kind)
        .unwrap()
}
#[test]
fn layout_allocates_grow_and_absolute_and_obeys_hit_clipping() {
    let host = Host::new(|| {
        view! { column(padding: 10, gap: 0) {
            button("Top", height: 30, on_click: || {}); spacer(grow: 1);
            button("Bottom", height: 30, key: "bottom", on_click: || {});
            button("Overlay", position: Position::Absolute, left: 20, top: 40, width: 80, height: 30, on_click: || {});
        } }
    });
    host.layout(Size::new(300., 200.), &Metrics).unwrap();
    let nodes = host.snapshot().unwrap();
    assert_eq!(
        nodes
            .iter()
            .find(|n| n.key.as_deref() == Some("bottom"))
            .unwrap()
            .bounds
            .y,
        160.
    );
    let overlay = nodes.iter().find(|n| n.text == "Overlay").unwrap();
    assert_eq!(overlay.bounds, Rect::new(30., 50., 80., 30.));
    assert_eq!(
        host.hit_test(Point { x: 40., y: 60. }).unwrap(),
        Some(overlay.id)
    );
    assert_eq!(host.hit_test(Point { x: 301., y: 60. }).unwrap(), None);
}
#[test]
fn keyboard_bindings_snap_and_skip_disabled_controls() {
    let check = State::new(false);
    let number = State::new(0.5);
    let host = Host::new({
        let (check, number) = (check.clone(), number.clone());
        move || {
            view! { column {
                button("disabled", enabled: false, on_click: || panic!("disabled"));
                checkbox("Check", bind: check); slider(bind: number, min: 0, max: 1, step: 0.1);
            } }
        }
    });
    host.layout(Size::new(300., 200.), &Metrics).unwrap();
    key(&host, Key::Tab);
    key(&host, Key::Space);
    assert!(check.get());
    key(&host, Key::Tab);
    key(&host, Key::Right);
    assert!((number.get() - 0.6).abs() < 0.00001);
    key(&host, Key::End);
    assert_eq!(number.get(), 1.);
    host.set_number(node(&host, Kind::Slider).id, 5.).unwrap();
    assert_eq!(number.get(), 1.);
}
#[test]
fn editing_preserves_graphemes_and_ime_draft_then_commits() {
    let value = State::new("A👨‍👩‍👧‍👦é".to_owned());
    let host = Host::new({
        let value = value.clone();
        move || view! { text_field("Name", bind: value); }
    });
    host.layout(Size::new(300., 50.), &Metrics).unwrap();
    let id = node(&host, Kind::TextField).id;
    host.focus(id).unwrap();
    host.text_command(TextCommand::End, false).unwrap();
    host.text_command(TextCommand::Backspace, false).unwrap();
    host.text_command(TextCommand::Backspace, false).unwrap();
    assert_eq!(value.get(), "A");
    host.mark_text("日本", 6..6, None).unwrap();
    assert_eq!(value.get(), "A");
    assert_eq!(host.text_input().unwrap().unwrap().text, "A日本");
    host.unmark_text(false).unwrap();
    assert_eq!(host.text_input().unwrap().unwrap().text, "A");
    host.mark_text("日本", 6..6, None).unwrap();
    host.unmark_text(true).unwrap();
    assert_eq!(value.get(), "A日本");
    value.set("External".into());
    assert_eq!(host.text_input().unwrap().unwrap().text, "External");
}

#[test]
fn inspector_picks_real_nodes_and_restores_style_overrides() {
    let inspector = Inspector::new();
    let host = Host::new(|| view! {column(padding:10){button("Inspect",on_click:||{});}});
    host.set_inspector(inspector.clone());
    host.layout(Size::new(400., 200.), &Metrics).unwrap();
    let info = host.pick(Point { x: 20., y: 20. }).unwrap().unwrap();
    assert_eq!(info.kind, Kind::Button);
    let original = info.bounds.width;
    inspector.selected.set(Some(info.clone()));
    inspector
        .set_override(info.id, StyleOverride::default().width(120))
        .unwrap();
    host.layout(Size::new(400., 200.), &Metrics).unwrap();
    assert_eq!(inspector.selected.get().unwrap().bounds.width, 120.);
    inspector.reset_all_overrides();
    host.layout(Size::new(400., 200.), &Metrics).unwrap();
    assert_eq!(inspector.selected.get().unwrap().bounds.width, original);
}

#[test]
fn percentage_heights_resolve_against_allocated_parent_bounds() {
    let host = Host::new(|| {
        view! {column(gap:0){
            button("Half",height:percent(50.),on_click:||{});
            button("Quarter",position:Position::Absolute,left:0,top:0,width:percent(25.),height:percent(25.),on_click:||{});
        }}
    });
    host.layout(Size::new(400., 200.), &Metrics).unwrap();
    let nodes = host.snapshot().unwrap();
    assert_eq!(
        nodes
            .iter()
            .find(|n| n.text == "Half")
            .unwrap()
            .bounds
            .height,
        100.
    );
    assert_eq!(
        nodes.iter().find(|n| n.text == "Quarter").unwrap().bounds,
        Rect::new(0., 0., 100., 50.)
    );
}

#[test]
fn slider_page_step_is_independent_of_arrows_and_obeys_bounds_and_readonly() {
    let value = State::new(0.3);
    let readonly = State::new(false);
    let host = Host::new({
        let (value, readonly) = (value.clone(), readonly.clone());
        move || view! { slider(bind:value,min:0,max:1,step:0.05,page_step:0.3,read_only:readonly.get()); }
    });
    host.layout(Size::new(300., 80.), &Metrics).unwrap();
    let id = node(&host, Kind::Slider).id;
    host.focus(id).unwrap();
    key(&host, Key::PageUp);
    assert!((value.get() - 0.6).abs() < 1e-10);
    key(&host, Key::Right);
    assert!((value.get() - 0.65).abs() < 1e-10);
    key(&host, Key::PageDown);
    assert!((value.get() - 0.35).abs() < 1e-10);
    key(&host, Key::PageDown);
    key(&host, Key::PageDown);
    assert_eq!(value.get(), 0.);
    readonly.set(true);
    key(&host, Key::PageUp);
    assert_eq!(value.get(), 0.);
    for invalid in [0., -1., f64::NAN, f64::INFINITY] {
        let host = Host::new(move || {
            View::slider(State::new(0.).binding(), 0., 1., 0.1).page_step(invalid)
        });
        assert!(host.snapshot().is_err());
    }
}

#[test]
fn labeled_components_supply_names_and_preserve_explicit_names() {
    fn custom(value: &State<String>) -> View {
        view! { labeled("Account", View::text_field("",value.binding()),key:"account"); }
    }
    let value = State::new("Ada".to_owned());
    let host = Host::new(move || {
        view! { column {
            custom(&value);
            labeled("Visible caption",View::text_field("",value.binding()).accessible_label("Explicit name"));
        } }
    });
    host.layout(Size::new(300., 200.), &Metrics).unwrap();
    let fields: Vec<_> = host
        .snapshot()
        .unwrap()
        .into_iter()
        .filter(|n| n.kind == Kind::TextField)
        .collect();
    assert_eq!(fields[0].label, "Account");
    assert_eq!(fields[1].label, "Explicit name");
    assert!(
        host.snapshot()
            .unwrap()
            .iter()
            .any(|n| n.text == "Visible caption")
    );
}
