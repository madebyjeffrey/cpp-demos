//! Native table semantics use logical lifetimes, never a fully materialized table.
use super::{
    accessibility::{Element, element},
    table_array::{Source, array},
    *,
};
use objc2::Message;
use ui::{Node, NodeId, TableCell, TableModel, TableSelectionMode};
pub(super) type Cache = std::collections::HashMap<(NodeId, u8, u64, u64), objc2::rc::Weak<Entry>>;
#[derive(Clone)]
pub(super) enum Part {
    Headers,
    Row(String, u64),
    Column(String, u64),
    Header(String, u64),
    Cell(TableCell, u64, u64),
}
impl Part {
    fn identity(&self, root: NodeId) -> (NodeId, u8, u64, u64) {
        match self {
            Self::Headers => (root, 4, 0, 0),
            Self::Row(_, r) => (root, 0, *r, 0),
            Self::Column(_, c) => (root, 1, 0, *c),
            Self::Header(_, c) => (root, 2, 0, *c),
            Self::Cell(_, r, c) => (root, 3, *r, *c),
        }
    }
    fn valid(&self, t: &TableModel) -> bool {
        match self {
            Self::Headers => true,
            Self::Row(k, r) => t.row_token(k) == Some(*r),
            Self::Column(k, c) | Self::Header(k, c) => t.column_token(k) == Some(*c),
            Self::Cell(k, r, c) => {
                t.row_token(&k.row) == Some(*r) && t.column_token(&k.column) == Some(*c)
            }
        }
    }
}
fn any<T: Message>(v: Retained<T>) -> Retained<AnyObject> {
    unsafe { Retained::cast_unchecked(v) }
}
pub(super) fn entry(owner: Weak<Mounted>, root: NodeId, part: Part) -> Retained<AnyObject> {
    let identity = part.identity(root);
    if let Some(m) = owner.upgrade()
        && let Some(v) = m
            .table_cache
            .borrow()
            .get(&identity)
            .and_then(objc2::rc::Weak::load)
    {
        return any(v);
    }
    let a = Entry::alloc(MainThreadMarker::new().unwrap()).set_ivars(EntryIvars {
        owner: owner.clone(),
        root,
        part,
    });
    let value: Retained<Entry> = unsafe { msg_send![super(a), init] };
    if let Some(m) = owner.upgrade() {
        m.table_cache
            .borrow_mut()
            .insert(identity, objc2::rc::Weak::from_retained(&value));
    }
    any(value)
}
pub(super) fn within(nodes: &[Node], mut id: NodeId, root: NodeId) -> bool {
    loop {
        if id == root {
            return true;
        }
        let Some(p) = nodes.iter().find(|n| n.id == id).and_then(|n| n.parent) else {
            return false;
        };
        id = p;
    }
}
fn body<'a>(nodes: &'a [Node], root: &Node, t: &TableModel) -> Option<&'a Node> {
    nodes
        .iter()
        .find(|n| n.table_model.as_ref() == Some(t) && within(nodes, n.id, root.id))
}
pub(super) fn logical(m: &Mounted, id: NodeId, focus: bool) -> Option<Retained<AnyObject>> {
    let nodes = m.host.snapshot().ok()?;
    let original = nodes.iter().find(|n| n.id == id)?;
    if original.table_editor.is_some() || original.table_resize {
        return None;
    }
    let root = nodes
        .iter()
        .find(|n| n.table_root.is_some() && within(&nodes, id, n.id))?;
    let t = root.table_root.as_ref()?;
    if original.table_model.as_ref() == Some(t) && focus {
        let part = if t.options().selection_mode == TableSelectionMode::Rows {
            t.row_cursor()
                .and_then(|k| Some(Part::Row(k.clone(), t.row_token(&k)?)))
        } else {
            t.cursor().and_then(|c| {
                let r = t.row_token(&c.row)?;
                let k = t.column_token(&c.column)?;
                Some(Part::Cell(c, r, k))
            })
        };
        let owner = m
            .surface
            .upgrade()?
            .state
            .components
            .borrow()
            .as_ref()
            .map(Rc::downgrade)?;
        return Some(part.map_or_else(|| any(element(m, root.id)), |p| entry(owner, root.id, p)));
    }
    let mut current = Some(id);
    let mut part = None;
    while let Some(id) = current {
        let n = nodes.iter().find(|n| n.id == id)?;
        if let Some(cell) = &n.table_cell {
            if n.id != original.id {
                return None;
            }
            part = Some(Part::Cell(
                cell.clone(),
                t.row_token(&cell.row)?,
                t.column_token(&cell.column)?,
            ));
            break;
        }
        if let Some(column) = &n.table_column {
            part = Some(Part::Header(column.clone(), t.column_token(column)?));
            break;
        }
        if n.kind == ui::Kind::CollectionRow
            && n.parent
                .and_then(|p| nodes.iter().find(|n| n.id == p))
                .is_some_and(|n| n.table_model.as_ref() == Some(t))
        {
            let key = n.key.clone()?;
            part = Some(Part::Row(key.clone(), t.row_token(&key)?));
            break;
        }
        if n.id == root.id {
            break;
        }
        current = n.parent;
    }
    if focus
        && t.options().selection_mode == TableSelectionMode::Rows
        && let Some(Part::Cell(c, r, _)) = part
    {
        part = Some(Part::Row(c.row, r));
    }
    let owner = m
        .surface
        .upgrade()?
        .state
        .components
        .borrow()
        .as_ref()
        .map(Rc::downgrade)?;
    Some(part.map_or_else(|| any(element(m, root.id)), |p| entry(owner, root.id, p)))
}
// Scaffolding remains available through logical cell children, but never duplicates
// a table in its parent's accessibility child list.
pub(super) fn scaffold(nodes: &[Node], n: &Node, parent: Option<NodeId>) -> bool {
    let root = nodes
        .iter()
        .find(|root| root.table_root.is_some() && n.id != root.id && within(nodes, n.id, root.id));
    root.is_some_and(|root| parent.is_none_or(|p| !within(nodes, p, root.id)))
}
pub(super) struct EntryIvars {
    owner: Weak<Mounted>,
    root: NodeId,
    part: Part,
}
impl Drop for EntryIvars {
    fn drop(&mut self) {
        if let Some(m) = self.owner.upgrade()
            && let Ok(mut c) = m.table_cache.try_borrow_mut()
        {
            c.remove(&self.part.identity(self.root));
        }
    }
}
define_class!(
    // SAFETY: Main-thread NSAccessibilityElement subclass with AppKit protocol signatures.
    #[unsafe(super=NSAccessibilityElement)] #[thread_kind=MainThreadOnly] #[ivars=EntryIvars]
    pub(super) struct Entry;
    unsafe impl NSObjectProtocol for Entry {}
    unsafe impl NSAccessibility for Entry {
        #[unsafe(method(isAccessibilityElement))] fn live(&self)->bool{self.with(|_,_,_,_,_,_|true).unwrap_or(false)}
        #[unsafe(method_id(accessibilityRole))] fn role(&self)->Option<Retained<NSString>>{self.with(|_,_,_,_,_,_|unsafe{match self.ivars().part{Part::Headers=>NSAccessibilityGroupRole,Part::Row(..)=>NSAccessibilityRowRole,Part::Column(..)=>NSAccessibilityColumnRole,Part::Header(..)=>NSAccessibilityButtonRole,Part::Cell(..)=>NSAccessibilityCellRole}.retain()})}
        #[unsafe(method_id(accessibilityIdentifier))] fn identifier(&self)->Option<Retained<NSString>>{self.with(|_,_,_,_,_,_|{let (root,kind,r,c)=self.ivars().part.identity(self.ivars().root);ns(&format!("{root}/table/{kind}/{r}/{c}"))})}
        #[unsafe(method_id(accessibilityLabel))] fn label(&self)->Option<Retained<NSString>>{self.with(|_,_,_,_,t,_|match &self.ivars().part{Part::Headers=>ns("Column headers"),Part::Row(k,_)=>ns(&{let row=t.row(t.row_index(k).unwrap()).unwrap();t.columns().iter().map(|c|row.cells.get(&c.id).map_or("",String::as_str)).collect::<Vec<_>>().join(" · ")}),Part::Column(k,_)|Part::Header(k,_)=>ns(&t.columns()[t.column_index(k).unwrap()].title),Part::Cell(c,..)=>ns(&t.columns()[t.column_index(&c.column).unwrap()].title)})}
        #[unsafe(method_id(accessibilityValue))] fn value(&self)->Option<Retained<AnyObject>>{self.cell_info().map(|c|any(ns(&c.text)))}
        #[unsafe(method_id(accessibilityHelp))] fn help(&self)->Option<Retained<NSString>>{self.entry_help()}
        #[unsafe(method_id(accessibilityValueDescription))] fn description(&self)->Option<Retained<NSString>>{self.cell_info().and_then(|c|if c.editing{Some(ns(if c.error.is_some(){"Editing, invalid value"}else{"Editing"}))}else{None})}
        #[unsafe(method(accessibilityIndex))] fn index(&self)->isize{self.with(|_,_,_,_,t,_|match &self.ivars().part{Part::Row(k,_)=>t.row_index(k),Part::Column(k,_)|Part::Header(k,_)=>t.column_index(k),Part::Cell(..)|Part::Headers=>None}.map_or(-1,|i|i as isize)).unwrap_or(-1)}
        #[unsafe(method(accessibilityRowIndexRange))] fn row_index(&self)->NSRange{self.cell_info().map_or(NSRange::new(NSNotFound as usize,0),|c|NSRange::new(c.row_index,1))}
        #[unsafe(method(accessibilityColumnIndexRange))] fn column_index(&self)->NSRange{self.cell_info().map_or(NSRange::new(NSNotFound as usize,0),|c|NSRange::new(c.column_index,1))}
        #[unsafe(method(accessibilitySortDirection))] fn sort(&self)->NSAccessibilitySortDirection{self.with(|_,_,_,_,t,_|{let key=match &self.ivars().part{Part::Column(k,_)|Part::Header(k,_)=>Some(k),_=>None};match key.and_then(|k|t.sort().into_iter().find(|s|s.column==*k)).map(|s|s.order){Some(ui::SortOrder::Ascending)=>NSAccessibilitySortDirection::Ascending,Some(ui::SortOrder::Descending)=>NSAccessibilitySortDirection::Descending,None=>NSAccessibilitySortDirection::Unknown}}).unwrap_or(NSAccessibilitySortDirection::Unknown)}
        #[unsafe(method(isAccessibilityEnabled))] fn enabled(&self)->bool{self.with(|_,_,root,_,t,_|root.enabled&&match &self.ivars().part{Part::Row(k,_)=>t.row(t.row_index(k).unwrap()).unwrap().enabled,Part::Cell(c,..)=>t.cell_info(c).unwrap().enabled,_=>true}).unwrap_or(false)}
        #[unsafe(method(isAccessibilitySelected))] fn selected(&self)->bool{self.with(|_,_,_,_,t,_|match &self.ivars().part{Part::Row(k,_)=>t.is_row_selected(k),Part::Cell(c,..)=>t.cell_info(c).unwrap().selected,_=>false}).unwrap_or(false)}
        #[unsafe(method(isAccessibilityFocused))] fn focused(&self)->bool{self.with(|_,_,root,_,t,nodes|{let active=nodes.iter().any(|n|n.focused&&n.table_editor.is_none()&&n.table_column.is_none()&&(n.table_cell.is_some()||n.collection.is_some()||n.kind==ui::Kind::CollectionRow)&&within(nodes,n.id,root.id));match &self.ivars().part{Part::Row(k,_)=>active&&t.options().selection_mode==TableSelectionMode::Rows&&t.row_cursor().as_ref()==Some(k),Part::Cell(c,..)=>active&&t.options().selection_mode==TableSelectionMode::Cells&&t.cursor().as_ref()==Some(c),Part::Header(k,_)=>nodes.iter().any(|n|n.focused&&n.table_column.as_ref()==Some(k)&&within(nodes,n.id,root.id)),_=>false}}).unwrap_or(false)}
        #[unsafe(method(setAccessibilityFocused:))] fn focus(&self,v:bool){if v&&matches!(self.ivars().part,Part::Row(..)|Part::Cell(..)){self.select(ui::ListSelection::Focus);}}
        #[unsafe(method(setAccessibilitySelected:))] fn selected_set(&self,v:bool){self.change_selected(v);}
        #[unsafe(method(accessibilityPerformPress))] fn press(&self)->bool{self.select(ui::ListSelection::Replace)}
        #[unsafe(method(accessibilityPerformConfirm))] fn confirm(&self)->bool{self.with(|m,_,_,body,_,_|match &self.ivars().part{Part::Cell(c,r,k)=>m.host.edit_table_cell(body.id,c.clone(),*r,*k).unwrap_or(false),_=>false}).unwrap_or(false)}
        #[unsafe(method(setAccessibilityValue:))] unsafe fn set_value(&self,value:Option<&AnyObject>){if let Some(s)=value.and_then(|v|v.downcast_ref::<NSString>()){self.with(|m,_,_,body,_,_|{if let Part::Cell(c,r,k)=&self.ivars().part{let _=m.host.set_table_cell_text(body.id,c.clone(),*r,*k,&s.to_string());}});}}
        #[unsafe(method_id(accessibilityParent))] fn parent(&self)->Option<Retained<AnyObject>>{self.with(|m,_,root,_,_,_|match &self.ivars().part{Part::Cell(c,r,_)=>entry(self.ivars().owner.clone(),root.id,Part::Row(c.row.clone(),*r)),Part::Header(..)=>entry(self.ivars().owner.clone(),root.id,Part::Headers),_=>any(element(m,root.id))})}
        #[unsafe(method_id(accessibilityWindow))] fn window(&self)->Option<Retained<AnyObject>>{self.with(|_,s,_,_,_,_|any(s.native_window().retain()))}
        #[unsafe(method_id(accessibilityHeader))] fn header(&self)->Option<Retained<AnyObject>>{self.with(|_,_,root,_,_,_|if let Part::Column(k,c)=&self.ivars().part{Some(entry(self.ivars().owner.clone(),root.id,Part::Header(k.clone(),*c)))}else{None}).flatten()}
        #[unsafe(method_id(accessibilityChildren))] fn children(&self)->Option<Retained<NSArray<AnyObject>>>{self.children_range(0,usize::MAX)}
        #[unsafe(method(accessibilityFrame))] fn frame(&self)->NSRect{self.with(|_,s,root,body,t,nodes|{
            let rect=match &self.ivars().part{
                Part::Headers=>ui::Rect::new(body.bounds.x,root.bounds.y,body.bounds.width,t.options().header_height).intersection(root.clip),
                Part::Cell(c,..)=>nodes.iter().find(|n|n.table_cell.as_ref()==Some(c)&&within(nodes,n.id,root.id)).map(|n|n.clip).unwrap_or_default(),
                Part::Header(k,_)=>nodes.iter().find(|n|n.table_column.as_ref()==Some(k)&&!n.table_resize&&within(nodes,n.id,root.id)).map(|n|n.clip).unwrap_or_default(),
                Part::Row(k,_)=>{let list=body.collection.as_ref().unwrap();ui::Rect::new(body.bounds.x,body.bounds.y+t.row_index(k).unwrap() as f64*list.row_height()-list.offset(),body.bounds.width,list.row_height()).intersection(body.clip)},
                Part::Column(k,_)=>{let columns=t.columns();let i=t.column_index(k).unwrap();let x=columns[..i].iter().map(|c|c.width).sum::<f64>();ui::Rect::new(body.bounds.x+x,root.bounds.y,columns[i].width,root.bounds.height).intersection(root.clip)}
            };s.native_window().convertRectToScreen(s.native_view().convertRect_toView(NSRect::new(NSPoint::new(rect.x,rect.y),NSSize::new(rect.width,rect.height)),None))}).unwrap_or(NSRect::ZERO)}
        #[unsafe(method(isAccessibilitySelectorAllowed:))] unsafe fn allowed(&self,s:objc2::runtime::Sel)->bool{self.with(|_,_,root,_,t,_|{let enabled=root.enabled&&self.part_enabled(t);match s.name().to_bytes(){b"setAccessibilityValue:"|b"accessibilityPerformConfirm"=>enabled&&self.cell_target().is_some_and(|c|t.is_editable(c)),b"setAccessibilitySelected:"=>enabled&&match self.ivars().part{Part::Row(..)=>t.options().selection_mode==TableSelectionMode::Rows,Part::Cell(..)=>t.options().selection_mode==TableSelectionMode::Cells,_=>false},b"accessibilityPerformPress"=>enabled&&match &self.ivars().part{Part::Header(k,_)=>t.columns()[t.column_index(k).unwrap()].sortable,Part::Row(..)|Part::Cell(..)=>true,_=>false},b"setAccessibilityFocused:"=>enabled&&matches!(self.ivars().part,Part::Row(..)|Part::Cell(..)),_=>true}}).unwrap_or(false)}
    }
    impl Entry {
        #[unsafe(method(accessibilityArrayAttributeCount:))] fn count(&self,a:&NSString)->usize{if a.to_string()=="AXChildren"{self.children_count()}else{0}}
        #[unsafe(method_id(accessibilityArrayAttributeValues:index:maxCount:))] fn range(&self,a:&NSString,start:usize,count:usize)->Option<Retained<NSArray<AnyObject>>>{if a.to_string()=="AXChildren"{self.children_range(start,count)}else{None}}
    }
);
impl Entry {
    fn with<T>(
        &self,
        f: impl FnOnce(&Mounted, &Surface, &Node, &Node, &TableModel, &[Node]) -> T,
    ) -> Option<T> {
        let m = self.ivars().owner.upgrade()?;
        m.run(|m, s| {
            let nodes = m.host.snapshot()?;
            Ok(nodes
                .iter()
                .find(|n| n.id == self.ivars().root && !n.hidden && m.host.in_scope(n.id))
                .and_then(|root| {
                    let t = root.table_root.as_ref()?;
                    if !self.ivars().part.valid(t) {
                        return None;
                    }
                    Some(f(m, s, root, body(&nodes, root, t)?, t, &nodes))
                }))
        })
        .flatten()
    }
    fn entry_help(&self) -> Option<Retained<NSString>> {
        self.with(|_, _, _, _, t, _| match &self.ivars().part {
            Part::Cell(c, ..) => t.cell_info(c).and_then(|c| c.error).map(|e| ns(&e)),
            Part::Column(k, _) | Part::Header(k, _) => {
                let sort = t.sort();
                sort.iter()
                    .position(|s| s.column == *k)
                    .map(|i| ns(&format!("Sort priority {} of {}", i + 1, sort.len())))
            }
            _ => None,
        })
        .flatten()
    }
    fn cell_target(&self) -> Option<&TableCell> {
        if let Part::Cell(c, ..) = &self.ivars().part {
            Some(c)
        } else {
            None
        }
    }
    fn cell_info(&self) -> Option<ui::TableCellInfo> {
        self.with(|_, _, _, _, t, _| self.cell_target().and_then(|c| t.cell_info(c)))
            .flatten()
    }
    fn part_enabled(&self, t: &TableModel) -> bool {
        match &self.ivars().part {
            Part::Cell(c, ..) => t.cell_info(c).is_some_and(|c| c.enabled),
            Part::Row(k, _) => t
                .row_index(k)
                .and_then(|i| t.row(i))
                .is_some_and(|r| r.enabled),
            _ => true,
        }
    }
    fn select(&self, operation: ui::ListSelection) -> bool {
        self.with(|m, _, root, body, t, _| {
            if !root.enabled || !self.part_enabled(t) {
                return false;
            }
            match &self.ivars().part {
                Part::Row(k, _) => m
                    .host
                    .select_collection_item_with(body.id, t.row_index(k).unwrap(), operation)
                    .unwrap_or(false),
                Part::Cell(c, r, k) => m
                    .host
                    .select_table_cell(body.id, c.clone(), *r, *k, operation)
                    .unwrap_or(false),
                Part::Header(k, c) => m
                    .host
                    .sort_table_column(body.id, k, *c, false)
                    .unwrap_or(false),
                _ => false,
            }
        })
        .unwrap_or(false)
    }
    fn change_selected(&self, value: bool) {
        self.with(|m, _, root, body, t, _| {
            if !root.enabled || !self.part_enabled(t) {
                return;
            }
            let mut selected = t.selection();
            let multiple = t.options().multiple_selection;
            match &self.ivars().part {
                Part::Row(k, _) if t.options().selection_mode == TableSelectionMode::Rows => {
                    if value {
                        if !multiple {
                            selected.rows.clear();
                        }
                        selected.rows.insert(k.clone());
                    } else {
                        selected.rows.remove(k);
                    }
                }
                Part::Cell(c, ..) if t.options().selection_mode == TableSelectionMode::Cells => {
                    if value {
                        if !multiple {
                            selected.cells.clear();
                        }
                        selected.cells.insert(c.clone());
                    } else {
                        selected.cells.remove(c);
                    }
                }
                _ => return,
            }
            let _ = m.host.set_table_selection(body.id, selected);
        });
    }
    fn children_count(&self) -> usize {
        self.with(|m, _, root, _, t, nodes| match &self.ivars().part {
            Part::Headers => t.column_count(),
            Part::Row(..) => t.column_count(),
            Part::Column(..) => t.row_count(),
            Part::Header(k, _) => usize::from(nodes.iter().any(|n| {
                n.table_resize && n.table_column.as_ref() == Some(k) && within(nodes, n.id, root.id)
            })),
            Part::Cell(c, ..) => {
                let actual = nodes
                    .iter()
                    .find(|n| n.table_cell.as_ref() == Some(c) && within(nodes, n.id, root.id));
                actual.map_or(0, |n| {
                    if n.table_editor.is_some() {
                        1
                    } else {
                        super::accessibility::list(m, Some(n.id)).map_or(0, |a| a.len())
                    }
                })
            }
        })
        .unwrap_or(0)
    }
    fn children_range(&self, start: usize, count: usize) -> Option<Retained<NSArray<AnyObject>>> {
        self.with(|m, _, root, _, t, nodes| {
            let snapshot = t.snapshot();
            let slice = |len: usize| {
                let end = start.saturating_add(count).min(len);
                start.min(end)..end
            };
            let source = match &self.ivars().part {
                Part::Headers => Source::Headers(slice(t.column_count())),
                Part::Row(k, r) => Source::Explicit(
                    slice(snapshot.column_count())
                        .filter_map(|i| {
                            snapshot
                                .column(i)
                                .map(|c| Part::Cell(TableCell::new(k, &c.id), *r, c.token))
                        })
                        .collect(),
                ),
                Part::Column(k, _) => {
                    Source::Cells(slice(t.row_count()), vec![t.column_index(k).unwrap()])
                }
                Part::Header(k, _) => {
                    let values: Vec<_> = nodes
                        .iter()
                        .filter(|n| {
                            n.table_resize
                                && n.table_column.as_ref() == Some(k)
                                && within(nodes, n.id, root.id)
                        })
                        .skip(start)
                        .take(count)
                        .map(|n| any(element(m, n.id)))
                        .collect();
                    return NSArray::from_retained_slice(&values);
                }
                Part::Cell(c, ..) => {
                    let actual = nodes
                        .iter()
                        .find(|n| n.table_cell.as_ref() == Some(c) && within(nodes, n.id, root.id));
                    let values = actual
                        .map(|n| {
                            if n.table_editor.is_some() {
                                NSArray::from_retained_slice(&[any(element(m, n.id))])
                            } else {
                                super::accessibility::list(m, Some(n.id))
                                    .unwrap_or_else(|_| NSArray::new())
                            }
                        })
                        .unwrap_or_else(NSArray::new);
                    return NSArray::from_retained_slice(
                        &values.iter().skip(start).take(count).collect::<Vec<_>>(),
                    );
                }
            };
            array(self.ivars().owner.clone(), root.id, snapshot, source)
        })
    }
}
fn clip_range(start: usize, count: usize, len: usize) -> std::ops::Range<usize> {
    let end = start.saturating_add(count).min(len);
    start.min(end)..end
}
impl Element {
    pub(super) fn table_count(&self, attribute: &str) -> Option<usize> {
        self.with(|m, _, root| {
            let Some(t) = &root.table_root else {
                return Ok(None);
            };
            let count = match attribute {
                "AXRows" => t.row_count(),
                "AXChildren" => t
                    .row_count()
                    .saturating_add(t.column_count())
                    .saturating_add(1),
                "AXColumns" | "AXColumnHeaderUIElements" => t.column_count(),
                "AXSelectedRows" => {
                    if t.options().selection_mode == TableSelectionMode::Rows {
                        t.selection_count()
                    } else {
                        0
                    }
                }
                "AXSelectedCells" => {
                    if t.options().selection_mode == TableSelectionMode::Cells {
                        t.selection_count()
                    } else {
                        0
                    }
                }
                "AXSelectedChildren" => t.selection_count(),
                "AXVisibleRows" | "AXVisibleCells" | "AXVisibleColumns" => {
                    let nodes = m.host.snapshot()?;
                    let Some(body) = body(&nodes, root, t) else {
                        return Ok(None);
                    };
                    let rows = super::collection_accessibility::viewport(
                        body.collection.as_ref().unwrap(),
                    )
                    .len();
                    if attribute == "AXVisibleRows" {
                        rows
                    } else {
                        let cols = snapshot_columns(&t.snapshot(), root, body).len();
                        if attribute == "AXVisibleColumns" {
                            cols
                        } else {
                            rows.saturating_mul(cols)
                        }
                    }
                }
                _ => return Ok(None),
            };
            Ok(Some(count))
        })
        .flatten()
    }
    pub(super) fn table_array(
        &self,
        attribute: &str,
        start: usize,
        count: usize,
    ) -> Option<Retained<NSArray<AnyObject>>> {
        if attribute == "AXSelectedChildren" {
            let mode = self
                .with(|_, _, n| Ok(n.table_root.as_ref().map(|t| t.options().selection_mode)))
                .flatten()?;
            return self.table_array(
                if mode == TableSelectionMode::Rows {
                    "AXSelectedRows"
                } else {
                    "AXSelectedCells"
                },
                start,
                count,
            );
        }
        self.with(|m, _, root| {
            let Some(t) = &root.table_root else {
                return Ok(None);
            };
            let nodes = m.host.snapshot()?;
            let Some(body) = body(&nodes, root, t) else {
                return Ok(None);
            };
            let snapshot = t.snapshot();
            let visible_columns = || snapshot_columns(&snapshot, root, body);
            let source = match attribute {
                "AXRows" => Source::Rows(clip_range(start, count, t.row_count())),
                "AXChildren" => Source::Children(clip_range(
                    start,
                    count,
                    t.row_count()
                        .saturating_add(t.column_count())
                        .saturating_add(1),
                )),
                "AXColumns" => Source::Columns(clip_range(start, count, snapshot.column_count())),
                "AXColumnHeaderUIElements" => {
                    Source::Headers(clip_range(start, count, snapshot.column_count()))
                }
                "AXVisibleColumns" => Source::Explicit(
                    visible_columns()
                        .into_iter()
                        .skip(start)
                        .take(count)
                        .map(|i| {
                            let c = snapshot.column(i).unwrap();
                            Part::Column(c.id.clone(), c.token)
                        })
                        .collect(),
                ),
                "AXVisibleRows" => {
                    let r = super::collection_accessibility::viewport(
                        body.collection.as_ref().unwrap(),
                    );
                    let window = clip_range(start, count, r.len());
                    Source::Rows(r.start + window.start..r.start + window.end)
                }
                "AXVisibleCells" => {
                    let r = super::collection_accessibility::viewport(
                        body.collection.as_ref().unwrap(),
                    );
                    let cols = visible_columns();
                    let range = clip_range(start, count, r.len().saturating_mul(cols.len()));
                    Source::Explicit(
                        range
                            .map(|i| {
                                let row = r.start + i / cols.len();
                                let col = cols[i % cols.len()];
                                let c = snapshot.cell(row, col).unwrap();
                                let token = snapshot.rows().item_token(&c.row).unwrap();
                                Part::Cell(c, token, snapshot.column(col).unwrap().token)
                            })
                            .collect(),
                    )
                }
                "AXSelectedRows" => Source::Explicit(
                    t.selected_rows(start, count)
                        .into_iter()
                        .filter_map(|k| Some(Part::Row(k.clone(), t.row_token(&k)?)))
                        .collect(),
                ),
                "AXSelectedCells" => Source::Explicit(
                    t.selected_cells(start, count)
                        .into_iter()
                        .filter_map(|c| {
                            let r = t.row_token(&c.row)?;
                            let col = t.column_token(&c.column)?;
                            Some(Part::Cell(c, r, col))
                        })
                        .collect(),
                ),
                _ => return Ok(None),
            };
            Ok(Some(array(
                self.ivars().owner.clone(),
                root.id,
                snapshot,
                source,
            )))
        })
        .flatten()
    }
    pub(super) fn table_cell(&self, column: isize, row: isize) -> Option<Retained<AnyObject>> {
        if column < 0 || row < 0 {
            return None;
        }
        self.with(|_, _, root| {
            Ok(root.table_root.as_ref().and_then(|t| {
                let s = t.snapshot();
                let c = s.cell(row as usize, column as usize)?;
                let r = s.rows().item_token(&c.row)?;
                Some(entry(
                    self.ivars().owner.clone(),
                    root.id,
                    Part::Cell(c, r, s.column(column as usize)?.token),
                ))
            }))
        })
        .flatten()
    }
    pub(super) fn table_selection(&self, values: Option<&NSArray<AnyObject>>, cells: bool) {
        self.with(|m, _, root| {
            let Some(t) = &root.table_root else {
                return Ok(false);
            };
            let nodes = m.host.snapshot()?;
            let Some(body) = body(&nodes, root, t) else {
                return Ok(false);
            };
            let mut selection = ui::TableSelection::default();
            for v in values.into_iter().flatten() {
                let Some(e) = v.downcast_ref::<Entry>() else {
                    return Ok(false);
                };
                let Some(owner) = e.ivars().owner.upgrade() else {
                    return Ok(false);
                };
                if !std::ptr::eq(&*owner, m)
                    || e.ivars().root != root.id
                    || !e.ivars().part.valid(t)
                {
                    return Ok(false);
                }
                match &e.ivars().part {
                    Part::Cell(c, ..) if cells => {
                        selection.cells.insert(c.clone());
                    }
                    Part::Row(k, _) if !cells => {
                        selection.rows.insert(k.clone());
                    }
                    _ => return Ok(false),
                }
            }
            Ok(m.host
                .set_table_selection(body.id, selection)
                .unwrap_or(false))
        });
    }
}
fn snapshot_columns(snapshot: &ui::TableSnapshot, root: &Node, body: &Node) -> Vec<usize> {
    let mut x = body.bounds.x;
    let mut columns = vec![];
    for i in 0..snapshot.column_count() {
        let c = snapshot.column(i).unwrap();
        if x + c.width > root.clip.x && x < root.clip.x + root.clip.width {
            columns.push(i)
        }
        x += c.width;
    }
    columns
}
pub(super) fn editor_parent(m: &Mounted, n: &Node) -> Option<Retained<AnyObject>> {
    if n.table_editor.is_none() && !n.table_resize {
        return None;
    }
    let nodes = m.host.snapshot().ok()?;
    let root = nodes
        .iter()
        .find(|r| r.table_root.is_some() && within(&nodes, n.id, r.id))?;
    let t = root.table_root.as_ref()?;
    let owner = m
        .surface
        .upgrade()?
        .state
        .components
        .borrow()
        .as_ref()
        .map(Rc::downgrade)?;
    if let Some(c) = &n.table_cell {
        Some(entry(
            owner,
            root.id,
            Part::Cell(c.clone(), t.row_token(&c.row)?, t.column_token(&c.column)?),
        ))
    } else {
        let k = n.table_column.as_ref()?;
        Some(entry(
            owner,
            root.id,
            Part::Header(k.clone(), t.column_token(k)?),
        ))
    }
}
