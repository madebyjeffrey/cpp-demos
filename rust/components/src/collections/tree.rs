//! Keyed tree metadata, lazy snapshots and generation-checked async completion.
use crate::{Error, Insets, ListItem, ListModel, State, View};
use std::{
    cell::{Cell, RefCell},
    collections::{BTreeSet, HashMap},
    rc::{Rc, Weak},
};
type Loader = Rc<dyn Fn() -> Result<Vec<TreeNode>, Error>>;
type LoadHandler = Rc<dyn Fn(TreeLoad)>;
type ExpandHandler = Rc<dyn Fn(TreeExpansion)>;
#[derive(Clone)]
pub(crate) struct TreeAction {
    pub model: TreeModel,
    pub key: String,
    pub token: u64,
    pub retry: bool,
}

#[derive(Clone)]
pub struct TreeNode {
    pub key: String,
    pub label: String,
    pub enabled: bool,
    pub children: Vec<TreeNode>,
    pub expandable: bool,
    pub loading: bool,
    pub error: Option<String>,
    pub accessible_label: String,
    deferred: bool,
    loader: Option<Loader>,
}
impl TreeNode {
    pub fn new(key: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            label: label.into(),
            enabled: true,
            children: vec![],
            expandable: false,
            loading: false,
            error: None,
            accessible_label: String::new(),
            deferred: false,
            loader: None,
        }
    }
    pub fn children(mut self, children: Vec<Self>) -> Self {
        self.children = children;
        self
    }
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
    pub fn expandable(mut self, expandable: bool) -> Self {
        self.expandable = expandable;
        self
    }
    pub fn accessible_label(mut self, label: impl Into<String>) -> Self {
        self.accessible_label = label.into();
        self
    }
    /// Pure snapshot loader; cached until explicit retry/replacement.
    pub fn lazy(mut self, loader: impl Fn() -> Result<Vec<Self>, Error> + 'static) -> Self {
        self.loader = Some(Rc::new(loader));
        self
    }
    /// Expansion requests children through TreeModel::on_load/pending_load.
    pub fn deferred(mut self) -> Self {
        self.deferred = true;
        self
    }
    /// Represent an already-running external load in an initial snapshot.
    pub fn loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }
    pub fn failed(mut self, message: impl Into<String>) -> Self {
        self.error = Some(message.into());
        self.deferred = true;
        self
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TreeLoadState {
    Unloaded,
    Loading,
    Loaded,
    Failed(String),
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TreeItem {
    pub key: String,
    pub label: String,
    pub parent: Option<String>,
    pub depth: usize,
    pub enabled: bool,
    pub expandable: bool,
    pub expanded: bool,
    pub loading: bool,
    pub error: Option<String>,
    pub child_count: usize,
    pub accessible_label: String,
    pub load_state: TreeLoadState,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TreeFailure {
    pub key: String,
    pub message: String,
}
#[derive(Clone)]
struct Record {
    key: String,
    label: String,
    parent: Option<String>,
    depth: usize,
    enabled: bool,
    children: Rc<Vec<String>>,
    expandable: bool,
    expanded: bool,
    accessible_label: String,
    loader: Option<Loader>,
    load: TreeLoadState,
    token: u64,
    request: Option<u64>,
}
impl Record {
    fn can_expand(&self) -> bool {
        self.expandable || !self.children.is_empty() || self.load != TreeLoadState::Loaded
    }
}
#[derive(Clone, Default)]
struct Snapshot {
    roots: Vec<String>,
    records: HashMap<String, Record>,
    revision: u64,
    failure: Option<TreeFailure>,
}
impl PartialEq for Snapshot {
    fn eq(&self, other: &Self) -> bool {
        self.revision == other.revision
    }
}
struct Inner {
    data: State<Rc<Snapshot>>,
    list: ListModel,
    on_load: RefCell<Option<LoadHandler>>,
    options: TreeOptions,
    on_expand: RefCell<Option<ExpandHandler>>,
    notifying: Cell<bool>,
}
#[derive(Clone)]
pub struct TreeModel(Rc<Inner>);
impl PartialEq for TreeModel {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}
impl Eq for TreeModel {}
/// Main-thread load completion. Clones share one request; stale/duplicate results
/// are ignored, and holding a request does not retain its tree.
#[derive(Clone)]
pub struct TreeLoad {
    owner: Weak<Inner>,
    key: String,
    token: u64,
    request: u64,
}
impl PartialEq for TreeLoad {
    fn eq(&self, other: &Self) -> bool {
        self.owner.ptr_eq(&other.owner)
            && self.token == other.token
            && self.request == other.request
    }
}
impl Eq for TreeLoad {}
impl TreeLoad {
    pub fn key(&self) -> &str {
        &self.key
    }
    pub fn is_current(&self) -> bool {
        self.owner.upgrade().is_some_and(|owner| {
            owner
                .data
                .get()
                .records
                .get(&self.key)
                .is_some_and(|r| r.token == self.token && r.request == Some(self.request))
        })
    }
    /// Ok(false) means stale/duplicate; Err records an accepted load failure.
    pub fn complete(&self, result: Result<Vec<TreeNode>, Error>) -> Result<bool, Error> {
        crate::state::check_write();
        let Some(owner) = self.owner.upgrade() else {
            return Ok(false);
        };
        TreeModel(owner).complete(self, result)
    }
}
fn message(error: Error) -> Error {
    if error.0.trim().is_empty() {
        Error("Unable to load tree children".into())
    } else {
        error
    }
}
fn register(
    nodes: Vec<TreeNode>,
    parent: Option<String>,
    depth: usize,
    records: &mut HashMap<String, Record>,
) -> Result<Vec<String>, Error> {
    let roots = nodes.iter().map(|n| n.key.clone()).collect();
    let mut stack = nodes
        .into_iter()
        .rev()
        .map(|n| (n, parent.clone(), depth))
        .collect::<Vec<_>>();
    while let Some((mut node, parent, depth)) = stack.pop() {
        if node.key.trim().is_empty() || records.contains_key(&node.key) {
            return Err(Error(format!(
                "Tree keys must be nonempty and globally unique: {:?}",
                node.key
            )));
        }
        if node.loading && node.error.is_some() {
            return Err(Error(format!(
                "Tree node {} cannot be loading and failed",
                node.key
            )));
        }
        let load = if let Some(error) = node.error.take() {
            TreeLoadState::Failed(message(Error(error)).0)
        } else if node.loading {
            TreeLoadState::Loading
        } else if node.deferred || node.loader.is_some() {
            TreeLoadState::Unloaded
        } else {
            TreeLoadState::Loaded
        };
        let children = std::mem::take(&mut node.children);
        let child_keys = children.iter().map(|n| n.key.clone()).collect();
        let key = node.key.clone();
        let record = Record {
            key: key.clone(),
            label: node.label,
            parent,
            depth,
            enabled: node.enabled,
            children: Rc::new(child_keys),
            expandable: node.expandable,
            expanded: false,
            accessible_label: node.accessible_label,
            loader: node.loader,
            request: if load == TreeLoadState::Loading {
                Some(super::list::next_token())
            } else {
                None
            },
            load,
            token: super::list::next_token(),
        };
        records.insert(key.clone(), record);
        let next_depth = depth
            .checked_add(1)
            .ok_or_else(|| Error("Tree is too deep".into()))?;
        stack.extend(
            children
                .into_iter()
                .rev()
                .map(|n| (n, Some(key.clone()), next_depth)),
        );
    }
    Ok(roots)
}
fn enabled(data: &Snapshot, key: &str) -> bool {
    let mut current = Some(key);
    while let Some(key) = current {
        let Some(record) = data.records.get(key) else {
            return false;
        };
        if !record.enabled {
            return false;
        }
        current = record.parent.as_deref();
    }
    true
}
fn item(record: &Record, enabled: bool) -> TreeItem {
    TreeItem {
        key: record.key.clone(),
        label: record.label.clone(),
        parent: record.parent.clone(),
        depth: record.depth,
        enabled,
        expandable: record.can_expand(),
        expanded: record.expanded,
        loading: record.load == TreeLoadState::Loading,
        error: if let TreeLoadState::Failed(error) = &record.load {
            Some(error.clone())
        } else {
            None
        },
        child_count: record.children.len(),
        accessible_label: if record.accessible_label.is_empty() {
            record.label.clone()
        } else {
            record.accessible_label.clone()
        },
        load_state: record.load.clone(),
    }
}
fn flatten(data: &Snapshot) -> Vec<TreeItem> {
    let mut result = vec![];
    let mut stack = data
        .roots
        .iter()
        .rev()
        .map(|key| (key.as_str(), true))
        .collect::<Vec<_>>();
    while let Some((key, parent_enabled)) = stack.pop() {
        let record = &data.records[key];
        let enabled = parent_enabled && record.enabled;
        result.push(item(record, enabled));
        if record.expanded {
            stack.extend(
                record
                    .children
                    .iter()
                    .rev()
                    .map(|key| (key.as_str(), enabled)),
            );
        }
    }
    result
}
impl TreeModel {
    pub fn new() -> Self {
        Self::with_options(vec![], TreeOptions::default()).expect("Valid default tree options")
    }
    pub fn from_nodes(nodes: Vec<TreeNode>) -> Result<Self, Error> {
        Self::with_options(nodes, TreeOptions::default())
    }
    pub fn with_options(nodes: Vec<TreeNode>, options: TreeOptions) -> Result<Self, Error> {
        if !options.row_height.is_finite()
            || options.row_height <= 0.
            || !options.indentation.is_finite()
            || options.indentation < 0.
            || options.visible_rows == 0
            || !(options.visible_rows as f64 * options.row_height).is_finite()
        {
            return Err(Error(
                "Invalid tree row height, visible rows or indentation".into(),
            ));
        }
        let list = ListModel::new(vec![], options.row_height).options(crate::ListOptions {
            selection: options.selection,
            visible_rows: options.visible_rows,
            overscan_rows: options.overscan_rows,
            ..Default::default()
        })?;
        let tree = Self(Rc::new(Inner {
            data: State::new(Rc::new(Snapshot::default())),
            list,
            on_load: RefCell::new(None),
            options,
            on_expand: RefCell::new(None),
            notifying: Cell::new(false),
        }));
        tree.set_nodes(nodes)?;
        Ok(tree)
    }
    /// Replace with a validated fresh snapshot. Existing requests/lifetimes and
    /// expansion caches retire; new branches start collapsed.
    pub fn set_nodes(&self, nodes: Vec<TreeNode>) -> Result<(), Error> {
        crate::state::check_write();
        let mut data = Snapshot::default();
        data.roots = register(nodes, None, 0, &mut data.records)?;
        self.publish(data)
    }
    pub fn set_tree(&self, nodes: &[TreeNode]) -> Result<(), Error> {
        self.set_nodes(nodes.to_vec())
    }
    fn publish(&self, mut data: Snapshot) -> Result<(), Error> {
        let visible = flatten(&data);
        if !(visible.len() as f64 * self.0.list.row_height()).is_finite() {
            return Err(Error("Tree extent must be finite".into()));
        }
        let tokens = visible
            .iter()
            .map(|i| (i.key.clone(), data.records[&i.key].token))
            .collect();
        let old = self.0.data.get();
        let enabled_keys = visible
            .iter()
            .filter(|i| i.enabled)
            .map(|i| i.key.clone())
            .collect::<BTreeSet<_>>();
        let project = |key: &str| {
            let mut current = Some(key);
            let mut result = None;
            while let Some(key) = current {
                if enabled_keys.contains(key) {
                    result = Some(key.to_owned());
                    break;
                }
                current = data
                    .records
                    .get(key)
                    .or_else(|| old.records.get(key))
                    .and_then(|r| r.parent.as_deref());
            }
            result
        };
        let selected = self
            .selected()
            .iter()
            .filter_map(|key| project(key))
            .collect();
        let previous_cursor = self.cursor();
        let cursor = previous_cursor.as_deref().and_then(project).or_else(|| {
            previous_cursor.as_ref().and_then(|key| {
                let start = self
                    .0
                    .list
                    .index_of(key)
                    .unwrap_or(0)
                    .min(visible.len().saturating_sub(1));
                (start..visible.len())
                    .chain((0..start).rev())
                    .find(|i| visible[*i].enabled)
                    .map(|i| visible[i].key.clone())
            })
        });
        let anchor = self.0.list.anchor().as_deref().and_then(project);
        let rows = visible
            .into_iter()
            .map(|i| ListItem::new(i.key, i.label).enabled(i.enabled))
            .collect();
        data.revision = super::list::next_token();
        crate::state::batch(|| {
            self.0.list.replace_items(rows, Some(&tokens))?;
            self.0.list.restore_selection(selected, cursor, anchor);
            self.0.data.set(Rc::new(data));
            Ok(())
        })?;
        drop(old);
        Ok(())
    }
    pub fn known_count(&self) -> usize {
        self.0.data.get().records.len()
    }
    pub fn visible_count(&self) -> usize {
        self.0.list.len()
    }
    pub fn flatten(&self) -> Vec<TreeItem> {
        flatten(&self.0.data.get())
    }
    pub fn item(&self, key: &str) -> Option<TreeItem> {
        let data = self.0.data.get();
        data.records.get(key).map(|r| item(r, enabled(&data, key)))
    }
    pub fn expanded(&self) -> BTreeSet<String> {
        self.0
            .data
            .get()
            .records
            .values()
            .filter(|r| r.expanded)
            .map(|r| r.key.clone())
            .collect()
    }
    pub fn last_failure(&self) -> Option<TreeFailure> {
        self.0.data.get().failure.clone()
    }
    pub fn on_load(self, handler: impl Fn(TreeLoad) + 'static) -> Self {
        crate::state::check_write();
        let old = self.0.on_load.replace(Some(Rc::new(handler)));
        drop(old);
        self
    }
    pub fn pending_load(&self, key: &str) -> Option<TreeLoad> {
        let data = self.0.data.get();
        let record = data.records.get(key)?;
        Some(TreeLoad {
            owner: Rc::downgrade(&self.0),
            key: key.into(),
            token: record.token,
            request: record.request?,
        })
    }
    pub fn begin_load(&self, key: &str) -> Result<TreeLoad, Error> {
        crate::state::check_write();
        let mut data = (*self.0.data.get()).clone();
        if !enabled(&data, key) {
            return Err(Error(format!("Tree node is missing or disabled: {key}")));
        }
        let record = data.records.get_mut(key).unwrap();
        let opening = !record.expanded;
        record.load = TreeLoadState::Loading;
        record.expanded = true;
        record.request = Some(super::list::next_token());
        if data.failure.as_ref().is_some_and(|f| f.key == key) {
            data.failure = None;
        }
        let record = &data.records[key];
        let request = TreeLoad {
            owner: Rc::downgrade(&self.0),
            key: key.into(),
            token: record.token,
            request: record.request.unwrap(),
        };
        self.publish(data)?;
        if opening {
            self.emit_expand(key, request.token, true);
        }
        Ok(request)
    }
    fn dispatch(&self, request: TreeLoad) -> Result<(), Error> {
        if !request.is_current() {
            return Ok(());
        }
        let loader = self
            .0
            .data
            .get()
            .records
            .get(request.key())
            .and_then(|r| r.loader.clone());
        if let Some(loader) = loader {
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                crate::state::read_only(|| loader())
            }))
            .unwrap_or_else(|_| Err(Error("Tree loader panicked".into())));
            request.complete(result)?;
        } else {
            let handler = self.0.on_load.borrow().clone();
            if let Some(handler) = handler {
                if std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    handler(request.clone())
                }))
                .is_err()
                {
                    let error = Error("Tree load handler panicked".into());
                    request.complete(Err(error.clone()))?;
                    return Err(error);
                }
                if let Some(record) = self
                    .0
                    .data
                    .get()
                    .records
                    .get(request.key())
                    .filter(|r| r.token == request.token)
                    && let TreeLoadState::Failed(error) = &record.load
                {
                    return Err(Error(error.clone()));
                }
            }
        }
        Ok(())
    }
    pub fn expand(&self, key: &str) -> Result<bool, Error> {
        crate::state::check_write();
        let mut data = (*self.0.data.get()).clone();
        let record = data
            .records
            .get(key)
            .ok_or_else(|| Error(format!("Unknown tree node: {key}")))?;
        if !enabled(&data, key) || !record.can_expand() {
            return Ok(false);
        }
        if record.load == TreeLoadState::Unloaded {
            let request = self.begin_load(key)?;
            self.dispatch(request)?;
            return Ok(true);
        }
        let token = record.token;
        let changed = !record.expanded;
        let failed = if let TreeLoadState::Failed(error) = &record.load {
            Some(error.clone())
        } else {
            None
        };
        if changed {
            data.records.get_mut(key).unwrap().expanded = true;
            self.publish(data)?;
            self.emit_expand(key, token, true);
        }
        if let Some(error) = failed {
            return Err(Error(error));
        }
        Ok(changed)
    }
    pub fn collapse(&self, key: &str) -> Result<bool, Error> {
        crate::state::check_write();
        let mut data = (*self.0.data.get()).clone();
        let record = data
            .records
            .get_mut(key)
            .ok_or_else(|| Error(format!("Unknown tree node: {key}")))?;
        if !record.expanded {
            return Ok(false);
        }
        let token = record.token;
        record.expanded = false;
        self.publish(data)?;
        self.emit_expand(key, token, false);
        Ok(true)
    }
    pub fn toggle(&self, key: &str) -> Result<bool, Error> {
        if self.item(key).is_some_and(|i| i.expanded) {
            self.collapse(key)
        } else {
            self.expand(key)
        }
    }
    pub fn retry(&self, key: &str) -> Result<bool, Error> {
        let request = self.begin_load(key)?;
        self.dispatch(request)?;
        Ok(true)
    }
    fn fail(&self, key: &str, error: Error) -> Result<bool, Error> {
        let error = message(error);
        let mut data = (*self.0.data.get()).clone();
        let record = data.records.get_mut(key).unwrap();
        record.load = TreeLoadState::Failed(error.0.clone());
        record.request = None;
        data.failure = Some(TreeFailure {
            key: key.into(),
            message: error.0.clone(),
        });
        self.publish(data)?;
        Err(error)
    }
    fn complete(
        &self,
        request: &TreeLoad,
        result: Result<Vec<TreeNode>, Error>,
    ) -> Result<bool, Error> {
        if !request.is_current() {
            return Ok(false);
        }
        let nodes = match result {
            Ok(nodes) => nodes,
            Err(error) => {
                return if request.is_current() {
                    self.fail(request.key(), error)
                } else {
                    Ok(false)
                };
            }
        };
        let mut data = (*self.0.data.get()).clone();
        let parent = data.records.get_mut(request.key()).unwrap();
        let mut old = Rc::unwrap_or_clone(std::mem::take(&mut parent.children));
        let depth = parent.depth + 1;
        while let Some(key) = old.pop() {
            if let Some(record) = data.records.remove(&key) {
                old.extend(Rc::unwrap_or_clone(record.children));
            }
        }
        let children = match register(nodes, Some(request.key.clone()), depth, &mut data.records) {
            Ok(children) => children,
            Err(error) => {
                return if request.is_current() {
                    self.fail(request.key(), error)
                } else {
                    Ok(false)
                };
            }
        };
        if !request.is_current() {
            return Ok(false);
        }
        let parent = data.records.get_mut(request.key()).unwrap();
        let was_expanded = parent.expanded;
        parent.children = Rc::new(children);
        parent.load = TreeLoadState::Loaded;
        parent.request = None;
        if !parent.can_expand() {
            parent.expanded = false;
        }
        let closed = was_expanded && !parent.expanded;
        if data
            .failure
            .as_ref()
            .is_some_and(|f| f.key == request.key || !data.records.contains_key(&f.key))
        {
            data.failure = None;
        }
        self.publish(data)?;
        if closed {
            self.emit_expand(request.key(), request.token, false);
        }
        Ok(true)
    }
    /// Publish one branch through checked completion; supersedes its pending load.
    pub fn set_children(&self, key: &str, nodes: Vec<TreeNode>) -> Result<bool, Error> {
        let request = self.begin_load(key)?;
        request.complete(Ok(nodes))
    }
    fn note_error(&self, key: &str, error: Error) {
        let mut data = (*self.0.data.get()).clone();
        let failure = TreeFailure {
            key: key.into(),
            message: error.0,
        };
        if data.failure.as_ref() != Some(&failure) {
            data.failure = Some(failure);
            self.publish(data).expect("Validated tree error update");
        }
    }
    pub fn view(&self) -> View {
        let data = self.0.data.get();
        let tree = self.clone();
        let mut view = self.0.list.view_with(move |row| {
            let record = &data.records[&row.key];
            let info = item(record, enabled(&data, &row.key));
            let action = |retry| {
                let label = if retry {
                    "Retry"
                } else if info.expanded {
                    "▾"
                } else {
                    "▸"
                };
                let mut control = View::button(label, || {})
                    .padding(2.)
                    .width(if retry { 60 } else { 26 })
                    .shrink(0.)
                    .key(if retry { "$retry" } else { "$disclosure" })
                    .accessible_label(format!(
                        "{} {}",
                        if retry {
                            "Retry loading"
                        } else if info.expanded {
                            "Collapse"
                        } else {
                            "Expand"
                        },
                        info.accessible_label
                    ));
                Rc::make_mut(&mut control.0).tree_action = Some(TreeAction {
                    model: tree.clone(),
                    key: row.key.clone(),
                    token: record.token,
                    retry,
                });
                control
            };
            let control = if info.expandable {
                action(false)
            } else {
                View::spacer().width(26).grow(0.)
            };
            let status = if info.loading {
                " — Loading…".into()
            } else if let Some(error) = &info.error {
                format!(" — {error}")
            } else {
                String::new()
            };
            let label = View::label(format!("{}{status}", info.label))
                .grow(1.)
                .accessible_label(format!("{}{status}", info.accessible_label))
                .label_style(if info.error.is_some() {
                    crate::LabelStyle::Error
                } else {
                    crate::LabelStyle::Standard
                });
            let mut children = vec![control, label];
            if info.error.is_some() {
                children.push(action(true));
            }
            View::row(children)
                .align(crate::Align::Center)
                .margin(Insets {
                    left: info.depth as f64 * tree.0.options.indentation,
                    ..Default::default()
                })
                .tooltip(info.error.unwrap_or_default())
        });
        let root = Rc::make_mut(&mut view.0);
        root.tree = Some(self.clone());
        for row in &mut root.children {
            if let Some((_, key, _, _)) = &row.0.collection_row {
                let info = self.item(key);
                let row = Rc::make_mut(&mut row.0);
                if let Some(info) = &info {
                    row.accessible = info.accessible_label.clone();
                }
                row.tree_item = info;
            }
        }
        view
    }
}
impl Default for TreeModel {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TreeOptions {
    pub row_height: f64,
    pub visible_rows: usize,
    pub overscan_rows: usize,
    pub indentation: f64,
    pub selection: crate::ListSelectionMode,
}
impl Default for TreeOptions {
    fn default() -> Self {
        Self {
            row_height: 36.,
            visible_rows: 8,
            overscan_rows: 2,
            indentation: 18.,
            selection: crate::ListSelectionMode::Single,
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TreeExpansion {
    pub key: String,
    pub expanded: bool,
}

impl TreeModel {
    pub fn tree_options(&self) -> TreeOptions {
        let mut options = self.0.options;
        let list = self.0.list.list_options();
        options.selection = list.selection;
        options.visible_rows = list.visible_rows;
        options.overscan_rows = list.overscan_rows;
        options
    }
    pub fn multiple(self) -> Self {
        self.0.list.clone().multiple();
        self
    }
    pub fn selected(&self) -> BTreeSet<String> {
        self.0.list.selected()
    }
    pub fn selection_count(&self) -> usize {
        self.0.list.selection_count()
    }
    pub fn cursor(&self) -> Option<String> {
        self.0.list.cursor()
    }
    pub fn select(&self, key: &str, extend: bool) -> bool {
        self.0.list.select(key, extend)
    }
    pub fn select_with(&self, key: &str, operation: crate::ListSelection) -> bool {
        self.0.list.select_with(key, operation)
    }
    pub fn clear_selection(&self) {
        self.0.list.clear_selection();
    }
    pub fn select_all(&self) -> bool {
        self.0.list.select_all()
    }
    pub fn item_token(&self, key: &str) -> Option<u64> {
        self.0.data.get().records.get(key).map(|r| r.token)
    }
    pub fn on_activate(self, callback: impl Fn(String) + 'static) -> Self {
        self.0.list.clone().on_activate(callback);
        self
    }
    pub fn on_activate_with_model(self, callback: impl Fn(&TreeModel, String) + 'static) -> Self {
        let weak = Rc::downgrade(&self.0);
        self.on_activate(move |key| {
            if let Some(owner) = weak.upgrade() {
                callback(&TreeModel(owner), key);
            }
        })
    }
    pub fn on_expand(self, callback: impl Fn(TreeExpansion) + 'static) -> Self {
        crate::state::check_write();
        let old = self.0.on_expand.replace(Some(Rc::new(callback)));
        drop(old);
        self
    }
    fn emit_expand(&self, key: &str, token: u64, expanded: bool) {
        if self.item_token(key) != Some(token)
            || self.item(key).is_none_or(|i| i.expanded != expanded)
            || self.0.notifying.replace(true)
        {
            return;
        }
        struct Guard<'a>(&'a Cell<bool>);
        impl Drop for Guard<'_> {
            fn drop(&mut self) {
                self.0.set(false);
            }
        }
        let _guard = Guard(&self.0.notifying);
        let callback = self.0.on_expand.borrow().clone();
        if let Some(callback) = callback {
            callback(TreeExpansion {
                key: key.into(),
                expanded,
            });
        }
    }
    /// Activate a visible enabled item. A custom callback owns activation;
    /// otherwise branches toggle/retry and leaves have no default action.
    pub fn activate(&self, key: &str) -> Result<bool, Error> {
        crate::state::check_write();
        let Some(info) = self
            .item(key)
            .filter(|i| i.enabled && self.0.list.index_of(key).is_some())
        else {
            return Ok(false);
        };
        if self.0.list.has_activation() {
            return Ok(self.0.list.activate(key));
        }
        if info.error.is_some() {
            self.retry(key)
        } else if info.expandable {
            self.toggle(key)
        } else {
            Ok(false)
        }
    }
    pub(crate) fn ui_activate(&self, key: &str) -> bool {
        let token = self.item_token(key);
        match self.activate(key) {
            Ok(handled) => handled,
            Err(error) => {
                if self.item_token(key) == token {
                    self.note_error(key, error);
                }
                true
            }
        }
    }
    fn ui_result(&self, key: &str, token: Option<u64>, result: Result<bool, Error>) -> bool {
        match result {
            Ok(changed) => changed,
            Err(error) => {
                if self.item_token(key) == token {
                    self.note_error(key, error);
                }
                true
            }
        }
    }
    pub(crate) fn ui_disclose(&self, key: &str, token: u64, retry: bool) -> bool {
        if self.item_token(key) != Some(token) {
            return false;
        }
        self.ui_result(
            key,
            Some(token),
            if retry {
                self.retry(key)
            } else {
                self.toggle(key)
            },
        )
    }
    pub(crate) fn ui_expansion(&self, key: &str, expanded: bool) -> bool {
        self.ui_result(
            key,
            self.item_token(key),
            if expanded {
                self.expand(key)
            } else {
                self.collapse(key)
            },
        )
    }
    pub(crate) fn child(&self, key: &str) -> Option<String> {
        let data = self.0.data.get();
        data.records
            .get(key)?
            .children
            .iter()
            .find(|key| enabled(&data, key) && self.0.list.index_of(key).is_some())
            .cloned()
    }
    pub(crate) fn parent(&self, key: &str) -> Option<String> {
        let data = self.0.data.get();
        let mut parent = data.records.get(key)?.parent.as_deref();
        while let Some(key) = parent {
            if enabled(&data, key) && self.0.list.index_of(key).is_some() {
                return Some(key.into());
            }
            parent = data.records.get(key)?.parent.as_deref();
        }
        None
    }
    pub(crate) fn index_of(&self, key: &str) -> Option<usize> {
        self.0.list.index_of(key)
    }
    /// Open known ancestors, then reveal a logical item without selecting it.
    pub fn reveal(&self, key: &str) -> Result<bool, Error> {
        crate::state::check_write();
        let data = self.0.data.get();
        let Some(target) = data.records.get(key) else {
            return Ok(false);
        };
        let token = target.token;
        if let Some(index) = self.index_of(key) {
            self.0.list.reveal(index);
            return Ok(true);
        }
        let mut parents = vec![];
        let mut parent = target.parent.as_deref();
        while let Some(key) = parent {
            let record = &data.records[key];
            parents.push((key.to_owned(), record.token));
            parent = record.parent.as_deref();
        }
        drop(data);
        for (key, token) in parents.into_iter().rev() {
            if self.item_token(&key) != Some(token) {
                return Ok(false);
            }
            self.expand(&key)?;
        }
        if self.item_token(key) != Some(token) {
            return Ok(false);
        }
        if let Some(index) = self.index_of(key) {
            self.0.list.reveal(index);
            Ok(true)
        } else {
            Ok(false)
        }
    }
}

/// A visible logical outline row; independent of viewport realization.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TreeRowInfo {
    pub item: TreeItem,
    pub index: usize,
    pub token: u64,
    pub selected: bool,
    pub activatable: bool,
}
impl TreeModel {
    pub fn row(&self, key: &str) -> Option<TreeRowInfo> {
        let index = self.0.list.index_of(key)?;
        let item = self.item(key)?;
        let activatable = item.enabled
            && (self.0.list.has_activation() || item.expandable || item.error.is_some());
        Some(TreeRowInfo {
            token: self.item_token(key)?,
            selected: self.0.list.is_selected(key),
            item,
            index,
            activatable,
        })
    }
    pub fn rows(&self, start: usize, count: usize) -> Vec<TreeRowInfo> {
        let end = start.saturating_add(count).min(self.visible_count());
        (start.min(end)..end)
            .filter_map(|i| self.0.list.item(i).and_then(|r| self.row(&r.key)))
            .collect()
    }
    pub fn disclosed_count(&self, key: &str) -> usize {
        let data = self.0.data.get();
        data.records
            .get(key)
            .filter(|r| r.expanded && self.0.list.index_of(key).is_some())
            .map_or(0, |r| r.children.len())
    }
    pub fn disclosed_rows(&self, key: &str, start: usize, count: usize) -> Vec<TreeRowInfo> {
        let data = self.0.data.get();
        let Some(record) = data
            .records
            .get(key)
            .filter(|r| r.expanded && self.0.list.index_of(key).is_some())
        else {
            return vec![];
        };
        let end = start.saturating_add(count).min(record.children.len());
        record.children[start.min(end)..end]
            .iter()
            .filter_map(|key| self.row(key))
            .collect()
    }
    pub fn row_snapshot(&self) -> crate::ListSnapshot {
        self.0.list.snapshot()
    }
    pub fn version(&self) -> crate::CollectionVersion {
        crate::CollectionVersion {
            content: self.0.data.version(),
            ..self.0.list.version()
        }
    }
    pub fn set_selection(&self, keys: &[String]) -> Result<(), Error> {
        crate::state::check_write();
        let selected = keys.iter().cloned().collect::<BTreeSet<_>>();
        if self.tree_options().selection == crate::ListSelectionMode::Single && selected.len() > 1
            || selected
                .iter()
                .any(|key| self.row(key).is_none_or(|r| !r.item.enabled))
        {
            return Err(Error(
                "Tree selection must contain visible enabled rows and respect the selection mode"
                    .into(),
            ));
        }
        let cursor = keys.last().cloned().or_else(|| self.cursor());
        let anchor = keys.first().cloned().or_else(|| self.0.list.anchor());
        crate::state::batch(|| self.0.list.restore_selection(selected, cursor, anchor));
        Ok(())
    }
}

/// Immutable direct-child membership without retaining loaders or allocating row objects.
#[derive(Clone)]
pub struct TreeChildrenSnapshot {
    rows: crate::ListSnapshot,
    keys: Rc<Vec<String>>,
}
impl TreeChildrenSnapshot {
    pub fn len(&self) -> usize {
        self.keys.len()
    }
    pub fn is_empty(&self) -> bool {
        self.keys.is_empty()
    }
    pub fn key(&self, index: usize) -> Option<&str> {
        self.keys.get(index).map(String::as_str)
    }
    pub fn token(&self, index: usize) -> Option<u64> {
        self.rows.item_token(self.key(index)?)
    }
}
impl TreeModel {
    pub fn disclosed_snapshot(&self, key: &str) -> Option<TreeChildrenSnapshot> {
        let data = self.0.data.get();
        let record = data
            .records
            .get(key)
            .filter(|r| r.expanded && self.0.list.index_of(key).is_some())?;
        Some(TreeChildrenSnapshot {
            rows: self.0.list.snapshot(),
            keys: record.children.clone(),
        })
    }
}
