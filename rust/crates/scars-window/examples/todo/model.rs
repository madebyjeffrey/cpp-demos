//! Small application model: durable keys, derived bindings, and atomic local storage.
use scars_ui::{Binding, State};
use std::{collections::HashSet, io::Write, path::PathBuf};
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Todo {
    pub id: u64,
    pub title: String,
    pub done: bool,
}
#[derive(Clone)]
pub struct Todos {
    pub items: State<Vec<Todo>>,
    pub input: State<String>,
    pub filter: State<String>,
    pub editing: State<Option<u64>>,
    pub draft: State<String>,
    pub error: State<Option<String>>,
    next: State<u64>,
    path: Option<PathBuf>,
}
impl Todos {
    pub fn load(path: Option<PathBuf>) -> Result<Self, String> {
        let items = if let Some(path) = &path {
            match std::fs::read_to_string(path) {
                Ok(text) => decode(&text)?,
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => vec![],
                Err(e) => return Err(e.to_string()),
            }
        } else {
            vec![]
        };
        Ok(Self::new(items, path))
    }
    fn new(items: Vec<Todo>, path: Option<PathBuf>) -> Self {
        let next = items.iter().map(|v| v.id).max().unwrap_or(0) + 1;
        Self {
            items: State::new(items),
            input: State::new(String::new()),
            filter: State::new("all".into()),
            editing: State::new(None),
            draft: State::new(String::new()),
            error: State::new(None),
            next: State::new(next),
            path,
        }
    }
    pub fn preview() -> Self {
        Self::new(
            vec![
                Todo {
                    id: 1,
                    title: "Make room for the important things".into(),
                    done: false,
                },
                Todo {
                    id: 2,
                    title: "Try the Rust component toolkit".into(),
                    done: true,
                },
                Todo {
                    id: 3,
                    title: "Take a proper lunch break".into(),
                    done: false,
                },
            ],
            None,
        )
    }
    pub fn action(&self, action: impl Fn(&Self) + 'static) -> impl Fn() + 'static {
        let model = self.clone();
        move || action(&model)
    }
    pub fn add(&self) {
        let title = self.input.get().trim().to_owned();
        if title.is_empty() {
            return;
        }
        let id = self.next.get().max(
            self.items
                .get()
                .iter()
                .map(|item| item.id)
                .max()
                .unwrap_or(0)
                .saturating_add(1),
        );
        let Some(next) = id.checked_add(1) else {
            self.error.set(Some("No task identifiers remain".into()));
            return;
        };
        self.next.set(next);
        self.items.update(|items| {
            items.push(Todo {
                id,
                title,
                done: false,
            })
        });
        self.input.set(String::new());
        self.save();
    }
    pub fn remove(&self, id: u64) {
        self.items
            .update(|items| items.retain(|item| item.id != id));
        if self.editing.get() == Some(id) {
            self.cancel_edit();
        }
        self.save();
    }
    pub fn rename(&self, id: u64, title: &str) {
        let title = title.trim();
        if title.is_empty() {
            self.remove(id);
            return;
        }
        self.items.update(|items| {
            if let Some(item) = items.iter_mut().find(|item| item.id == id) {
                item.title = title.into();
            }
        });
        self.save();
    }
    pub fn begin_edit(&self, id: u64) {
        if let Some(item) = self.items.get().into_iter().find(|item| item.id == id) {
            self.draft.set(item.title);
            self.editing.set(Some(id));
        }
    }
    pub fn commit_edit(&self) {
        if let Some(id) = self.editing.get() {
            self.rename(id, &self.draft.get());
            self.cancel_edit();
        }
    }
    pub fn cancel_edit(&self) {
        self.editing.set(None);
        self.draft.set(String::new());
    }
    pub fn clear_completed(&self) {
        self.items.update(|items| items.retain(|item| !item.done));
        self.save();
    }
    pub fn remaining(&self) -> usize {
        self.items.get().iter().filter(|item| !item.done).count()
    }
    pub fn visible(&self) -> Vec<Todo> {
        let filter = self.filter.get();
        self.items
            .get()
            .into_iter()
            .filter(|item| match filter.as_str() {
                "active" => !item.done,
                "completed" => item.done,
                _ => true,
            })
            .collect()
    }
    pub fn done(&self, id: u64) -> Binding<bool> {
        let read = self.clone();
        let write = self.clone();
        Binding::new(
            move || {
                read.items
                    .get()
                    .iter()
                    .find(|item| item.id == id)
                    .is_some_and(|item| item.done)
            },
            move |done| {
                write.items.update(|items| {
                    if let Some(item) = items.iter_mut().find(|item| item.id == id) {
                        item.done = done;
                    }
                });
                write.save();
            },
        )
    }
    pub fn all_done(&self) -> Binding<bool> {
        let read = self.clone();
        let write = self.clone();
        Binding::new(
            move || {
                let items = read.items.get();
                !items.is_empty() && items.iter().all(|item| item.done)
            },
            move |done| {
                write.items.update(|items| {
                    for item in items {
                        item.done = done;
                    }
                });
                write.save();
            },
        )
    }
    fn save(&self) {
        let Some(path) = &self.path else {
            return;
        };
        let result = (|| -> std::io::Result<()> {
            if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
                std::fs::create_dir_all(parent)?;
            }
            let temporary = path.with_extension(format!("tmp-{}", std::process::id()));
            let mut file = std::fs::File::create(&temporary)?;
            file.write_all(encode(&self.items.get()).as_bytes())?;
            file.sync_all()?;
            std::fs::rename(temporary, path)
        })();
        self.error.set(
            result
                .err()
                .map(|e| format!("Changes are in memory; saving failed: {e}")),
        );
    }
}
fn escape(text: &str) -> String {
    text.replace('\\', "\\\\")
        .replace('\t', "\\t")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
}
fn unescape(text: &str) -> Result<String, String> {
    let mut output = String::new();
    let mut chars = text.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            output.push(c);
            continue;
        }
        output.push(match chars.next() {
            Some('\\') => '\\',
            Some('t') => '\t',
            Some('n') => '\n',
            Some('r') => '\r',
            _ => return Err("Invalid escape in todo data".into()),
        });
    }
    Ok(output)
}
pub fn encode(items: &[Todo]) -> String {
    let mut text = "SCARS-TODO-1\n".to_owned();
    for item in items {
        text.push_str(&format!(
            "{}\t{}\t{}\n",
            item.id,
            u8::from(item.done),
            escape(&item.title)
        ));
    }
    text
}
pub fn decode(text: &str) -> Result<Vec<Todo>, String> {
    let mut lines = text.lines();
    if lines.next() != Some("SCARS-TODO-1") {
        return Err("Unsupported todo data format".into());
    }
    let mut ids = HashSet::new();
    let mut items = Vec::new();
    for line in lines {
        let fields: Vec<_> = line.splitn(3, '\t').collect();
        if fields.len() != 3 {
            return Err("Incomplete todo record".into());
        }
        let id = fields[0].parse::<u64>().map_err(|e| e.to_string())?;
        if id == 0 || id == u64::MAX || !ids.insert(id) {
            return Err("Invalid or duplicate task identifier".into());
        }
        let done = match fields[1] {
            "0" => false,
            "1" => true,
            _ => return Err("Invalid completion flag".into()),
        };
        let title = unescape(fields[2])?;
        if title.trim().is_empty() {
            return Err("Empty saved task".into());
        }
        items.push(Todo { id, title, done });
    }
    Ok(items)
}
