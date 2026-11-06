use super::*;
pub fn view(m: &Lab) -> View {
    let list = m.list.clone();
    let tree = m.tree.clone();
    let table = m.table.clone();
    grid(vec![
        card(
            m,
            "list",
            "Virtual list · 10,000 rows",
            view! {column(gap:12){
                embed(m.list.view(),height:190,key:"list");
                button("Reveal last row",on_click:move||{list.reveal(9999);});
                label(format!("{} mounted rows",m.list.realized_indices().len()));
            }},
        ),
        card(
            m,
            "tree",
            "Lazy tree",
            view! {column(gap:12){
                embed(m.tree.view(),height:190,key:"tree");
                button("Load lazy branch",on_click:move||{let _=tree.expand("lazy");});
            }},
        ),
        card(
            m,
            "combo",
            "Virtual combo · 1,000 choices",
            view! {column(gap:12){
                embed(m.combo.view("Find a document"),key:"combo");
                label("Type to filter; arrows highlight; Return accepts; Escape restores the opening query.",wrap:true);
            }},
        ),
        card(
            m,
            "table",
            "Editable table · 1,000 rows",
            view! {column(gap:12){
                embed(m.table.view(),height:220,key:"table");
                button("Edit first name",enabled:!m.readonly.get(),on_click:move||{let _=table.start_edit(TableCell::new("0","name"));});
                label("Headers sort/resize/reorder. Return starts editing a name; Escape cancels.",wrap:true);
            }},
        ),
    ])
}
