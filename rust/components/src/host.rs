use crate::{
    Kind, View,
    state::{Observer, Tracking},
};
use std::{
    cell::{Cell, RefCell},
    collections::{HashMap, HashSet},
    fmt,
    rc::Rc,
    sync::atomic::{AtomicU64, Ordering},
};

/// Opaque mounted identity. Removed IDs are never recycled, including across hosts.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct NodeId(u64);
impl fmt::Display for NodeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// Owned inspection data; never a borrowed pointer into a mutable tree.
#[derive(Clone, Debug)]
pub struct Node {
    pub id: NodeId,
    pub parent: Option<NodeId>,
    pub depth: usize,
    pub kind: Kind,
    pub key: Option<String>,
    pub text: String,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Error(pub String);
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}
impl std::error::Error for Error {}

#[derive(Clone, Hash, PartialEq, Eq)]
enum Segment {
    Key(String),
    Index(usize),
}
type Path = Vec<(Segment, Kind)>;
#[derive(Default)]
struct Mounted {
    identities: HashMap<Path, NodeId>,
    nodes: Vec<Node>,
    actions: HashMap<NodeId, Rc<dyn Fn()>>,
}
struct Inner {
    builder: Box<dyn Fn() -> View>,
    observer: Rc<Observer>,
    mounted: RefCell<Mounted>,
    building: Cell<bool>,
    generation: Cell<u64>,
}

/// Main-thread-style reactive host, independent of windowing or rendering.
/// State updates coalesce until `refresh`/`snapshot`/`activate`. Builders are pure;
/// actions run outside all host borrows and may update state or query this host.
#[derive(Clone)]
pub struct Host(Rc<Inner>);
impl Host {
    pub fn new(builder: impl Fn() -> View + 'static) -> Self {
        Self(Rc::new(Inner {
            builder: Box::new(builder),
            observer: Observer::new(),
            mounted: RefCell::new(Mounted::default()),
            building: Cell::new(false),
            generation: Cell::new(0),
        }))
    }
    /// Connect invalidation to a redraw scheduler. Repeated writes coalesce.
    pub fn on_invalidate(&self, wake: impl Fn() + 'static) {
        self.0.observer.set_wake(Rc::new(wake));
    }
    pub fn generation(&self) -> u64 {
        self.0.generation.get()
    }
    pub fn refresh(&self) -> Result<bool, Error> {
        if self.0.building.replace(true) {
            return Err(Error("Host cannot rebuild during view construction".into()));
        }
        struct Building<'a>(&'a Cell<bool>);
        impl Drop for Building<'_> {
            fn drop(&mut self) {
                self.0.set(false);
            }
        }
        let _building = Building(&self.0.building);
        if !self.0.observer.dirty.get() {
            return Ok(false);
        }
        let tracking = Tracking::begin();
        let view = (self.0.builder)();
        let deps = tracking.finish();
        // Stage the complete tree before publishing either identity or subscriptions.
        let mut next = Mounted::default();
        mount(
            &view,
            Vec::new(),
            None,
            0,
            &self.0.mounted.borrow(),
            &mut next,
        )?;
        let epoch = self.0.observer.epoch.get().wrapping_add(1);
        for dep in deps {
            dep.subscribe(&self.0.observer, epoch);
        }
        self.0.observer.epoch.set(epoch);
        let old = self.0.mounted.replace(next);
        self.0
            .generation
            .set(self.0.generation.get().wrapping_add(1));
        self.0.observer.dirty.set(false);
        drop(old);
        Ok(true)
    }
    pub fn snapshot(&self) -> Result<Vec<Node>, Error> {
        self.refresh()?;
        Ok(self.0.mounted.borrow().nodes.clone())
    }
    /// Dispatch a current action. A stale, removed or foreign ID returns false.
    pub fn activate(&self, id: NodeId) -> Result<bool, Error> {
        self.refresh()?;
        let action = self.0.mounted.borrow().actions.get(&id).cloned();
        if let Some(action) = action {
            action();
            Ok(true)
        } else {
            Ok(false)
        }
    }
}

fn mount(
    view: &View,
    path: Path,
    parent: Option<NodeId>,
    index: usize,
    old: &Mounted,
    next: &mut Mounted,
) -> Result<(), Error> {
    static NEXT_ID: AtomicU64 = AtomicU64::new(1);
    let d = &view.0;
    let segment = match &d.key {
        Some(key) if key.is_empty() => return Err(Error("Keys must not be empty".into())),
        Some(key) => Segment::Key(key.clone()),
        None => Segment::Index(index),
    };
    let mut identity = path;
    identity.push((segment, d.kind));
    let id = old
        .identities
        .get(&identity)
        .copied()
        .unwrap_or_else(|| NodeId(NEXT_ID.fetch_add(1, Ordering::Relaxed)));
    next.identities.insert(identity.clone(), id);
    next.nodes.push(Node {
        id,
        parent,
        depth: identity.len() - 1,
        kind: d.kind,
        key: d.key.clone(),
        text: d.text.clone(),
    });
    if let Some(action) = &d.action {
        next.actions.insert(id, action.clone());
    }
    let mut keys = HashSet::new();
    for (index, child) in d.children.iter().enumerate() {
        if let Some(key) = &child.0.key
            && !keys.insert(key)
        {
            return Err(Error(format!("Duplicate sibling key: {key}")));
        }
        mount(child, identity.clone(), Some(id), index, old, next)?;
    }
    Ok(())
}
