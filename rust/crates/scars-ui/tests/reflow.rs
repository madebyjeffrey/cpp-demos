use scars_ui::*;
use std::rc::Rc;
struct Metrics;
impl Measure for Metrics {
    fn text(&self, text: &str, style: TextStyle, width: f64) -> TextLayout {
        let natural = text.chars().count() as f64 * 10.;
        let line = if style.line_height > 0. {
            style.line_height
        } else {
            style.size + 5.
        };
        let lines = if style.wrap && width.is_finite() {
            (natural / width.max(1.)).ceil().max(1.)
        } else {
            1.
        };
        let lines = if style.max_lines > 0 {
            lines.min(style.max_lines as f64)
        } else {
            lines
        };
        TextLayout {
            lines: Vec::new(),
            placeholders: Vec::new(),
            size: Size::new(
                if style.constrained() {
                    natural.min(width)
                } else {
                    natural
                },
                line * lines,
            ),
            baseline: style.size * 0.8,
            hit_test: Rc::new(|_| 0),
            range_rects: Rc::new(|_| vec![]),
        }
    }
    fn image(&self, _: &Bitmap) -> Result<Size, Error> {
        Ok(Size::new(10., 30.))
    }
}
fn node(host: &Host, text: &str) -> Node {
    host.snapshot()
        .unwrap()
        .into_iter()
        .find(|n| n.text == text)
        .unwrap()
}
#[test]
fn resized_wrapping_content_reflows_its_ancestors_and_following_siblings() {
    let host = Host::new(|| {
        view! {column(gap:10){
            row(gap:10){label("abcdefghijklmnopqrstuvwxyz",wrap:true);button("Go",width:70,shrink:0,on_click:||{});}
            label("Footer");
        }}
    });
    for (width, height, footer) in [
        (300., 40., 50.),
        (180., 60., 70.),
        (600., 34., 44.),
        (300., 40., 50.),
    ] {
        host.layout(Size::new(width, 200.), &Metrics).unwrap();
        let row = host
            .snapshot()
            .unwrap()
            .into_iter()
            .find(|n| n.kind == Kind::Row)
            .unwrap();
        assert_eq!(row.bounds.height, height);
        assert_eq!(node(&host, "Footer").bounds.y, footer);
        assert!(node(&host, "abcdefghijklmnopqrstuvwxyz").text_bounds.height <= row.bounds.height);
    }
}
#[test]
fn baseline_alignment_includes_control_centering_and_nested_container_padding() {
    let host = Host::new(|| {
        view! {column(gap:10){row(align:Align::Baseline,gap:10){
            label("Small",font_size:12);label("Large",font_size:28);button("Control",height:44,on_click:||{});
            column(padding:3){label("Nested",font_size:20);}
        }label("Footer");}}
    });
    host.layout(Size::new(800., 200.), &Metrics).unwrap();
    let nodes = [
        node(&host, "Small"),
        node(&host, "Large"),
        node(&host, "Control"),
        node(&host, "Nested"),
    ];
    for n in &nodes {
        assert!((n.bounds.y + n.baseline - 24.).abs() < 0.0001);
        assert_eq!(n.clip, n.bounds);
    }
    assert!((node(&host, "Footer").bounds.y - 54.).abs() < 0.0001);
}
#[test]
fn wrapping_uses_flex_basis_and_recomputes_each_line_cross_extent() {
    let host = Host::new(|| {
        view! {column(gap:10){row(wrap:true,gap:10){
            label("A much wider intrinsic label",basis:100,height:30);label("B",basis:100,height:50);
            label("C",basis:100,height:20);label("D",basis:100,height:40);
        }label("Footer");}}
    });
    host.layout(Size::new(230., 300.), &Metrics).unwrap();
    assert_eq!(node(&host, "C").bounds.y, 60.);
    assert_eq!(node(&host, "D").bounds.y, 60.);
    assert_eq!(node(&host, "Footer").bounds.y, 110.);
}
#[test]
fn layer_and_absolute_children_respect_limits_and_hit_clipping() {
    let host = Host::new(|| {
        view! {layer{
            button("Limited",max_width:80,max_height:30,align_self:Align::End,on_click:||{});
            button("Absolute",position:Position::Absolute,left:250,top:80,width:100,height:60,max_width:70,max_height:30,z_index:1,on_click:||{});
        }}
    });
    host.layout(Size::new(300., 100.), &Metrics).unwrap();
    assert_eq!(
        node(&host, "Limited").bounds,
        Rect::new(220., 70., 80., 30.)
    );
    assert_eq!(
        node(&host, "Absolute").bounds,
        Rect::new(250., 80., 70., 30.)
    );
    assert_eq!(node(&host, "Absolute").clip, Rect::new(250., 80., 50., 20.));
    assert_eq!(
        host.hit_test(Point { x: 270., y: 90. }).unwrap(),
        Some(node(&host, "Absolute").id)
    );
    assert_eq!(host.hit_test(Point { x: 310., y: 90. }).unwrap(), None);
}

#[test]
fn layer_order_is_by_sibling_subtree_and_respects_parent_clipping() {
    let host = Host::new(|| {
        view! {layer{
            column(z_index:0,width:200,height:70){button("Nested",z_index:99,width:200,height:100,on_click:||{});}
            button("Front",z_index:1,width:200,height:50,on_click:||{});
        }}
    });
    host.layout(Size::new(300., 150.), &Metrics).unwrap();
    assert_eq!(
        host.hit_test(Point { x: 20., y: 20. }).unwrap(),
        Some(node(&host, "Front").id)
    );
    assert_eq!(
        host.hit_test(Point { x: 20., y: 60. }).unwrap(),
        Some(node(&host, "Nested").id)
    );
    assert_ne!(
        host.hit_test(Point { x: 20., y: 80. }).unwrap(),
        Some(node(&host, "Nested").id)
    );
}
