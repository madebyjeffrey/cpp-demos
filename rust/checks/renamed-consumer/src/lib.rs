use kit::ui::*;
pub fn inspected(inspector: &Inspector) -> View {
    let _: Vec<Inspection> = inspector.nodes();
    view! { row { button("Save", on_click: || {}); embed(inspector.view()); } }
}
pub fn selection(host: &Host) -> Option<Inspection> {
    host.inspection()
}
