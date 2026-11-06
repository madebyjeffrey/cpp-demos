//! Logical row proxies are created only for requested ranges, including offscreen rows.
use super::{
    accessibility::{Element, element},
    *,
};
use objc2::Message;
use ui::{ListItem, ListModel, Node, NodeId};
fn any<T: Message>(value: Retained<T>) -> Retained<AnyObject> {
    unsafe { Retained::cast_unchecked(value) }
}
pub(super) fn rows(
    owner: Weak<Mounted>,
    node: &Node,
    start: usize,
    count: usize,
) -> Retained<NSArray<AnyObject>> {
    let Some(model) = &node.collection else {
        return NSArray::new();
    };
    let end = start.saturating_add(count).min(model.len());
    super::row_array::array(
        owner,
        node.id,
        super::row_array::Source::Range(model.snapshot(), start.min(end)..end),
    )
}
pub(super) fn row(owner: Weak<Mounted>, parent: NodeId, key: String, token: u64) -> Retained<Row> {
    let identity = (parent, token);
    if let Some(m) = owner.upgrade()
        && let Some(value) = m
            .row_cache
            .borrow()
            .get(&identity)
            .and_then(objc2::rc::Weak::load)
    {
        return value;
    }
    let spec = super::notification_state::Target::Row(parent, key.clone(), token);
    let allocated = Row::alloc(MainThreadMarker::new().unwrap()).set_ivars(RowIvars {
        owner: owner.clone(),
        parent,
        key,
        token,
    });
    let value: Retained<Row> = unsafe { msg_send![super(allocated), init] };
    if let Some(m) = owner.upgrade() {
        m.row_cache
            .borrow_mut()
            .insert(identity, objc2::rc::Weak::from_retained(&value));
        m.prime_notification(spec);
    }
    value
}

pub(super) fn logical(m: &Mounted, id: NodeId) -> Option<Retained<AnyObject>> {
    let nodes = m.host.snapshot().ok()?;
    let original = nodes.iter().find(|n| n.id == id)?;
    if original.key.as_deref() == Some("$retry") {
        return None;
    }
    let mut current = Some(id);
    while let Some(id) = current {
        let n = nodes.iter().find(|n| n.id == id)?;
        if n.collection.is_some() {
            return None;
        }
        if n.kind == ui::Kind::CollectionRow
            && let Some(root) = n
                .parent
                .and_then(|p| nodes.iter().find(|n| n.id == p && n.collection.is_some()))
        {
            if root.tree_model.is_none() && n.id != original.id {
                return None;
            }
            let key = n.key.as_ref()?;
            let token = root.collection.as_ref()?.item_token(key)?;
            let owner = m
                .surface
                .upgrade()?
                .state
                .components
                .borrow()
                .as_ref()
                .map(Rc::downgrade)?;
            return Some(any(row(owner, root.id, key.clone(), token)));
        }
        current = n.parent;
    }
    None
}
pub(super) fn viewport(model: &ListModel) -> std::ops::Range<usize> {
    if model.viewport_height() <= 0. {
        return 0..0;
    }
    let start = (model.offset() / model.row_height()).floor() as usize;
    let end = ((model.offset() + model.viewport_height()) / model.row_height()).ceil() as usize;
    start.min(model.len())..end.min(model.len())
}
pub(super) struct RowIvars {
    owner: Weak<Mounted>,
    parent: NodeId,
    key: String,
    token: u64,
}
impl Drop for RowIvars {
    fn drop(&mut self) {
        // The registry is weak: keep identity only while a caller/array holds the proxy.
        if let Some(owner) = self.owner.upgrade()
            && let Ok(mut cache) = owner.row_cache.try_borrow_mut()
        {
            cache.remove(&(self.parent, self.token));
            owner.forget_notification(super::notification_state::Key::Row(self.parent, self.token));
        }
    }
}
define_class!(
    // SAFETY: NSAccessibilityElement is subclassable; all callbacks run on the main thread.
    #[unsafe(super=NSAccessibilityElement)]
    #[thread_kind=MainThreadOnly]
    #[ivars=RowIvars]
    pub(super) struct Row;
    unsafe impl NSObjectProtocol for Row {}
    unsafe impl NSAccessibility for Row {
        #[unsafe(method(isAccessibilityElement))]
        fn is_element(&self) -> bool {
            self.with(|_, _, _, _, _| true).unwrap_or(false)
        }
        #[unsafe(method_id(accessibilityRole))]
        fn role(&self) -> Option<Retained<NSString>> {
            self.with(|_, _, _, _, _| unsafe { NSAccessibilityRowRole.retain() })
        }
        #[unsafe(method_id(accessibilityRoleDescription))]
        fn role_description(&self)->Option<Retained<NSString>>{self.accessibilityRole().and_then(|role|NSAccessibilityRoleDescription(&role,None))}
        #[unsafe(method_id(accessibilityIdentifier))]
        fn identifier(&self) -> Option<Retained<NSString>> {
            self.with(|_, _, node, _, item| {
                ns(&format!("{}/{}/{}", node.id, item.key, self.ivars().token))
            })
        }
        #[unsafe(method_id(accessibilityLabel))]
        fn label(&self) -> Option<Retained<NSString>> {
            self.with(|_, _, node, _, item| ns(&node.tree_model.as_ref().and_then(|t|t.row(&item.key)).map_or_else(||item.label(),|r|r.item.accessible_label)))
        }
        #[unsafe(method(accessibilityIndex))]
        fn index(&self) -> isize {
            self.with(|_, _, _, model, item| model.index_of(&item.key).unwrap() as isize)
                .unwrap_or(-1)
        }
        #[unsafe(method(isAccessibilityEnabled))]
        fn enabled(&self) -> bool {
            self.with(|_, _, node, _, item| node.enabled&&item.enabled).unwrap_or(false)
        }
        #[unsafe(method(isAccessibilitySelected))]
        fn selected(&self) -> bool {
            self.with(|_, _, _, model, item| model.is_selected(&item.key))
                .unwrap_or(false)
        }
        #[unsafe(method(isAccessibilityFocused))]
        fn focused(&self)->bool{self.with(|m,_,node,model,item|model.cursor().as_deref()==Some(item.key.as_str())&&m.host.snapshot().is_ok_and(|nodes|nodes.iter().any(|n|n.focused&&(n.id==node.id||n.kind==ui::Kind::CollectionRow&&n.key.as_deref()==Some(item.key.as_str())&&n.parent==Some(node.id))))).unwrap_or(false)}
        #[unsafe(method(setAccessibilitySelected:))]
        fn select(&self,value:bool){self.with(|m,_,node,model,item|{let mut keys=model.selected();if value{if model.list_options().selection==ui::ListSelectionMode::Single{keys.clear();}keys.insert(item.key.clone());}else{keys.remove(&item.key);}let _=m.host.set_collection_selection(node.id,&keys.into_iter().collect::<Vec<_>>());});}
        #[unsafe(method(isAccessibilityDisclosed))]
        fn disclosed(&self)->bool{self.tree_info().is_some_and(|r|r.item.expanded)}
        #[unsafe(method(isAccessibilityExpanded))]
        fn expanded(&self)->bool{self.isAccessibilityDisclosed()}
        #[unsafe(method(setAccessibilityDisclosed:))]
        fn set_disclosed(&self,value:bool){self.with(|m,_,node,_,item|{let _=m.host.set_tree_disclosed(node.id,&item.key,self.ivars().token,value);});}
        #[unsafe(method(setAccessibilityExpanded:))]
        fn set_expanded(&self,value:bool){self.setAccessibilityDisclosed(value)}
        #[unsafe(method(accessibilityDisclosureLevel))]
        fn level(&self)->isize{self.tree_info().map_or(0,|r|r.item.depth.min(isize::MAX as usize) as isize)}
        #[unsafe(method_id(accessibilityDisclosedByRow))]
        fn disclosed_by(&self)->Option<Retained<AnyObject>>{self.with(|_,_,node,_,item|{let tree=node.tree_model.as_ref()?;let key=tree.row(&item.key)?.item.parent?;let token=tree.item_token(&key)?;Some(any(row(self.ivars().owner.clone(),node.id,key,token)))}).flatten()}
        #[unsafe(method_id(accessibilityDisclosedRows))]
        fn disclosed_rows(&self)->Option<Retained<AnyObject>>{self.disclosed_array(0,usize::MAX).map(any)}
        #[unsafe(method_id(accessibilityHelp))]
        fn help(&self)->Option<Retained<NSString>>{self.tree_info().and_then(|r|r.item.error.map(|e|ns(&e)))}
        #[unsafe(method_id(accessibilityValueDescription))]
        fn value_description(&self)->Option<Retained<NSString>>{self.tree_info().and_then(|r|if r.item.loading{Some(ns("Loading"))}else{r.item.error.map(|e|ns(&format!("Loading failed: {e}")))})}
        #[unsafe(method(accessibilityPerformConfirm))]
        fn confirm(&self)->bool{self.with(|m,_,node,_,item|m.host.activate_tree_item(node.id,&item.key,self.ivars().token).unwrap_or(false)).unwrap_or(false)}
        #[unsafe(method_id(accessibilityChildren))]
        fn children(&self)->Option<Retained<NSArray<AnyObject>>>{self.with(|m,_,node,_,item|{
            let nodes=m.host.snapshot().unwrap_or_default();let realized=nodes.iter().find(|n|n.parent==Some(node.id)&&n.key.as_deref()==Some(item.key.as_str()));
            if node.tree_model.is_none(){return realized.map_or_else(NSArray::new,|r|super::accessibility::list(m,Some(r.id)).unwrap_or_else(|_|NSArray::new()))}
            let retry=realized.and_then(|root|nodes.iter().find(|n|n.key.as_deref()==Some("$retry")&&within(&nodes,n.id,root.id)));
            let values:Vec<_>=retry.map(|n|vec![any(element(m,n.id))]).unwrap_or_default();NSArray::from_retained_slice(&values)
        })}
        #[unsafe(method(isAccessibilitySelectorAllowed:))]
        unsafe fn allowed(&self,selector:objc2::runtime::Sel)->bool{self.with(|_,_,node,_,item|{
            let info=node.tree_model.as_ref().and_then(|t|t.row(&item.key));let enabled=node.enabled&&item.enabled;
            match selector.name().to_bytes(){b"setAccessibilityDisclosed:"|b"setAccessibilityExpanded:"=>enabled&&info.as_ref().is_some_and(|r|r.item.expandable),b"isAccessibilityDisclosed"|b"isAccessibilityExpanded"|b"accessibilityDisclosedRows"=>info.as_ref().is_some_and(|r|r.item.expandable),b"accessibilityPerformConfirm"=>enabled&&info.is_some_and(|r|r.activatable),b"setAccessibilitySelected:"=>enabled&&node.combo_popup.is_none()&&node.table_model.is_none(),b"accessibilityPerformPress"|b"setAccessibilityFocused:"=>enabled,_=>true}
        }).unwrap_or(false)}
        #[unsafe(method_id(accessibilityParent))]
        fn parent(&self) -> Option<Retained<AnyObject>> {
            self.with(|m, _, node, _, _| any(element(m, node.id)))
        }
        #[unsafe(method_id(accessibilityWindow))]
        fn window(&self) -> Option<Retained<AnyObject>> {
            self.with(|_, s, _, _, _| any(s.native_window().retain()))
        }
        #[unsafe(method(accessibilityFrame))]
        fn frame(&self) -> NSRect {
            self.with(|_, s, node, model, item| {
                let y = node.bounds.y
                    + model.index_of(&item.key).unwrap() as f64 * model.row_height()
                    - model.offset();
                let r = ui::Rect::new(node.bounds.x, y, node.bounds.width, model.row_height())
                    .intersection(node.clip);
                s.native_window()
                    .convertRectToScreen(s.native_view().convertRect_toView(
                        NSRect::new(NSPoint::new(r.x, r.y), NSSize::new(r.width, r.height)),
                        None,
                    ))
            })
            .unwrap_or(NSRect::ZERO)
        }
        #[unsafe(method(accessibilityPerformPress))]
        fn press(&self) -> bool {
            self.with(|m, _, node, model, item| {
                m.host
                    .select_collection_item(node.id, model.index_of(&item.key).unwrap(), false)
                    .unwrap_or(false)
            })
            .unwrap_or(false)
        }
        #[unsafe(method(setAccessibilityFocused:))]
        fn focus(&self, value: bool) {
            if value {
                self.with(|m,_,node,model,item|{
                    m.host.select_collection_item_with(node.id,model.index_of(&item.key).unwrap(),ui::ListSelection::Focus).unwrap_or(false)
                });
            }
        }
    }
    impl Row {
        #[unsafe(method(accessibilityArrayAttributeCount:))]
        fn count_attribute(&self,attribute:&NSString)->usize {if attribute.to_string()=="AXDisclosedRows"{self.with(|_,_,node,_,item|node.tree_model.as_ref().map_or(0,|t|t.disclosed_count(&item.key))).unwrap_or(0)}else if attribute.to_string()=="AXChildren"{self.accessibilityChildren().map_or(0,|a|a.len())}else{0}}
        #[unsafe(method_id(accessibilityArrayAttributeValues:index:maxCount:))]
        fn range_attribute(&self,attribute:&NSString,index:usize,count:usize)->Option<Retained<NSArray<AnyObject>>>{if attribute.to_string()=="AXDisclosedRows"{self.disclosed_array(index,count)}else if attribute.to_string()=="AXChildren"{self.accessibilityChildren().map(|a|NSArray::from_retained_slice(&a.iter().skip(index).take(count).collect::<Vec<_>>()))}else{None}}
    }
);
impl Row {
    pub(super) fn target(&self) -> super::notification_state::Target {
        super::notification_state::Target::Row(
            self.ivars().parent,
            self.ivars().key.clone(),
            self.ivars().token,
        )
    }
    pub(super) fn selection_key(
        &self,
        m: &Mounted,
        parent: NodeId,
        model: &ListModel,
    ) -> Option<String> {
        let owner = self.ivars().owner.upgrade()?;
        if !std::ptr::eq(&*owner, m)
            || self.ivars().parent != parent
            || model.item_token(&self.ivars().key) != Some(self.ivars().token)
        {
            return None;
        }
        Some(self.ivars().key.clone())
    }
    fn tree_info(&self) -> Option<ui::TreeRowInfo> {
        self.with(|_, _, node, _, item| node.tree_model.as_ref().and_then(|t| t.row(&item.key)))
            .flatten()
    }
    fn disclosed_array(&self, start: usize, count: usize) -> Option<Retained<NSArray<AnyObject>>> {
        self.with(|_, _, node, _, item| {
            let tree = node.tree_model.as_ref()?;
            if let Some(snapshot) = tree.disclosed_snapshot(&item.key) {
                let end = start.saturating_add(count).min(snapshot.len());
                Some(super::row_array::array(
                    self.ivars().owner.clone(),
                    node.id,
                    super::row_array::Source::Children(snapshot, start.min(end)..end),
                ))
            } else {
                Some(NSArray::new())
            }
        })
        .flatten()
    }
    fn with<T>(
        &self,
        action: impl FnOnce(&Mounted, &Surface, &Node, &ListModel, &ListItem) -> T,
    ) -> Option<T> {
        let m = self.ivars().owner.upgrade()?;
        m.run(|m, s| {
            let node = m
                .host
                .snapshot()?
                .into_iter()
                .find(|n| n.id == self.ivars().parent && !n.hidden && m.host.in_scope(n.id));
            Ok(node.and_then(|node| {
                let model = node.collection.as_ref()?;
                if model.item_token(&self.ivars().key) != Some(self.ivars().token) {
                    return None;
                }
                let item = model.item(model.index_of(&self.ivars().key)?)?;
                Some(action(m, s, &node, model, &item))
            }))
        })
        .flatten()
    }
}
impl Element {
    pub(super) fn rows_all(&self) -> Option<Retained<NSArray<AnyObject>>> {
        self.with(|_, _, node| {
            Ok(node.collection.as_ref().map(|m| {
                super::row_array::array(
                    self.ivars().owner.clone(),
                    node.id,
                    super::row_array::Source::Range(m.snapshot(), 0..m.len()),
                )
            }))
        })
        .flatten()
    }
    pub(super) fn rows_visible(
        &self,
        start: usize,
        count: usize,
    ) -> Option<Retained<NSArray<AnyObject>>> {
        self.with(|_, _, node| {
            Ok(node.collection.as_ref().map(|m| {
                let range = viewport(m);
                let end = start.saturating_add(count).min(range.len());
                super::row_array::array(
                    self.ivars().owner.clone(),
                    node.id,
                    super::row_array::Source::Range(
                        m.snapshot(),
                        range.start + start.min(end)..range.start + end,
                    ),
                )
            }))
        })
        .flatten()
    }
    pub(super) fn collection_selected(
        &self,
        start: usize,
        count: usize,
    ) -> Option<Retained<NSArray<AnyObject>>> {
        self.with(|_, _, node| {
            Ok(node.collection.as_ref().map(|m| {
                let indices = m
                    .selected_range(start, count)
                    .iter()
                    .filter_map(|key| m.index_of(key))
                    .collect();
                super::row_array::array(
                    self.ivars().owner.clone(),
                    node.id,
                    super::row_array::Source::Indices(m.snapshot(), indices),
                )
            }))
        })
        .flatten()
    }
    pub(super) fn set_collection_selection(&self, values: Option<&NSArray<AnyObject>>) {
        self.with(|m, _, node| {
            if node.collection.is_none() {
                return Ok(false);
            }
            let Some(model) = &node.collection else {
                return Ok(false);
            };
            let mut keys = vec![];
            if let Some(values) = values {
                for value in values {
                    let Some(key) = value
                        .downcast_ref::<Row>()
                        .and_then(|r| r.selection_key(m, node.id, model))
                    else {
                        return Ok(false);
                    };
                    keys.push(key);
                }
            }
            Ok(m.host
                .set_collection_selection(node.id, &keys)
                .unwrap_or(false))
        });
    }
    pub(super) fn row_array(
        &self,
        start: usize,
        count: usize,
    ) -> Option<Retained<NSArray<AnyObject>>> {
        self.with(|m, _, node| {
            if node.collection.is_some() {
                Ok(rows(self.ivars().owner.clone(), node, start, count))
            } else {
                let children = super::accessibility::list(m, Some(node.id))?;
                let values: Vec<_> = children.iter().skip(start).take(count).collect();
                Ok(NSArray::from_retained_slice(&values))
            }
        })
    }
}

fn within(nodes: &[Node], mut id: NodeId, parent: NodeId) -> bool {
    loop {
        if id == parent {
            return true;
        }
        let Some(next) = nodes.iter().find(|n| n.id == id).and_then(|n| n.parent) else {
            return false;
        };
        id = next;
    }
}
