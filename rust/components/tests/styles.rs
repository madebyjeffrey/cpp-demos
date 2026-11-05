use scars_ui::*;
use std::rc::Rc;
struct Metrics;
impl Measure for Metrics {
    fn text(&self, text: &str, style: TextStyle, _: f64) -> TextLayout {
        TextLayout {
            placeholders: Vec::new(),
            size: Size::new(text.len() as f64 * 8., style.size + 4.),
            baseline: style.size,
            hit_test: Rc::new(|_| 0),
            range_rects: Rc::new(|_| vec![]),
        }
    }
    fn image(&self, _: &Bitmap) -> Result<Size, Error> {
        Ok(Size::new(10., 10.))
    }
}
fn text(host: &Host, value: &str) -> Node {
    host.snapshot()
        .unwrap()
        .into_iter()
        .find(|n| n.text == value)
        .unwrap()
}
#[test]
fn insets_and_relative_offsets_keep_the_original_flow_slot() {
    let host = Host::new(|| {
        view! {column(padding:Insets::from([10.,20.,30.,40.]),gap:0,align:Align::Start){
            button("Moved",width:20,height:20,margin:Insets::from([5.,7.,11.,13.]),position:Position::Relative,left:percent(10.),top:5,on_click:||{});
            button("Next",width:20,height:20,on_click:||{});
        }}
    });
    host.layout(Size::new(400., 240.), &Metrics).unwrap();
    assert_eq!(text(&host, "Moved").bounds, Rect::new(87., 20., 20., 20.));
    assert_eq!(text(&host, "Next").bounds, Rect::new(40., 46., 20., 20.));
}
#[test]
fn flex_basis_redistributes_after_items_reach_limits_and_honors_self_alignment() {
    let host = Host::new(|| {
        view! {row(padding:10,gap:0,align:Align::Start){
            button("Capped",basis:60,grow:1,max_width:80,height:20,on_click:||{});
            button("Flexible",basis:60,grow:3,height:20,align_self:Align::End,on_click:||{});
        }}
    });
    host.layout(Size::new(360., 100.), &Metrics).unwrap();
    assert_eq!(text(&host, "Capped").bounds, Rect::new(10., 10., 80., 20.));
    assert_eq!(
        text(&host, "Flexible").bounds,
        Rect::new(90., 70., 260., 20.)
    );
    let shrinking = Host::new(
        || view! {row(gap:0){button("Min",basis:150,min_width:120,on_click:||{});button("Rest",basis:150,on_click:||{});}},
    );
    shrinking.layout(Size::new(200., 40.), &Metrics).unwrap();
    assert_eq!(text(&shrinking, "Min").bounds.width, 120.);
    assert_eq!(text(&shrinking, "Rest").bounds.width, 80.);
}
#[test]
fn space_around_and_vertical_separators_use_the_allocated_cross_extent() {
    let host = Host::new(|| {
        view! {row(justify:Justify::SpaceAround,gap:0){
            button("A",width:40,on_click:||{});separator(orientation:Orientation::Vertical);button("B",width:40,on_click:||{});
        }}
    });
    host.layout(Size::new(301., 40.), &Metrics).unwrap();
    let a = text(&host, "A");
    let b = text(&host, "B");
    assert!((a.bounds.x - 220. / 6.).abs() < 0.001);
    assert!((b.bounds.x + 40. - (301. - 220. / 6.)).abs() < 0.001);
    let line = host
        .snapshot()
        .unwrap()
        .into_iter()
        .find(|n| n.kind == Kind::Separator)
        .unwrap();
    assert_eq!(line.bounds.width, 1.);
    assert_eq!(line.bounds.height, 40.);
}
#[test]
fn text_roles_and_accent_density_inherit_without_mutating_source_descriptions() {
    let upper = State::new(true);
    let host = Host::new({
        let upper = upper.clone();
        move || {
            view! {column(accent:0xffbb5500,density:Density::Compact){
                label("Straße",uppercase:upper.get());label("Status",label_style:LabelStyle::Status);button("Small",on_click:||{});
            }}
        }
    });
    host.layout(Size::new(300., 200.), &Metrics).unwrap();
    assert_eq!(text(&host, "Small").bounds.height, 29.);
    assert_eq!(text(&host, "Status").text_style.color, 0xffbb5500);
    text(&host, "STRASSE");
    upper.set(false);
    host.layout(Size::new(300., 200.), &Metrics).unwrap();
    text(&host, "Straße");
    let invalid = Host::new(|| view! {label("Bad",padding:Insets::from([1.,-1.,1.,1.]));});
    assert!(invalid.refresh().is_err());
}
