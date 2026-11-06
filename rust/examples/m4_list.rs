//! Keyed list interaction and bounded realization across 100,000 rows.
mod common;
use scars_window::ui::*;
fn rows(reverse: bool) -> Vec<ListItem> {
    let mut rows = (0..100_000)
        .map(|i| ListItem::new(format!("doc-{i}"), format!("Document {i:06}")).enabled(i % 13 != 0))
        .collect::<Vec<_>>();
    if reverse {
        rows.reverse();
    }
    rows
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let activity = State::new("Select a document; Return or double-click opens it.".to_string());
    let reverse = State::new(false);
    let list = ListModel::new(rows(false), 34.)
        .options(ListOptions {
            selection: ListSelectionMode::Multiple,
            visible_rows: 10,
            overscan_rows: 3,
            ..Default::default()
        })?
        .on_activate(activity.handler(|s, key| s.set(format!("Opened {key}"))));
    common::run("SCARS · Keyed lists", (820, 600), move || {
        let sort = list.clone();
        let direction = reverse.clone();
        let tune = list.clone();
        let clear = list.clone();
        view! {column(padding:28,gap:16){
            label("A hundred thousand, a handful mounted",label_style:LabelStyle::Heading,shrink:0);
            label("Shift extends a range. ⌘-click toggles one row. ⌘-Shift adds a range. Disabled rows are skipped.",wrap:true,shrink:0,label_style:LabelStyle::Secondary);
            row(shrink:0){
                button("Reverse order",on_click:move||{let next=!direction.get();sort.set_items(rows(next)).unwrap();direction.set(next);});
                button("Toggle overscan",on_click:move||{let mut options=tune.list_options();options.overscan_rows=if options.overscan_rows==3{0}else{3};tune.set_options(options).unwrap();});
                button("Clear selection",on_click:move||clear.clear_selection());
            }
            embed(list.view(),grow:1,key:"documents");
            label(format!("{} selected · cursor {} · {} realized rows · overscan {}",list.selection_count(),list.cursor().as_deref().unwrap_or("—"),list.visible_range().len(),list.list_options().overscan_rows),wrap:true,shrink:0,label_style:LabelStyle::Micro);
            label(activity.get(),wrap:true,shrink:0,label_style:LabelStyle::Status);
        }}
    })
}
