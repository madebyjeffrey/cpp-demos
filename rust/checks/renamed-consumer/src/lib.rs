use kit::ui::*;
pub fn inspected(inspector: &Inspector) -> View {
    let _: Vec<Inspection> = inspector.nodes();
    view! { row { button("Save", on_click: || {}); embed(inspector.view()); } }
}
pub fn selection(host: &Host) -> Option<Inspection> {
    host.inspection()
}

pub fn resize(inspector: &Inspector, id: NodeId) -> Result<bool, Error> {
    inspector.set_override(id, StyleOverride::default().width(percent(50.)).padding(12))
}

pub fn counter(count: &State<i64>) -> View {
    view! { row {
        label(count.get().to_string());
        button("+1", on_click: count.action(|s| s.update(|n| *n += 1)));
    } }
}
pub fn custom_components(first: &State<i64>, second: &State<i64>, level: &State<f64>) -> View {
    view! { column {
        counter(first, key: "first");
        counter(second, key: "second");
        labeled("Level", View::slider(level.binding(), 0., 1., 0.05).page_step(0.25));
    } }
}
