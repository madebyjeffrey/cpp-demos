use super::*;
pub fn view(m: &Lab) -> View {
    let readonly = m.readonly.get();
    let validation = Validation {
        status: if m.text.get().trim().is_empty() {
            ValidationStatus::Incomplete
        } else {
            ValidationStatus::Valid
        },
        message: m
            .text
            .get()
            .trim()
            .is_empty()
            .then(|| "Enter a project name".into()),
        dirty: m.text.get().trim().is_empty(),
    };
    grid(vec![
        card(
            m,
            "text",
            "Text fields and editor",
            view! {column(gap:12){
                labeled("Project name",View::text_field("Name",m.text.binding()).read_only(readonly).validation(validation).key("name"));
                text_editor("Notes",bind:m.notes,height:160,read_only:readonly,accessible_label:"Notes",key:"notes");
                text_field("Read-only example",bind:m.text,read_only:true,accessible_label:"Read-only name");
            }},
        ),
        card(
            m,
            "search",
            "Search",
            view! {column(gap:12){
                search_field("Find a document",bind:m.search,read_only:readonly,debounce:std::time::Duration::from_millis(200),on_query:m.status.handler(|s,query|s.set(format!("Search: {query}"))),key:"search");
                label("Queries debounce; clear and Escape cancel pending work.",wrap:true);
            }},
        ),
        card(
            m,
            "password",
            "Password",
            view! {column(gap:12){
                password_field("Password",bind:m.password,read_only:readonly,allow_reveal:true,accessible_label:"Password",key:"password");
                label("Reveal is explicit. Masked values are excluded from clipboard and inspector snapshots.",wrap:true);
            }},
        ),
        card(
            m,
            "numeric",
            "Numeric drafts and steppers",
            view! {column(gap:12){
                numeric_field("Copies (0–20)",&m.number.clone().read_only(readonly));
                stepper("Standalone stepper",&m.number.clone().read_only(readonly));
                label("Unfinished text stays a draft. Enter commits; Escape cancels.",wrap:true);
            }},
        ),
    ])
}
