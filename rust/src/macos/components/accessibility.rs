use super::*;
use objc2::Message;
use ui::{Kind, Node, NodeId, Value};
fn any<T: Message>(value: Retained<T>) -> Retained<AnyObject> {
    // SAFETY: Upcast a live Objective-C object to id.
    unsafe { Retained::cast_unchecked(value) }
}
pub(super) fn semantic(n: &Node) -> bool {
    if n.table_root.is_some() {
        return !n.hidden;
    }
    !n.hidden
        && !n.combo_toggle
        && !(n.kind == Kind::Modal && n.overlay_owner.is_some() && n.combo_owner.is_some())
        && !n.tab_strip
        && !matches!(
            n.kind,
            Kind::Fragment
                | Kind::Column
                | Kind::Row
                | Kind::Block
                | Kind::Inline
                | Kind::Layer
                | Kind::Spacer
                | Kind::Separator
        )
}
pub(super) fn tab_group(n: &Node, nodes: &[Node]) -> Option<NodeId> {
    let mut parent = n.parent;
    while let Some(id) = parent {
        let node = nodes.iter().find(|n| n.id == id)?;
        if node.kind == Kind::Tabs {
            return Some(id);
        }
        parent = node.parent;
    }
    None
}
fn menu_scope(n: &Node, nodes: &[Node]) -> Option<NodeId> {
    if n.menu_root {
        return Some(n.id);
    }
    let mut parent = n.parent;
    while let Some(id) = parent {
        let p = nodes.iter().find(|n| n.id == id)?;
        if p.menu_root {
            return Some(id);
        }
        parent = p.parent;
    }
    None
}
fn parent(n: &Node, nodes: &[Node]) -> Option<NodeId> {
    if n.menu_panel && !n.menu_path.is_empty() {
        return nodes
            .iter()
            .find(|item| {
                item.menu_item
                    .as_ref()
                    .is_some_and(|info| info.path == n.menu_path)
                    && menu_scope(item, nodes) == menu_scope(n, nodes)
            })
            .map(|n| n.id);
    }
    if let Some(key) = &n.combo_owner {
        return nodes
            .iter()
            .find(|n| n.key.as_ref() == Some(key) && n.combo.is_some())
            .map(|n| n.id);
    }
    let mut id = n.parent;
    while let Some(p) = id {
        let node = nodes.iter().find(|n| n.id == p)?;
        if semantic(node) {
            return Some(p);
        }
        id = node.parent;
    }
    None
}
pub(super) fn element(m: &Mounted, id: NodeId) -> Retained<Element> {
    if let Some(element) = m.cache.borrow().get(&id) {
        return element.clone();
    }
    let owner = m
        .surface
        .upgrade()
        .and_then(|s| s.state.components.borrow().as_ref().map(Rc::downgrade))
        .unwrap_or_default();
    let allocated =
        Element::alloc(MainThreadMarker::new().unwrap()).set_ivars(ElementIvars { owner, id });
    // SAFETY: NSAccessibilityElement's designated NSObject initializer; ivars are initialized.
    let element: Retained<Element> = unsafe { msg_send![super(allocated), init] };
    m.cache.borrow_mut().insert(id, element.clone());
    m.prime_notification(super::notification_state::Target::Node(id));
    element
}
pub(super) fn list(
    m: &Mounted,
    parent_id: Option<NodeId>,
) -> std::result::Result<Retained<NSArray<AnyObject>>, ui::Error> {
    let nodes = m.host.snapshot()?;
    let elements: Vec<_> = nodes
        .iter()
        .filter(|n| {
            semantic(n)
                && !super::table_accessibility::scaffold(&nodes, n, parent_id)
                && super::control_accessibility::readable(m, n, &nodes)
                && parent(n, &nodes).filter(|id| {
                    nodes
                        .iter()
                        .find(|n| n.id == *id)
                        .is_some_and(|n| super::control_accessibility::readable(m, n, &nodes))
                }) == parent_id
        })
        .map(|n| any(element(m, n.id)))
        .collect();
    Ok(NSArray::from_retained_slice(&elements))
}
pub(crate) fn children(s: &Rc<SurfaceState>) -> Option<Retained<NSArray<AnyObject>>> {
    let m = s.components.borrow().clone()?;
    m.run(|m, _| list(m, None))
}
pub(crate) fn focused(s: &Rc<SurfaceState>) -> Option<Retained<AnyObject>> {
    let m = s.components.borrow().clone()?;
    m.run(|m, _| Ok(focused_element(m))).flatten()
}
pub(super) fn focused_element(m: &Mounted) -> Option<Retained<AnyObject>> {
    let id = m.host.focused()?;
    let nodes = m.host.snapshot().ok()?;
    let n = nodes.iter().find(|n| {
        n.id == id && n.focused && !n.hidden && super::control_accessibility::readable(m, n, &nodes)
    })?;
    if let Some(value) = super::table_accessibility::logical(m, id, true) {
        return Some(value);
    }
    if let Some(tree) = &n.tree_model
        && let Some(info) = tree.cursor().and_then(|key| tree.row(&key))
    {
        let owner = m
            .surface
            .upgrade()?
            .state
            .components
            .borrow()
            .as_ref()
            .map(Rc::downgrade)?;
        return Some(any(super::collection_accessibility::row(
            owner,
            n.id,
            info.item.key,
            info.token,
        )));
    }
    Some(super::collection_accessibility::logical(m, id).unwrap_or_else(|| any(element(m, id))))
}
pub(crate) fn hit_test(s: &Rc<SurfaceState>, point: NSPoint) -> Option<Retained<AnyObject>> {
    let m = s.components.borrow().clone()?;
    m.run(|m, s| {
        let local = s.native_view().convertPoint_fromView(
            s.native_window()
                .convertRectFromScreen(NSRect::new(point, NSSize::ZERO))
                .origin,
            None,
        );
        let p = ui::Point {
            x: local.x,
            y: local.y,
        };
        let nodes = m.host.snapshot()?;
        Ok(m.host
            .scene()?
            .iter()
            .rev()
            .find(|n| {
                semantic(n)
                    && super::control_accessibility::readable(m, n, &nodes)
                    && n.clip.contains(p)
                    && n.bounds.contains(p)
            })
            .map(|n| {
                super::table_accessibility::logical(m, n.id, false)
                    .or_else(|| super::collection_accessibility::logical(m, n.id))
                    .unwrap_or_else(|| any(element(m, n.id)))
            }))
    })
    .flatten()
}
fn screen(s: &Surface, r: ui::Rect) -> NSRect {
    s.native_window()
        .convertRectToScreen(s.native_view().convertRect_toView(
            NSRect::new(NSPoint::new(r.x, r.y), NSSize::new(r.width, r.height)),
            None,
        ))
}
fn numeric(value: &str) -> Option<Retained<AnyObject>> {
    if let Ok(v) = value.parse::<i64>() {
        Some(any(NSNumber::new_i64(v)))
    } else if let Ok(v) = value.parse::<u64>() {
        Some(any(NSNumber::new_u64(v)))
    } else {
        value
            .parse::<f64>()
            .ok()
            .filter(|v| v.is_finite())
            .map(|v| any(NSNumber::new_f64(v)))
    }
}
fn role(kind: Kind) -> &'static NSString {
    // SAFETY: Immutable role constants exported by AppKit.
    unsafe {
        match kind {
            Kind::Label => NSAccessibilityStaticTextRole,
            Kind::Button | Kind::Toggle => NSAccessibilityButtonRole,
            Kind::CheckBox | Kind::Switch => NSAccessibilityCheckBoxRole,
            Kind::Radio | Kind::Segment | Kind::Tab => NSAccessibilityRadioButtonRole,
            Kind::Tabs => NSAccessibilityTabGroupRole,
            Kind::RadioGroup | Kind::Segmented => NSAccessibilityRadioGroupRole,
            Kind::Slider => NSAccessibilitySliderRole,
            Kind::Stepper => NSAccessibilityIncrementorRole,
            Kind::Split => NSAccessibilitySplitGroupRole,
            Kind::SplitDivider => NSAccessibilitySplitterRole,
            Kind::Progress => NSAccessibilityProgressIndicatorRole,
            Kind::Image => NSAccessibilityImageRole,
            Kind::TextField => NSAccessibilityTextFieldRole,
            Kind::Scroll => NSAccessibilityScrollAreaRole,
            _ => NSAccessibilityGroupRole,
        }
    }
}
pub(super) struct ElementIvars {
    pub(super) owner: Weak<Mounted>,
    pub(super) id: NodeId,
}
define_class!(
    // SAFETY: NSAccessibilityElement is subclassable; selectors match AppKit's protocol.
    #[unsafe(super=NSAccessibilityElement)]
    #[thread_kind=MainThreadOnly]
    #[ivars=ElementIvars]
    pub(super) struct Element;
    unsafe impl NSObjectProtocol for Element {}
    impl Element {
        #[unsafe(method(scarsLogicalRowCacheCount))]
        fn logical_cache_count(&self)->usize{self.with(|m,_,_|Ok(m.row_cache.borrow().len())).unwrap_or(0)}
        #[unsafe(method(scarsTableEntryCacheCount))]
        fn table_cache_count(&self)->usize{self.with(|m,_,_|Ok(m.table_cache.borrow().len())).unwrap_or(0)}
    }
    unsafe impl NSAccessibility for Element {
        #[unsafe(method(isAccessibilityElement))]
        fn is_element(&self) -> bool {
            self.node().is_some()
        }
        #[unsafe(method_id(accessibilityRole))]
        fn role(&self) -> Option<Retained<NSString>> {
            self.node().map(|n| {
                if n.color_well.is_some(){unsafe{NSAccessibilityColorWellRole.retain()}}
                else if n.table_root.is_some() {unsafe {NSAccessibilityTableRole.retain()}}
                else if n.menu.as_ref().is_some_and(|m|m.trigger){unsafe{NSAccessibilityMenuButtonRole.retain()}}
                else if n.kind==Kind::Modal&&n.anchor.is_some()&&!n.menu_root&&n.combo_popup.is_none(){unsafe{NSAccessibilityPopoverRole.retain()}}
                else if n.kind==Kind::TextField&&n.multiline{unsafe{NSAccessibilityTextAreaRole.retain()}}
                else if n.combo.is_some() {unsafe {NSAccessibilityComboBoxRole.retain()}}
                else if n.menu_item.is_some() { unsafe { NSAccessibilityMenuItemRole.retain() } }
                else if n.menu_panel { unsafe { NSAccessibilityMenuRole.retain() } }
                else if n.tree_model.is_some(){unsafe{NSAccessibilityOutlineRole.retain()}}
                else if n.collection.is_some() {
                    unsafe { NSAccessibilityListRole.retain() }
                } else {
                    ns(&role(n.kind).to_string())
                }
            })
        }
        #[unsafe(method_id(accessibilityRoleDescription))] fn role_description(&self)->Option<Retained<NSString>>{self.accessibilityRole().and_then(|role|NSAccessibilityRoleDescription(&role,self.accessibilitySubrole().as_deref()))}
        #[unsafe(method_id(accessibilityLabel))]
        fn label(&self) -> Option<Retained<NSString>> {
            self.node().map(|n| ns(&n.label))
        }
        #[unsafe(method_id(accessibilityHelp))]
        fn help(&self) -> Option<Retained<NSString>> {
            self.node().and_then(|n|super::control_accessibility::help(&n))
        }
        #[unsafe(method(isAccessibilityExpanded))]
        fn expanded(&self) -> bool {
            self.node().is_some_and(|n| n.menu.as_ref().is_some_and(|m|m.trigger&&m.expanded)||n.combo.as_ref().is_some_and(|c|c.open) || n.menu_item.is_some_and(|m|m.expanded))
        }
        #[unsafe(method_id(accessibilityIdentifier))]
        fn identifier(&self) -> Option<Retained<NSString>> {
            self.node()
                .map(|n| ns(&format!("node/{}/{}",n.id,n.key.as_deref().unwrap_or(""))))
        }
        #[unsafe(method(accessibilityFrame))]
        fn frame(&self) -> NSRect {
            self.with(|_, s, n| Ok(screen(s, n.clip)))
                .unwrap_or(NSRect::ZERO)
        }
        #[unsafe(method_id(accessibilityParent))]
        fn parent(&self) -> Option<Retained<AnyObject>> {
            self.with(|m, s, n| {
                if let Some(parent)=super::table_accessibility::editor_parent(m,n){return Ok(parent)}
                let nodes = m.host.snapshot()?;
                Ok(parent(n, &nodes)
                    .filter(|id|nodes.iter().find(|n|n.id==*id).is_some_and(|n|super::control_accessibility::readable(m,n,&nodes)))
                    .map(|id|super::table_accessibility::logical(m,id,false).or_else(||super::collection_accessibility::logical(m,id)).unwrap_or_else(||any(element(m,id))))
                    .unwrap_or_else(|| any(s.native_view().retain())))
            })
        }
        #[unsafe(method_id(accessibilityWindow))]
        fn window(&self) -> Option<Retained<AnyObject>> {
            self.with(|_, s, _| Ok(any(s.native_window().retain())))
        }
        #[unsafe(method_id(accessibilityTopLevelUIElement))]
        fn top(&self) -> Option<Retained<AnyObject>> {
            self.accessibilityWindow()
        }
        #[unsafe(method_id(accessibilityChildren))]
        fn children(&self) -> Option<Retained<NSArray<AnyObject>>> {
            self.table_array("AXChildren",0,usize::MAX).or_else(||self.with(|m, _, n| {
                if let Some(model) = &n.collection {
                    let range = 0..model.len();
                    Ok(super::collection_accessibility::rows(
                        self.ivars().owner.clone(),
                        n,
                        range.start,
                        range.len(),
                    ))
                } else {
                    list(m, Some(n.id))
                }
            }))
        }
        #[unsafe(method_id(accessibilityLinkedUIElements))]
        fn linked(&self)->Option<Retained<NSArray<AnyObject>>> {
            self.with(|m,_,n|{
                let nodes=m.host.snapshot()?;
                let linked:Vec<_>=if let Some(info)=&n.menu{nodes.iter().filter(|p|p.menu.as_ref().is_some_and(|other|other.id==info.id&&other.trigger!=info.trigger)).map(|p|any(element(m,p.id))).collect()}
                else if n.combo.is_some(){nodes.iter().filter(|p|p.kind!=Kind::Modal&&p.combo_owner.as_ref()==n.key.as_ref()).map(|p|any(element(m,p.id))).collect()}
                else {n.combo_owner.as_ref().and_then(|key|nodes.iter().find(|p|p.key.as_ref()==Some(key)&&p.combo.is_some())).map(|p|vec![any(element(m,p.id))]).unwrap_or_default()};
                Ok(NSArray::from_retained_slice(&linked))
            })
        }
        #[unsafe(method_id(accessibilityTabs))]
        fn tabs(&self)->Option<Retained<NSArray<AnyObject>>>{self.tab_elements(1)}
        #[unsafe(method_id(accessibilitySelectedChildren))]
        fn selected_children(&self)->Option<Retained<NSArray<AnyObject>>>{self.table_array("AXSelectedChildren",0,usize::MAX).or_else(||self.calendar_selected()).or_else(||self.combo_selected()).or_else(||self.collection_selected(0,usize::MAX)).or_else(||self.choices(true)).or_else(||self.tab_elements(0))}
        #[unsafe(method_id(accessibilityContents))]
        fn contents(&self)->Option<Retained<NSArray<AnyObject>>>{self.tab_elements(2)}
        #[unsafe(method_id(accessibilityRows))]
        fn rows(&self)->Option<Retained<NSArray<AnyObject>>>{self.table_array("AXRows",0,usize::MAX).or_else(||self.rows_all())}
        #[unsafe(method_id(accessibilityVisibleChildren))] fn visible_children(&self)->Option<Retained<NSArray<AnyObject>>>{self.rows_visible(0,usize::MAX)}
        #[unsafe(method_id(accessibilityVisibleRows))]
        fn visible_rows(&self)->Option<Retained<NSArray<AnyObject>>>{self.table_array("AXVisibleRows",0,usize::MAX).or_else(||self.rows_visible(0,usize::MAX))}
        #[unsafe(method_id(accessibilitySelectedRows))]
        fn selected_rows(&self)->Option<Retained<NSArray<AnyObject>>>{self.table_array("AXSelectedRows",0,usize::MAX).or_else(||self.collection_selected(0,usize::MAX))}
        #[unsafe(method(setAccessibilitySelectedRows:))]
        fn set_selected_rows(&self,values:Option<&NSArray<AnyObject>>){if self.node().is_some_and(|n|n.table_root.is_some()){self.table_selection(values,false)}else{self.set_collection_selection(values)}}
        #[unsafe(method_id(accessibilityHeader))] fn header(&self)->Option<Retained<AnyObject>>{self.with(|_,_,n|Ok(n.table_root.as_ref().map(|_|super::table_accessibility::entry(self.ivars().owner.clone(),n.id,super::table_accessibility::Part::Headers)))).flatten()}
        #[unsafe(method_id(accessibilitySelectedColumns))] fn selected_columns(&self)->Option<Retained<NSArray<AnyObject>>>{self.with(|_,_,n|Ok(n.table_root.as_ref().map(|_|NSArray::new()))).flatten()}
        #[unsafe(method(setAccessibilitySelectedColumns:))] fn set_selected_columns(&self,_values:Option<&NSArray<AnyObject>>){}
        #[unsafe(method_id(accessibilityColumns))] fn columns(&self)->Option<Retained<NSArray<AnyObject>>>{self.table_array("AXColumns",0,usize::MAX)}
        #[unsafe(method_id(accessibilityVisibleColumns))] fn visible_columns(&self)->Option<Retained<NSArray<AnyObject>>>{self.table_array("AXVisibleColumns",0,usize::MAX)}
        #[unsafe(method_id(accessibilityColumnHeaderUIElements))] fn headers(&self)->Option<Retained<NSArray<AnyObject>>>{self.table_array("AXColumnHeaderUIElements",0,usize::MAX)}
        #[unsafe(method_id(accessibilitySelectedCells))] fn selected_cells(&self)->Option<Retained<NSArray<AnyObject>>>{self.table_array("AXSelectedCells",0,usize::MAX)}
        #[unsafe(method(setAccessibilitySelectedCells:))] fn select_cells(&self,values:Option<&NSArray<AnyObject>>){self.table_selection(values,true)}
        #[unsafe(method_id(accessibilityVisibleCells))] fn visible_cells(&self)->Option<Retained<NSArray<AnyObject>>>{self.table_array("AXVisibleCells",0,usize::MAX)}
        #[unsafe(method_id(accessibilityCellForColumn:row:))] fn cell_at(&self,column:isize,row:isize)->Option<Retained<AnyObject>>{self.table_cell(column,row)}
        #[unsafe(method(accessibilityColumnCount))] fn column_count(&self)->isize{self.node().and_then(|n|n.table_root).map_or(0,|t|t.column_count() as isize)}
        #[unsafe(method(accessibilityRowCount))]
        fn row_count(&self) -> isize {
            self.node().map_or(0,|n|n.table_root.map_or_else(||n.collection.map_or(0,|m|m.len() as isize),|t|t.row_count() as isize))
        }
        #[unsafe(method_id(accessibilitySubrole))]
        fn subrole(&self)->Option<Retained<NSString>>{
            self.node().and_then(|n|unsafe{if n.kind==Kind::Tab {Some(NSAccessibilityTabButtonSubrole.retain())}else if n.kind==Kind::Toggle{Some(NSAccessibilityToggleSubrole.retain())}else if n.kind==Kind::Modal&&n.anchor.is_none()&&!n.menu_root{Some(NSAccessibilityDialogSubrole.retain())}else{match n.field_kind{
                ui::TextFieldKind::Password=>Some(NSAccessibilitySecureTextFieldSubrole.retain()),
                ui::TextFieldKind::Search=>Some(NSAccessibilitySearchFieldSubrole.retain()),
                _=>None,
            }}})
        }
        #[unsafe(method(isAccessibilityEnabled))]
        fn enabled(&self) -> bool {
            self.node().is_some_and(|n| n.enabled)
        }
        #[unsafe(method(isAccessibilityFocused))]
        fn focused(&self) -> bool {
            self.with(|m,_,n|Ok(if let Some(t)=&n.table_root {
                let no_cursor=if t.options().selection_mode==ui::TableSelectionMode::Rows{t.row_cursor().is_none()}else{t.cursor().is_none()};
                let nodes=m.host.snapshot()?;
                no_cursor&&nodes.iter().any(|body|body.focused&&body.table_model.as_ref()==Some(t)&&super::table_accessibility::within(&nodes,body.id,n.id))
            }else{n.focused&&n.tree_model.as_ref().is_none_or(|t|t.cursor().is_none())})).unwrap_or(false)
        }
        #[unsafe(method(setAccessibilityFocused:))]
        fn focus(&self, value: bool) {
            if value {
                self.with(|m, s, n| {
                    s.focus();
                    if let Some(table)=&n.table_root {
                        let nodes=m.host.snapshot()?;
                        if let Some(body)=nodes.iter().find(|body|body.table_model.as_ref()==Some(table)&&super::table_accessibility::within(&nodes,body.id,n.id)) {return m.host.focus(body.id)}
                    }
                    m.host.focus(n.id)
                });
            }
        }
        #[unsafe(method_id(accessibilityValue))]
        fn value(&self)->Option<Retained<AnyObject>>{
            self.node().filter(|n|!n.secure).and_then(|n|{
                if let Some(color)=n.color_well{return Some(any(ns(&color.to_string())))}
                if n.kind==Kind::Tabs{return self.tab_elements(0).and_then(|tabs|tabs.into_iter().next())}
                if matches!(n.kind,Kind::RadioGroup|Kind::Segmented){return self.choices(true).and_then(|a|a.into_iter().next())}
                if let Some(checked)=n.menu_item.as_ref().and_then(|m|m.checked){return Some(any(NSNumber::new_bool(checked)))}
                if let Some(stepper)=&n.stepper{return numeric(&stepper.value)}
                match n.value{
                    Value::Text(v)=>Some(any(ns(&v))),
                    Value::Bool(v)=>Some(any(NSNumber::new_bool(v))),
                    Value::Check(v)=>Some(any(NSNumber::new_i32(match v{ui::CheckState::Unchecked=>0,ui::CheckState::Checked=>1,ui::CheckState::Indeterminate=>2}))),
                    Value::Number(v)=>Some(any(NSNumber::new_f64(v))),
                    Value::None if n.kind==Kind::Label=>Some(any(ns(&n.text))),
                    _=>None,
                }
            })
        }
        #[unsafe(method(setAccessibilityValue:))]
        unsafe fn set_value(&self, value: Option<&AnyObject>) {
            self.set_control_value(value);
        }

        #[unsafe(method(accessibilityOrientation))]
        fn orientation(&self)->NSAccessibilityOrientation{
            if self.node().is_some_and(|n|n.kind==Kind::Tabs){return NSAccessibilityOrientation::Horizontal;}
            self.node().and_then(|n|n.split.map(|s|if (s.orientation==ui::Orientation::Vertical)!=(n.kind==Kind::SplitDivider){NSAccessibilityOrientation::Vertical}else{NSAccessibilityOrientation::Horizontal})).unwrap_or(NSAccessibilityOrientation::Unknown)
        }
        #[unsafe(method_id(accessibilityValueDescription))]
        fn value_description(&self)->Option<Retained<NSString>>{
            self.node().and_then(|n|super::control_accessibility::description(&n))
        }
        #[unsafe(method_id(accessibilityPreviousContents))]
        fn previous_contents(&self)->Option<Retained<NSArray<AnyObject>>>{self.split_contents(true)}
        #[unsafe(method_id(accessibilityNextContents))]
        fn next_contents(&self)->Option<Retained<NSArray<AnyObject>>>{self.split_contents(false)}
        #[unsafe(method_id(accessibilitySplitters))]
        fn splitters(&self)->Option<Retained<NSArray<AnyObject>>>{
            self.with(|m,_,n|Ok(if n.kind==Kind::Split{Some(NSArray::from_retained_slice(&m.host.snapshot()?.iter().filter(|child|child.parent==Some(n.id) && child.kind==Kind::SplitDivider && !child.hidden).map(|child|any(element(m,child.id))).collect::<Vec<_>>()))}else{None})).flatten()
        }
        #[unsafe(method(isAccessibilityModal))]
        fn modal(&self) -> bool { self.node().is_some_and(|n| n.kind==Kind::Modal&&!n.menu_root&&n.combo_popup.is_none()&&n.overlay.is_none_or(|o|o.focus==ui::OverlayFocus::Content)) }
        #[unsafe(method_id(accessibilityDefaultButton))]
        fn default_button(&self) -> Option<Retained<AnyObject>> { self.alert_button(ui::AlertActionRole::Default) }
        #[unsafe(method_id(accessibilityCancelButton))]
        fn cancel_button(&self) -> Option<Retained<AnyObject>> { self.alert_button(ui::AlertActionRole::Cancel) }
        #[unsafe(method_id(accessibilityMinValue))]
        fn min(&self) -> Option<Retained<AnyObject>> {
            self.node()
                .and_then(|n|if let Some(stepper)=&n.stepper{numeric(&stepper.min)}else{n.range.map(|r|any(NSNumber::new_f64(r.0)))})
        }
        #[unsafe(method_id(accessibilityMaxValue))]
        fn max(&self) -> Option<Retained<AnyObject>> {
            self.node()
                .and_then(|n|if let Some(stepper)=&n.stepper{numeric(&stepper.max)}else{n.range.map(|r|any(NSNumber::new_f64(r.1)))})
        }
        #[unsafe(method(accessibilityPerformPress))]
        fn press(&self) -> bool {
            self.with(|m, _, n| if n.menu.as_ref().is_some_and(|i|i.trigger){m.host.show_menu(n.id)}else{m.host.activate(n.id)}).unwrap_or(false)
        }
        #[unsafe(method(accessibilityPerformIncrement))]
        fn increment(&self) -> bool {
            self.adjust(1.)
        }
        #[unsafe(method(accessibilityPerformDecrement))]
        fn decrement(&self) -> bool {
            self.adjust(-1.)
        }
        #[unsafe(method(accessibilityPerformShowMenu))] fn show_menu(&self)->bool{self.with(|m,_,n|m.host.show_menu(n.id)).unwrap_or(false)}
        #[unsafe(method(accessibilityPerformCancel))] fn cancel(&self)->bool{self.with(|m,_,n|m.host.cancel_presentation(n.id)).unwrap_or(false)}
        #[unsafe(method(accessibilityPerformConfirm))] fn confirm(&self)->bool{self.confirm_control()}
        #[unsafe(method(isAccessibilitySelected))] fn selected(&self)->bool{self.node().is_some_and(|n|n.calendar_day.map(|d|d.selected).or_else(||n.menu_item.and_then(|i|i.checked)).unwrap_or_else(||n.value.selected()))}
        #[unsafe(method(setAccessibilitySelected:))] fn set_selected(&self,v:bool){self.with(|m,_,n|if matches!(n.kind,Kind::Tab|Kind::Radio|Kind::Segment){m.host.set_selected(n.id,v)}else{Ok(false)});}
        #[unsafe(method(setAccessibilitySelectedChildren:))] fn set_selected_children(&self,v:Option<&NSArray<AnyObject>>){self.set_choice_children(v)}
        #[unsafe(method_id(accessibilityIncrementButton))] fn increment_button(&self)->Option<Retained<AnyObject>>{self.step_button(true)}
        #[unsafe(method_id(accessibilityDecrementButton))] fn decrement_button(&self)->Option<Retained<AnyObject>>{self.step_button(false)}
        #[unsafe(method_id(accessibilityPlaceholderValue))]
        fn placeholder(&self) -> Option<Retained<NSString>> {
            self.node()
                .filter(|n| n.kind == Kind::TextField)
                .map(|n| ns(&n.text))
        }
        #[unsafe(method(accessibilityNumberOfCharacters))]
        fn characters(&self) -> isize {
            self.node().map_or(0, |n| {
                if let Value::Text(t) = n.value {
                    t.encode_utf16().count() as isize
                } else {
                    0
                }
            })
        }
        #[unsafe(method_id(accessibilitySelectedText))]
        fn selected_text(&self) -> Option<Retained<NSString>> {
            self.with(|m, _, n| {
                Ok(m.host
                    .text_input()?
                    .filter(|i| i.id == n.id && !i.secure)
                    .map(|i| ns(&i.text[i.selection.range()])))
            })
            .flatten()
        }
        #[unsafe(method(accessibilitySelectedTextRange))]
        fn selected_range(&self) -> NSRange {
            self.with(|m, _, n| {
                Ok(m.host
                    .text_input()?
                    .filter(|i| i.id == n.id && !i.secure)
                    .and_then(|i| native_range(&i.text, i.selection.range())))
            })
            .flatten()
            .unwrap_or(NSRange::new(NSNotFound as usize, 0))
        }
        #[unsafe(method(setAccessibilitySelectedTextRange:))]
        fn select_range(&self, r: NSRange) {
            self.with(|m, _, n| {
                if n.secure {
                    return Ok(false);
                }
                let Value::Text(text) = &n.value else {
                    return Ok(false);
                };
                let Some(range) = byte_range(text, r) else {
                    return Ok(false);
                };
                m.host.focus(n.id)?;
                m.host.select_text(
                    n.id,
                    ui::Selection {
                        anchor: range.start,
                        caret: range.end,
                    },
                )
            });
        }
        #[unsafe(method_id(accessibilityStringForRange:))]
        fn string_range(&self, r: NSRange) -> Option<Retained<NSString>> {
            self.node().filter(|n| !n.secure).and_then(|n| {
                if let Value::Text(text) = n.value {
                    byte_range(&text, r).map(|r| ns(&text[r]))
                } else {
                    None
                }
            })
        }
        #[unsafe(method_id(accessibilityAttributedStringForRange:))]
        fn attributed_range(&self, r: NSRange) -> Option<Retained<NSAttributedString>> {
            self.accessibilityStringForRange(r)
                .map(|s| NSAttributedString::initWithString(NSAttributedString::alloc(), &s))
        }
        #[unsafe(method(accessibilityFrameForRange:))]
        fn frame_range(&self, r: NSRange) -> NSRect {
            self.with(|m, s, n| {
                let Value::Text(text) = &n.value else {
                    return Ok(NSRect::ZERO);
                };
                Ok(byte_range(text, r)
                    .and_then(|r| m.host.text_rectangles(n.id, r).first().copied())
                    .map_or(NSRect::ZERO, |r| screen(s, r)))
            })
            .unwrap_or(NSRect::ZERO)
        }
        #[unsafe(method(isAccessibilitySelectorAllowed:))]
        unsafe fn allowed(&self, selector: objc2::runtime::Sel) -> bool {
            self.node()
                .is_some_and(|n| match selector.name().to_bytes() {
                    b"isAccessibilityExpanded"=>n.combo.is_some()||n.menu.as_ref().is_some_and(|m|m.trigger)||n.menu_item.as_ref().is_some_and(|m|m.submenu),
                    b"setAccessibilitySelectedColumns:"=>false,
                    b"setAccessibilitySelectedRows:" if n.table_root.is_some()=>n.enabled&&n.table_root.as_ref().is_some_and(|t|t.options().selection_mode==ui::TableSelectionMode::Rows),
                    b"setAccessibilitySelectedCells:"=>n.enabled&&n.table_root.as_ref().is_some_and(|t|t.options().selection_mode==ui::TableSelectionMode::Cells),
                    b"setAccessibilitySelectedRows:"=>n.enabled&&n.collection.is_some()&&n.combo_popup.is_none(),
                    b"accessibilityPerformPress" => {
                        n.enabled
                            && (!n.read_only||matches!(n.kind,Kind::Button|Kind::TextField))
                            && matches!(
                                n.kind,
                                Kind::Button
                                    | Kind::Tab
                                    | Kind::Toggle
                                    | Kind::CheckBox
                                    | Kind::Radio
                                    | Kind::Segment
                                    | Kind::Switch
                                    | Kind::TextField
                                    | Kind::SplitDivider
                            )
                    }
                    b"setAccessibilityValue:"=>super::control_accessibility::can_set(&n),
                    b"accessibilityPerformIncrement"=>super::control_accessibility::can_adjust(&n,true),
                    b"accessibilityPerformDecrement"=>super::control_accessibility::can_adjust(&n,false),
                    b"accessibilityPerformShowMenu"=>n.enabled&&(n.context_menu||n.menu.as_ref().is_some_and(|m|m.trigger)||n.menu_item.as_ref().is_some_and(|m|m.submenu)),
                    b"accessibilityPerformCancel"=>self.with(|m,_,node|Ok(super::control_accessibility::can_cancel(m,node))).unwrap_or(false),
                    b"accessibilityPerformConfirm"=>n.enabled&&((n.alert&&self.accessibilityDefaultButton().is_some())||matches!(n.kind,Kind::Tab|Kind::Radio|Kind::Segment)),
                    b"setAccessibilitySelectedChildren:"=>n.enabled&&!n.read_only&&matches!(n.kind,Kind::Tabs|Kind::RadioGroup|Kind::Segmented),
                    b"setAccessibilitySelected:"=>n.enabled&&!n.read_only&&matches!(n.kind,Kind::Tab|Kind::Radio|Kind::Segment),
                    _ => true,
                })
        }
    }
    impl Element {
        #[unsafe(method(accessibilityArrayAttributeCount:))]
        fn array_count(&self, attribute: &NSString) -> usize {
            let name=attribute.to_string();
            if let Some(count)=self.table_count(&name){return count}
            if let Some(array)=self.control_array(&name){return array.len()}
            if matches!(name.as_str(), "AXChildren" | "AXRows") {
                self.node().and_then(|n|n.collection).map_or_else(||self.accessibilityChildren().map_or(0,|v|v.len()),|m|m.len())
            } else if name=="AXSelectedRows"||name=="AXSelectedChildren" {self.node().and_then(|n|n.collection).map_or_else(||self.accessibilitySelectedChildren().map_or(0,|a|a.len()),|m|m.selection_count())}
            else if name=="AXVisibleRows"||name=="AXVisibleChildren" {self.node().and_then(|n|n.collection).map_or(0,|m|super::collection_accessibility::viewport(&m).len())}
            else {
                0
            }
        }
        #[unsafe(method_id(accessibilityArrayAttributeValues:index:maxCount:))]
        fn indexed_rows(
            &self,
            attribute: &NSString,
            index: usize,
            count: usize,
        ) -> Option<Retained<NSArray<AnyObject>>> {
            let name=attribute.to_string();
            self.table_array(&name,index,count).or_else(||self.control_range(&name,index,count)).or_else(||{if matches!(name.as_str(), "AXChildren" | "AXRows") {
                self.row_array(index, count)
            } else if name=="AXSelectedRows"||name=="AXSelectedChildren" {self.collection_selected(index,count)}
            else if name=="AXVisibleRows"||name=="AXVisibleChildren" {self.rows_visible(index,count)}
            else {
                None
            }})
        }

    }
);
impl Element {
    fn calendar_selected(&self) -> Option<Retained<NSArray<AnyObject>>> {
        self.with(|m, _, n| {
            if n.calendar.is_none() {
                return Ok(None);
            }
            let nodes = m.host.snapshot()?;
            let selected = nodes
                .iter()
                .filter(|p| {
                    p.calendar_day.is_some_and(|d| d.selected)
                        && super::table_accessibility::within(&nodes, p.id, n.id)
                })
                .map(|p| any(element(m, p.id)))
                .collect::<Vec<_>>();
            Ok(Some(NSArray::from_retained_slice(&selected)))
        })
        .flatten()
    }
    fn combo_selected(&self) -> Option<Retained<NSArray<AnyObject>>> {
        self.with(|m, _, n| {
            if n.combo.is_none() && n.combo_popup.is_none() {
                return Ok(None);
            }
            let nodes = m.host.snapshot()?;
            let list = if n.collection.is_some() {
                Some(n)
            } else {
                nodes
                    .iter()
                    .find(|p| p.collection.is_some() && p.combo_owner.as_ref() == n.key.as_ref())
            };
            let result = list
                .and_then(|n| {
                    n.collection
                        .as_ref()
                        .and_then(|model| model.cursor().and_then(|key| model.index_of(&key)))
                        .map(|index| {
                            super::collection_accessibility::rows(
                                self.ivars().owner.clone(),
                                n,
                                index,
                                1,
                            )
                        })
                })
                .unwrap_or_else(NSArray::new);
            Ok(Some(result))
        })
        .flatten()
    }
    fn tab_elements(&self, part: u8) -> Option<Retained<NSArray<AnyObject>>> {
        self.with(|m, _, n| {
            if n.kind == Kind::Tab && (part != 2 || !n.value.selected()) {
                return Ok(None);
            }
            let nodes = m.host.snapshot()?;
            let root = if n.kind == Kind::Tabs {
                Some(n.id)
            } else if n.kind == Kind::Tab {
                tab_group(n, &nodes)
            } else {
                None
            };
            let Some(root) = root else { return Ok(None) };
            let elements: Vec<_> = nodes
                .iter()
                .filter(|p| {
                    !p.hidden
                        && if part == 2 {
                            p.kind == Kind::TabPanel && p.parent == Some(root)
                        } else {
                            p.kind == Kind::Tab
                                && tab_group(p, &nodes) == Some(root)
                                && (part == 1 || p.value.selected())
                        }
                })
                .map(|n| any(element(m, n.id)))
                .collect();
            Ok(Some(NSArray::from_retained_slice(&elements)))
        })
        .flatten()
    }

    pub(super) fn with<T>(
        &self,
        f: impl FnOnce(&Mounted, &Surface, &Node) -> std::result::Result<T, ui::Error>,
    ) -> Option<T> {
        let m = self.ivars().owner.upgrade()?;
        m.run(|m, s| {
            let nodes = m.host.snapshot()?;
            let node = nodes
                .iter()
                .find(|n| {
                    n.id == self.ivars().id
                        && semantic(n)
                        && super::control_accessibility::readable(m, n, &nodes)
                })
                .cloned();
            node.map(|n| f(m, s, &n)).transpose()
        })
        .flatten()
    }
    fn alert_button(&self, role: ui::AlertActionRole) -> Option<Retained<AnyObject>> {
        self.with(|m, _, root| {
            if !root.alert {
                return Ok(None);
            }
            let nodes = m.host.snapshot()?;
            Ok(nodes
                .iter()
                .find(|n| {
                    if n.alert_role != Some(role) || !n.enabled || n.hidden {
                        return false;
                    }
                    let mut ancestor = n.parent;
                    while let Some(id) = ancestor {
                        let Some(p) = nodes.iter().find(|p| p.id == id) else {
                            break;
                        };
                        if p.kind == Kind::Modal {
                            return p.id == root.id;
                        }
                        ancestor = p.parent;
                    }
                    false
                })
                .map(|n| any(element(m, n.id))))
        })
        .flatten()
    }
    fn node(&self) -> Option<Node> {
        self.with(|_, _, n| Ok(n.clone()))
    }
    fn split_contents(&self, first: bool) -> Option<Retained<NSArray<AnyObject>>> {
        self.with(|m, _, n| {
            if n.kind != Kind::SplitDivider {
                return Ok(None);
            }
            let nodes = m.host.snapshot()?;
            let pane = nodes
                .iter()
                .find(|p| {
                    p.parent == n.parent
                        && p.key.as_deref() == Some(if first { "$first" } else { "$second" })
                })
                .map(|p| p.id);
            let items: Vec<_> = nodes
                .iter()
                .filter(|child| {
                    if !semantic(child) || parent(child, &nodes) != n.parent || child.id == n.id {
                        return false;
                    }
                    let mut ancestor = child.parent;
                    while let Some(id) = ancestor {
                        if Some(id) == pane {
                            return true;
                        }
                        ancestor = nodes.iter().find(|n| n.id == id).and_then(|n| n.parent);
                    }
                    false
                })
                .map(|n| any(element(m, n.id)))
                .collect();
            Ok(Some(NSArray::from_retained_slice(&items)))
        })
        .flatten()
    }
    fn adjust(&self, delta: f64) -> bool {
        self.with(|m, _, n| {
            if !super::control_accessibility::can_adjust(n, delta > 0.) {
                return Ok(false);
            }
            if n.kind == Kind::Stepper {
                return m.host.step_control(n.id, delta > 0.);
            }
            if n.kind == Kind::SplitDivider {
                return m
                    .host
                    .resize_split(n.id, delta * n.range.map_or(10., |r| r.2));
            }

            let (Some((min, max, step)), Value::Number(value)) = (n.range, &n.value) else {
                return Ok(false);
            };
            m.host.set_number(
                n.id,
                *value + delta * if step > 0. { step } else { (max - min) / 100. },
            )
        })
        .unwrap_or(false)
    }
}
fn byte_range(text: &str, r: NSRange) -> Option<std::ops::Range<usize>> {
    Some(
        ui::text::byte_offset(text, r.location)?
            ..ui::text::byte_offset(text, r.location.checked_add(r.length)?)?,
    )
}
fn native_range(text: &str, r: std::ops::Range<usize>) -> Option<NSRange> {
    let start = ui::text::utf16_offset(text, r.start)?;
    Some(NSRange::new(
        start,
        ui::text::utf16_offset(text, r.end)? - start,
    ))
}
