use scars_ui::*;
use std::rc::Rc;
struct Metrics;
impl Measure for Metrics {
    fn text(&self, text: &str, _: TextStyle, _: f64) -> TextLayout {
        TextLayout {
            lines: Vec::new(),
            placeholders: Vec::new(),
            size: Size::new(text.len() as f64 * 8., 18.),
            baseline: 14.,
            hit_test: Rc::new(|_| 0),
            range_rects: Rc::new(|_| vec![]),
        }
    }
    fn image(&self, _: &Bitmap) -> Result<Size, Error> {
        Ok(Size::default())
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
#[test]
fn scroll_clips_hits_and_clamps_at_edges() {
    let host = Host::new(
        || view! {scroll(height:100,gap:0){for i in 0..10{button(format!("{i}"),height:30,key:format!("{i}"),on_click:||{});}}},
    );
    host.layout(Size::new(200., 100.), &Metrics).unwrap();
    let p = Point { x: 50., y: 50. };
    assert!(host.scroll(p, 10000.).unwrap());
    host.layout(Size::new(200., 100.), &Metrics).unwrap();
    let last = host.snapshot().unwrap().last().unwrap().clone();
    assert_eq!(last.bounds.y, 70.);
    assert!(!host.scroll(p, 10.).unwrap());
    assert!(host.hit_test(Point { x: 50., y: 101. }).unwrap().is_none());
    assert!(host.scroll(p, -10000.).unwrap());
    host.layout(Size::new(200., 100.), &Metrics).unwrap();
    assert!(!host.scroll(p, -10.).unwrap());
}
#[test]
fn modal_traps_focus_dismisses_and_restores_anchor() {
    let popup = Popup::new();
    let host = Host::new({
        let popup = popup.clone();
        move || {
            View::layer(vec![
                View::button("Anchor", || {}).key("anchor"),
                popup.modal("Dialog", View::button("Inside", || {})),
            ])
        }
    });
    host.layout(Size::new(500., 400.), &Metrics).unwrap();
    key(&host, Key::Tab);
    let anchor = host.focused().unwrap();
    popup.show();
    host.layout(Size::new(500., 400.), &Metrics).unwrap();
    assert_ne!(host.focused(), Some(anchor));
    assert!(!host.in_scope(anchor));
    let modal = host.active_modal().unwrap();
    for _ in 0..5 {
        key(&host, Key::Tab);
        assert!(host.in_scope(host.focused().unwrap()));
    }
    key(&host, Key::Escape);
    host.layout(Size::new(500., 400.), &Metrics).unwrap();
    assert_eq!(host.focused(), Some(anchor));
    assert!(!host.activate(modal).unwrap());
}
#[test]
fn popover_positions_against_anchor_and_stays_in_viewport() {
    let popup = Popup::new();
    popup.show();
    let host = Host::new(move || {
        View::layer(vec![
            View::button("Anchor", || {})
                .key("anchor")
                .position(Position::Absolute)
                .left(250)
                .top(160)
                .width(50)
                .height(30),
            popup.popover(
                "anchor",
                View::panel(vec![View::label("Menu")])
                    .width(180)
                    .height(100),
            ),
        ])
    });
    host.layout(Size::new(320., 240.), &Metrics).unwrap();
    let panel = host
        .snapshot()
        .unwrap()
        .into_iter()
        .find(|n| n.kind == Kind::Panel)
        .unwrap();
    assert_eq!(panel.bounds.x, 140.);
    assert_eq!(panel.bounds.y, 54.); // The configured six-point gap also applies when flipped above.
}
#[test]
fn dropdown_keeps_distinct_option_ids_when_labels_match() {
    let popup = Popup::new();
    let selected = State::new("first".to_owned());
    let host = Host::new({
        let popup = popup.clone();
        let selected = selected.clone();
        move || {
            let (button, overlay) = dropdown(
                "Choice",
                &selected,
                vec![
                    OptionItem::new("first", "Same"),
                    OptionItem::new("second", "Same"),
                ],
                &popup,
                "choice",
            );
            View::layer(vec![button, overlay])
        }
    });
    host.layout(Size::new(500., 400.), &Metrics).unwrap();
    popup.show();
    host.layout(Size::new(500., 400.), &Metrics).unwrap();
    key(&host, Key::End);
    key(&host, Key::Enter);
    assert_eq!(selected.get(), "second");
}
