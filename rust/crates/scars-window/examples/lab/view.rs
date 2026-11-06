//! The catalog composes plain Rust functions and retained reactive models.
use super::model::{Counter, Lab};
use scars_window::ui::*;
#[path = "pages/controls.rs"]
mod controls;
#[path = "pages/data.rs"]
mod data;
#[path = "pages/fields.rs"]
mod fields;
#[path = "pages/flow.rs"]
mod flow;
#[path = "pages/layout.rs"]
mod layout;
#[path = "pages/values.rs"]
mod values;

pub const PAGES: &[(&str, &str)] = &[
    ("controls", "Controls"),
    ("layout", "Layout"),
    ("fields", "Fields"),
    ("flow", "Flow"),
    ("data", "Data"),
    ("values", "Values"),
    ("tools", "Tools"),
];
/// Stable keys also drive sample reachability and visual regression checks.
pub const SAMPLES: &[(&str, &str)] = &[
    ("actions", "controls"),
    ("choices", "controls"),
    ("slider", "controls"),
    ("custom", "controls"),
    ("typography", "layout"),
    ("boxes", "layout"),
    ("images", "layout"),
    ("styles", "layout"),
    ("scroll", "layout"),
    ("text", "fields"),
    ("search", "fields"),
    ("password", "fields"),
    ("numeric", "fields"),
    ("menus", "flow"),
    ("overlays", "flow"),
    ("splits", "flow"),
    ("tabs", "flow"),
    ("wizard", "flow"),
    ("list", "data"),
    ("tree", "data"),
    ("combo", "data"),
    ("table", "data"),
    ("date", "values"),
    ("time", "values"),
    ("datetime", "values"),
    ("zoned", "values"),
    ("color", "values"),
    ("inspector", "tools"),
];
pub fn card(model: &Lab, id: &str, title: &str, body: View) -> View {
    View::group_box(title, vec![body])
        .key(format!("sample-{id}"))
        .basis(330)
        .grow(1.)
        .min_width(0.)
        .padding(16)
        .enabled(model.enabled.get())
}
pub fn grid(cards: Vec<View>) -> View {
    View::scroll(vec![
        View::row(cards).wrap(true).align(Align::Start).gap(16.),
    ])
    .grow(1.)
    .min_height(0.)
}
/// A reusable stateful component: ownership and reactivity stay in its model.
pub fn counter(title: &str, model: &Counter) -> View {
    view! { column(gap:8) {
        label(format!("{title}: {}",model.count.get()),key:"counter-value");
        row {
            button("+1",on_click:model.count.action(|s|s.update(|n|*n+=1)),accessible_label:format!("Increase {title}"));
            button("Clear",on_click:model.count.action(|s|s.set(0)),enabled:model.count.get()!=0);
        }
        checkbox("Show details",bind:model.details);
        if model.details.get(){label("Each instance owns independent reactive state.",wrap:true);}
    } }
}
pub fn page(model: &Lab, id: &str) -> View {
    match id {
        "controls"=>controls::view(model),"layout"=>layout::view(model),"fields"=>fields::view(model),
        "flow"=>flow::view(model),"data"=>data::view(model),"values"=>values::view(model),
        "tools"=>View::column(vec![View::label("Browse the mounted catalog. Visited pages retain their controls; hidden pages stay inspectable.").wrap(true),model.inspector.view().grow(1.)]).min_height(0.).key("sample-inspector"),
        _=>unreachable!("validated catalog page"),
    }
}
pub fn view(model: &Lab) -> View {
    let pages = model
        .pages
        .view(
            PAGES
                .iter()
                .map(|(id, title)| {
                    let model = model.clone();
                    Tab::lazy(*id, *title, move || page(&model, id))
                })
                .collect(),
        )
        .accessible_label("Component catalog");
    view! { column(padding:20,gap:12) {
        label("Component lab",label_style:LabelStyle::Heading,shrink:0);
        row(wrap:true,shrink:0) {
            switch("Enable samples",bind:model.enabled);
            checkbox("Read-only editors",bind:model.readonly_binding());
        }
        embed(pages,grow:1,min_height:0);
        label(model.status.get(),wrap:true,label_style:LabelStyle::Status,shrink:0);
    } }
}

/// Focus a real interactive descendant, preserving the normal scope policy.
pub fn focus_sample(host: &Host, key: Option<&str>) -> Result<Option<NodeId>, Error> {
    let nodes = host.snapshot()?;
    let card = nodes
        .iter()
        .find(|n| {
            !n.hidden
                && match key {
                    Some(key) => n.key.as_deref() == Some(key),
                    None => n.key.as_ref().is_some_and(|key| key.starts_with("sample-")),
                }
        })
        .ok_or_else(|| Error("No active sample matches the focus request".into()))?;
    let parents: std::collections::HashMap<_, _> = nodes.iter().map(|n| (n.id, n.parent)).collect();
    for n in &nodes {
        let mut ancestor = Some(n.id);
        while ancestor.is_some() && ancestor != Some(card.id) {
            ancestor = parents.get(&ancestor.unwrap()).copied().flatten();
        }
        if ancestor == Some(card.id)
            && n.enabled
            && !n.hidden
            && (host.focus(n.id)? || host.focused() == Some(n.id))
        {
            return Ok(host.focused());
        }
    }
    Ok(None) // Images and static typography need not expose a focus target.
}
