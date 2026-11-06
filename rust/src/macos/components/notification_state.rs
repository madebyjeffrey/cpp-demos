//! Bounded semantic snapshots: mounted nodes plus logical objects an AX client requested.
use super::{
    collection_accessibility::Row,
    table_accessibility::{Entry, Part},
    *,
};
use std::{
    collections::{HashMap, hash_map::RandomState},
    hash::{BuildHasher, Hash, Hasher},
};
use ui::{Kind, Node, NodeId, Value};
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(super) enum Key {
    Node(NodeId),
    Row(NodeId, u64),
    Table((NodeId, u8, u64, u64)),
}
#[derive(Clone)]
pub(super) enum Target {
    Node(NodeId),
    Row(NodeId, String, u64),
    Table(NodeId, Part),
}
impl Target {
    pub(super) fn key(&self) -> Key {
        match self {
            Self::Node(n) => Key::Node(*n),
            Self::Row(n, _, t) => Key::Row(*n, *t),
            Self::Table(n, p) => Key::Table(p.identity(*n)),
        }
    }
    pub(super) fn object(&self, m: &Mounted) -> Option<Retained<AnyObject>> {
        match self {
            Self::Node(id) => Some(any(accessibility::element(m, *id))),
            Self::Row(id, key, token) => Some(any(collection_accessibility::row(
                owner(m)?,
                *id,
                key.clone(),
                *token,
            ))),
            Self::Table(id, part) => Some(table_accessibility::entry(owner(m)?, *id, part.clone())),
        }
    }
}
fn owner(m: &Mounted) -> Option<Weak<Mounted>> {
    m.surface
        .upgrade()?
        .state
        .components
        .borrow()
        .as_ref()
        .map(Rc::downgrade)
}
fn any<T: objc2::Message>(value: Retained<T>) -> Retained<AnyObject> {
    unsafe { Retained::cast_unchecked(value) }
}
pub(super) fn target(object: &AnyObject) -> Option<Target> {
    if let Some(n) = object.downcast_ref::<accessibility::Element>() {
        Some(Target::Node(n.ivars().id))
    } else if let Some(row) = object.downcast_ref::<Row>() {
        Some(row.target())
    } else {
        object.downcast_ref::<Entry>().map(Entry::target)
    }
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) struct Selection {
    pub kind: u8,
    pub identity: u64,
    pub version: u64,
}
#[derive(Clone, PartialEq, Eq, Default)]
pub(super) struct Attributes {
    pub value: u64,
    pub title: u64,
    pub meta: u64,
    pub bounds: Option<[u64; 4]>,
    pub selection: Option<Selection>,
    pub rows: Option<usize>,
    pub content: Option<(u64, u64)>,
    pub expanded: Option<bool>,
}
#[derive(Clone)]
pub(super) struct Item {
    pub target: Target,
    pub attrs: Option<Attributes>,
    pub retired: bool,
}
#[derive(Default)]
pub(super) struct Snapshot {
    pub items: HashMap<Key, Item>,
    pub focus: Option<Key>,
    pub text: Option<Text>,
    pub context: Option<[u64; 5]>,
    pub initialized: bool,
}
#[derive(Clone, PartialEq, Eq)]
pub(super) struct Text {
    pub id: NodeId,
    pub selection: ui::Selection,
    pub marked: Option<std::ops::Range<usize>>,
}
#[derive(Default)]
pub(super) struct Notifications {
    pub snapshot: Snapshot,
    pub(super) hashes: RandomState,
}
impl Notifications {
    pub(super) fn fork(&self) -> Self {
        Self {
            snapshot: Snapshot::default(),
            hashes: self.hashes.clone(),
        }
    }
    fn hash(&self, value: impl Hash) -> u64 {
        self.hashes.hash_one(value)
    }
    fn value(&self, n: &Node) -> u64 {
        let mut h = self.hashes.build_hasher();
        // Fingerprints only: never retain text/passwords in notification state or diagnostics.
        match &n.value {
            Value::Text(t) => t.hash(&mut h),
            Value::Number(v) => v.to_bits().hash(&mut h),
            Value::Bool(v) => v.hash(&mut h),
            Value::Check(v) => format!("{v:?}").hash(&mut h),
            Value::None => n.text.hash(&mut h),
        }
        if let Some(s) = &n.stepper {
            s.value.hash(&mut h);
            s.min.hash(&mut h);
            s.max.hash(&mut h)
        }
        n.color_well.map(|c| c.0).hash(&mut h);
        if let Some(c) = &n.combo {
            c.query.hash(&mut h);
            c.selection.hash(&mut h);
            c.loading.hash(&mut h)
        }
        if let Some(v) = &n.validation {
            format!("{v:?}").hash(&mut h)
        }
        if let Some(i) = &n.menu_item {
            i.checked.hash(&mut h)
        }
        if let Some(t) = &n.tabs {
            t.selected.hash(&mut h)
        }
        h.finish()
    }
    fn bounds(&self, r: ui::Rect) -> Option<[u64; 4]> {
        (r.width > 0. && r.height > 0.).then_some([
            r.x.to_bits(),
            r.y.to_bits(),
            r.width.to_bits(),
            r.height.to_bits(),
        ])
    }
    pub(super) fn capture(&self, target: Target, m: &Mounted, nodes: &[Node]) -> Item {
        let root_id = match &target {
            Target::Node(id) | Target::Row(id, ..) | Target::Table(id, _) => *id,
        };
        let Some(n) = nodes.iter().find(|n| n.id == root_id) else {
            return Item {
                target,
                attrs: None,
                retired: true,
            };
        };
        let active = !n.hidden && control_accessibility::readable(m, n, nodes);
        let mut attrs = Attributes::default();
        let mut retired = false;
        match &target {
            Target::Node(_) => {
                if !exposed(m, n, nodes) {
                    return Item {
                        target,
                        attrs: None,
                        retired: false,
                    };
                }
                attrs.value = self.value(n);
                attrs.title = self.hash(&n.label);
                attrs.bounds = self.bounds(n.clip);
                attrs.meta = self.hash((
                    n.parent,
                    n.enabled,
                    n.read_only,
                    &n.help,
                    n.menu.as_ref().map(|i| (i.id, i.expanded)),
                    n.stepper
                        .as_ref()
                        .map(|s| (s.can_increment, s.can_decrement)),
                ));
                attrs.expanded = n
                    .menu_item
                    .as_ref()
                    .filter(|i| i.submenu)
                    .map(|i| i.expanded)
                    .or_else(|| n.menu.as_ref().filter(|i| i.trigger).map(|i| i.expanded))
                    .or_else(|| n.combo.as_ref().map(|c| c.open));
                if let Some(t) = &n.table_root {
                    let v = t.version();
                    attrs.content = Some((v.identity, v.content));
                    attrs.rows = Some(t.row_count());
                    attrs.selection = Some(Selection {
                        kind: if t.options().selection_mode == ui::TableSelectionMode::Rows {
                            0
                        } else {
                            1
                        },
                        identity: v.identity,
                        version: v.selection,
                    });
                } else if let Some(list) = &n.collection {
                    let v = n
                        .tree_model
                        .as_ref()
                        .map_or_else(|| list.version(), |t| t.version());
                    attrs.content = Some((v.identity, v.content));
                    attrs.rows = Some(list.len());
                    attrs.selection = Some(Selection {
                        kind: 0,
                        identity: v.identity,
                        version: v.selection,
                    });
                } else if matches!(n.kind, Kind::Tabs | Kind::RadioGroup | Kind::Segmented) {
                    let choices: Vec<_> = nodes
                        .iter()
                        .filter(|c| {
                            c.parent == Some(n.id)
                                || n.kind == Kind::Tabs
                                    && c.kind == Kind::Tab
                                    && accessibility::tab_group(c, nodes) == Some(n.id)
                        })
                        .map(|c| (c.id, c.value.selected()))
                        .collect();
                    attrs.selection = Some(Selection {
                        kind: 2,
                        identity: self.hash(n.id),
                        version: self.hash(choices),
                    });
                }
                if let Some(c) = &n.combo {
                    attrs.selection = Some(Selection {
                        kind: 2,
                        identity: self.hash(n.id),
                        version: self.hash((&c.highlighted, c.open)),
                    });
                } else if n.combo_popup.is_some() {
                    attrs.selection = Some(Selection {
                        kind: 2,
                        identity: self.hash(n.id),
                        version: self.hash(n.collection.as_ref().and_then(|l| l.cursor())),
                    });
                }
            }
            Target::Row(_, key, token) => {
                let Some(list) = &n.collection else {
                    return Item {
                        target,
                        attrs: None,
                        retired: true,
                    };
                };
                if let Some(t) = &n.tree_model {
                    retired = t.item_token(key) != Some(*token)
                } else {
                    retired = list.item_token(key) != Some(*token)
                }
                if retired || !active || list.item_token(key) != Some(*token) {
                    return Item {
                        target,
                        attrs: None,
                        retired,
                    };
                }
                let index = list.index_of(key).unwrap();
                let item = list.item(index).unwrap();
                attrs.value = self.hash(&item.cells);
                attrs.title = attrs.value;
                attrs.meta = self.hash((index, n.enabled && item.enabled));
                attrs.bounds = self.bounds(
                    ui::Rect::new(
                        n.bounds.x,
                        n.bounds.y + index as f64 * list.row_height() - list.offset(),
                        n.bounds.width,
                        list.row_height(),
                    )
                    .intersection(n.clip),
                );
                if let Some(info) = n.tree_model.as_ref().and_then(|t| t.row(key)) {
                    attrs.value = self.hash((&item.cells, info.item.loading, &info.item.error));
                    attrs.title = self.hash(&info.item.accessible_label);
                    attrs.expanded = info.item.expandable.then_some(info.item.expanded);
                }
            }
            Target::Table(_, part) => {
                let Some(t) = &n.table_root else {
                    return Item {
                        target,
                        attrs: None,
                        retired: true,
                    };
                };
                retired = !part.valid(t);
                if retired || !active {
                    return Item {
                        target,
                        attrs: None,
                        retired,
                    };
                }
                let snapshot = t.snapshot();
                match part {
                    Part::Headers => {
                        attrs.title = self.hash("Column headers");
                        attrs.value = self.hash(snapshot.column_count());
                    }
                    Part::Row(key, _) => {
                        let index = t.row_index(key).unwrap();
                        let row = t.row(index).unwrap();
                        attrs.title = self.hash(&row.cells);
                        attrs.value = attrs.title;
                        attrs.meta = self.hash((index, n.enabled && row.enabled));
                    }
                    Part::Column(key, _) | Part::Header(key, _) => {
                        let c = snapshot.column(t.column_index(key).unwrap()).unwrap();
                        attrs.title = self.hash(&c.title);
                        attrs.value =
                            self.hash((&c.title, format!("{:?}", c.sort), c.sort_priority));
                        attrs.meta = self.hash((
                            c.index,
                            c.width.to_bits(),
                            c.editable,
                            c.sortable,
                            n.enabled,
                        ));
                    }
                    Part::Cell(cell, ..) => {
                        let c = t.cell_info(cell).unwrap();
                        attrs.title = self.hash(snapshot.column(c.column_index).map(|c| &c.title));
                        attrs.value = self.hash((&c.text, c.editing, &c.error));
                        attrs.meta = self.hash((
                            c.row_index,
                            c.column_index,
                            n.enabled && c.enabled,
                            c.editable,
                        ));
                        attrs.bounds = nodes
                            .iter()
                            .find(|p| {
                                p.table_cell.as_ref() == Some(cell)
                                    && table_accessibility::within(nodes, p.id, n.id)
                            })
                            .and_then(|p| self.bounds(p.clip));
                    }
                }
            }
        }
        Item {
            target,
            attrs: active.then_some(attrs),
            retired,
        }
    }
    pub(super) fn next(
        &self,
        m: &Mounted,
        s: &Surface,
        watched: &[(Target, Retained<AnyObject>)],
        focus: Option<Key>,
    ) -> std::result::Result<Snapshot, ui::Error> {
        let nodes = m.host.snapshot()?;
        let mut items = HashMap::new();
        for n in &nodes {
            if exposed(m, n, &nodes) {
                let item = self.capture(Target::Node(n.id), m, &nodes);
                items.insert(item.target.key(), item);
            }
        }
        for (target, _) in watched {
            let item = self.capture(target.clone(), m, &nodes);
            items.insert(target.key(), item);
        }
        let text = m.host.text_input()?.map(|t| Text {
            id: t.id,
            selection: t.selection,
            marked: t.marked,
        });
        let frame = s.native_window().frame();
        let context = Some([
            frame.origin.x.to_bits(),
            frame.origin.y.to_bits(),
            frame.size.width.to_bits(),
            frame.size.height.to_bits(),
            s.native_window().backingScaleFactor().to_bits(),
        ]);
        Ok(Snapshot {
            items,
            focus,
            text,
            context,
            initialized: true,
        })
    }
}
/// Native row wrappers are represented by logical proxies, while actual controls
/// nested inside list/table content keep their own text-input/action identities.
fn exposed(m: &Mounted, n: &Node, nodes: &[Node]) -> bool {
    if !accessibility::semantic(n) || !control_accessibility::readable(m, n, nodes) {
        return false;
    }
    if n.table_root.is_some() {
        return true;
    }
    if n.kind == Kind::CollectionRow
        && n.parent
            .and_then(|p| nodes.iter().find(|n| n.id == p))
            .is_some_and(|p| p.collection.is_some())
    {
        return false;
    }
    let mut p = Some(n.id);
    while let Some(id) = p {
        let Some(a) = nodes.iter().find(|n| n.id == id) else {
            break;
        };
        if a.tree_item.is_some() {
            return n.key.as_deref() == Some("$retry");
        }
        if a.table_root.is_some() {
            return n.table_editor.is_some()
                || n.table_resize
                || nodes.iter().any(|c| {
                    c.id != n.id
                        && c.table_cell.is_some()
                        && table_accessibility::within(nodes, n.id, c.id)
                });
        }
        p = a.parent;
    }
    true
}
