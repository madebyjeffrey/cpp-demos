//! A classic todo app composed from SCARS Rust controls.
mod common;
#[path = "todo/model.rs"]
mod model;
#[path = "todo/view.rs"]
mod todo_view;
use model::Todos;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    let preview = args
        .iter()
        .any(|v| matches!(v.as_str(), "--render" | "--smoke" | "--preview"));
    let path = if let Some(i) = args.iter().position(|v| v == "--data") {
        Some(std::path::PathBuf::from(
            args.get(i + 1).ok_or("--data needs a path")?,
        ))
    } else {
        std::env::var_os("HOME").map(|home| {
            std::path::PathBuf::from(home).join("Library/Application Support/SCARS Todo/tasks.tsv")
        })
    };
    let model = if preview {
        Todos::preview()
    } else {
        Todos::load(path)?
    };
    common::run("SCARS · Things to do", (760, 720), move || {
        todo_view::view(&model)
    })
}
