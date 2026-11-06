//! Completion M2: scoped search and explicit password policies. Also --light/--render/--smoke.
mod common;
use scars_window::ui::*;
use std::time::Duration;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let query = State::new(String::new());
    let last = State::new(String::from("No query yet"));
    let password = State::new("secret 😀".to_owned());
    let readonly = State::new(false);
    let submitted = State::new(0);
    common::run("SCARS · Field policies", (700, 460), move || {
        view! {column(padding:28,gap:20){
            label("Fields with a little more care",label_style:LabelStyle::Heading,wrap:true);
            search_field("Search…",bind:query,key:"search",debounce:Duration::from_millis(250),on_query:last.handler(|state,q:String|state.set(if q.is_empty(){"Query cleared".into()}else{format!("Search: {q}")})),on_submit:submitted.action(|v|v.update(|n|*n+=1)));
            label(format!("{} · {} submissions",last.get(),submitted.get()),label_style:LabelStyle::Secondary,wrap:true);
            separator();
            checkbox("Read only",bind:readonly);
            password_field("Password",bind:password,key:"password",allow_reveal:true,read_only:readonly.get(),max_length:12);
            label("Reveal is opt-in. Copy/cut and accessibility text stay protected while shown. Focus loss hides it again.",wrap:true,label_style:LabelStyle::Secondary);
            label("Search callbacks follow committed edits; replacement, removal and deactivation cancel pending queries.",wrap:true,label_style:LabelStyle::Micro);
        }}
    })
}
