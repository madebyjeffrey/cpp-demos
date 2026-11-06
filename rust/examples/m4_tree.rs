//! Eager/lazy branches, observable load failure, deferred completion and replacement.
mod common;
use scars_window::ui::*;
fn nodes(online: &State<bool>) -> Vec<TreeNode> {
    let online = online.clone();
    vec![TreeNode::new("workspace", "Workspace").children(vec![
            TreeNode::new("local", "Local documents").children(vec![
                TreeNode::new("notes", "Notes.md"),
                TreeNode::new("design", "Design.sketch"),
            ]),
            TreeNode::new("remote", "Remote documents").lazy(move || {
                if online.get() {
                    Ok(vec![
                        TreeNode::new("remote-a", "Roadmap.md"),
                        TreeNode::new("remote-b", "Release notes.md"),
                    ])
                } else {
                    Err(Error("Connection unavailable".into()))
                }
            }),
            TreeNode::new("deferred", "Deferred documents").deferred(),
            TreeNode::new("disabled", "Unavailable workspace")
                .enabled(false)
                .expandable(true),
        ])]
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let online = State::new(false);
    let pending = State::new(None::<TreeLoad>);
    let status = State::new(
        "Click a disclosure arrow. Failed loads remain visible until an explicit retry.".to_owned(),
    );
    let expansion = State::new("No disclosure changes yet".to_owned());
    let tree = TreeModel::with_options(
        nodes(&online),
        TreeOptions {
            selection: ListSelectionMode::Multiple,
            ..Default::default()
        },
    )?
    .on_load(pending.handler(|s, request| s.set(Some(request))))
    .on_expand(expansion.handler(|s, event: TreeExpansion| {
        s.set(format!(
            "{} {}",
            if event.expanded {
                "Expanded"
            } else {
                "Collapsed"
            },
            event.key
        ))
    }))
    .on_activate_with_model({
        let status = status.clone();
        move |tree, key| {
            if tree.item(&key).is_some_and(|i| i.error.is_some()) {
                match tree.retry(&key) {
                    Ok(_) => status.set(format!("Retried {key}")),
                    Err(e) => status.set(e.to_string()),
                }
            } else {
                status.set(format!("Activated {key}"));
            }
        }
    });
    tree.expand("workspace")?;
    tree.expand("local")?;
    if let Err(error) = tree.expand("remote") {
        status.set(format!("Remote load failed: {error}"));
    }
    common::run("SCARS · Lazy tree model", (880, 620), move || {
        let connect = tree.clone();
        let backend = online.clone();
        let connect_status = status.clone();
        let request = pending.clone();
        let finish_status = status.clone();
        let replace = tree.clone();
        let source = online.clone();
        let replace_status = status.clone();
        let can_finish = pending.get().is_some();
        view! {column(padding:28,gap:18){
            label("A tree that knows what it has loaded",label_style:LabelStyle::Heading,shrink:0);
            label("Lazy snapshots cache their children. Deferred branches expose Loading until a checked completion arrives.",wrap:true,label_style:LabelStyle::Secondary,shrink:0);
            label("Right opens/enters; Left closes/visits parent. Shift selects a range; Return/double-click activates.",wrap:true,label_style:LabelStyle::Secondary,shrink:0);
            row(shrink:0){
                button("Reconnect and retry",on_click:move||{backend.set(true);match connect.retry("remote"){Ok(_)=>connect_status.set("Remote children loaded; collapse/reopen uses the cache.".into()),Err(e)=>connect_status.set(e.to_string())}});
                button("Finish deferred load",enabled:can_finish,on_click:move||{
                    if let Some(load)=request.get(){match load.complete(Ok(vec![TreeNode::new("deferred-a","Fetched document.md")])){
                        Ok(true)=>finish_status.set("Deferred result accepted.".into()),Ok(false)=>finish_status.set("Stale result ignored; the tree was replaced.".into()),Err(e)=>finish_status.set(e.to_string()),
                    }request.set(None);}
                });
                button("Replace snapshot",on_click:move||{replace.set_nodes(nodes(&source)).unwrap();replace.expand("workspace").unwrap();replace_status.set("New snapshot: prior requests and item lifetimes retired.".into());});
            }
            embed(tree.view(),grow:1,key:"workspace-tree");
            label(format!("{} known nodes · {} visible · deferred request: {}",tree.known_count(),tree.visible_count(),pending.get().map_or("none".into(),|load|format!("{} ({})",load.key(),if load.is_current(){"current"}else{"stale"}))),wrap:true,label_style:LabelStyle::Micro,shrink:0);
            label(format!("{} selected · cursor {} · {}",tree.selection_count(),tree.cursor().as_deref().unwrap_or("—"),expansion.get()),wrap:true,label_style:LabelStyle::Micro,shrink:0);
            label(status.get(),wrap:true,label_style:LabelStyle::Status,shrink:0);
        }}
    })
}
