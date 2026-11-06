use super::*;
use objc2::Message;
use ui::{Kind, Node, NodeId, Value};
fn any<T: Message>(value: Retained<T>) -> Retained<AnyObject> {
    // SAFETY: Upcast a live Objective-C object to id.
    unsafe { Retained::cast_unchecked(value) }
}
fn semantic(n: &Node) -> bool {
    !n.hidden
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
fn tab_group(n: &Node, nodes: &[Node]) -> Option<NodeId> {
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
fn parent(n: &Node, nodes: &[Node]) -> Option<NodeId> {
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
                && m.host.in_scope(n.id)
                && parent(n, &nodes).filter(|id| m.host.in_scope(*id)) == parent_id
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
    m.run(|m, _| Ok(m.host.focused().map(|id| any(element(m, id)))))
        .flatten()
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
        Ok(m.host
            .scene()?
            .iter()
            .rev()
            .find(|n| semantic(n) && n.clip.contains(p) && n.bounds.contains(p))
            .map(|n| any(element(m, n.id))))
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
            Kind::RadioGroup => NSAccessibilityRadioGroupRole,
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
    id: NodeId,
}
define_class!(
    // SAFETY: NSAccessibilityElement is subclassable; selectors match AppKit's protocol.
    #[unsafe(super=NSAccessibilityElement)]
    #[thread_kind=MainThreadOnly]
    #[ivars=ElementIvars]
    pub(super) struct Element;
    unsafe impl NSObjectProtocol for Element {}
    unsafe impl NSAccessibility for Element {
        #[unsafe(method(isAccessibilityElement))]
        fn is_element(&self) -> bool {
            self.node().is_some()
        }
        #[unsafe(method_id(accessibilityRole))]
        fn role(&self) -> Option<Retained<NSString>> {
            self.node().map(|n| {
                if n.menu_item.is_some() { unsafe { NSAccessibilityMenuItemRole.retain() } }
                else if n.menu_panel { unsafe { NSAccessibilityMenuRole.retain() } }
                else if n.collection.is_some() {
                    unsafe { NSAccessibilityListRole.retain() }
                } else {
                    ns(&role(n.kind).to_string())
                }
            })
        }
        #[unsafe(method_id(accessibilityLabel))]
        fn label(&self) -> Option<Retained<NSString>> {
            self.node().map(|n| ns(&n.label))
        }
        #[unsafe(method_id(accessibilityHelp))]
        fn help(&self) -> Option<Retained<NSString>> {
            self.node().and_then(|n| n.menu_item.filter(|m| !m.shortcut.is_empty()).map(|m| ns(&m.shortcut)).or_else(||(!n.help.is_empty()).then(||ns(&n.help))))
        }
        #[unsafe(method(isAccessibilityExpanded))]
        fn expanded(&self) -> bool {
            self.node().and_then(|n| n.menu_item).is_some_and(|m| m.expanded)
        }
        #[unsafe(method_id(accessibilityIdentifier))]
        fn identifier(&self) -> Option<Retained<NSString>> {
            self.node()
                .map(|n| ns(&n.key.unwrap_or_else(|| n.id.to_string())))
        }
        #[unsafe(method(accessibilityFrame))]
        fn frame(&self) -> NSRect {
            self.with(|_, s, n| Ok(screen(s, n.clip)))
                .unwrap_or(NSRect::ZERO)
        }
        #[unsafe(method_id(accessibilityParent))]
        fn parent(&self) -> Option<Retained<AnyObject>> {
            self.with(|m, s, n| {
                let nodes = m.host.snapshot()?;
                Ok(parent(n, &nodes)
                    .map(|id| any(element(m, id)))
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
            self.with(|m, _, n| {
                if let Some(model) = &n.collection {
                    let range = model.visible_range();
                    Ok(super::collection_accessibility::rows(
                        self.ivars().owner.clone(),
                        n,
                        range.start,
                        range.len(),
                    ))
                } else {
                    list(m, Some(n.id))
                }
            })
        }
        #[unsafe(method_id(accessibilityTabs))]
        fn tabs(&self)->Option<Retained<NSArray<AnyObject>>>{self.tab_elements(1)}
        #[unsafe(method_id(accessibilitySelectedChildren))]
        fn selected_children(&self)->Option<Retained<NSArray<AnyObject>>>{self.tab_elements(0)}
        #[unsafe(method_id(accessibilityContents))]
        fn contents(&self)->Option<Retained<NSArray<AnyObject>>>{self.tab_elements(2)}
        #[unsafe(method(accessibilityRowCount))]
        fn row_count(&self) -> isize {
            self.node()
                .and_then(|n| n.collection)
                .map_or(0, |m| m.len() as isize)
        }
        #[unsafe(method_id(accessibilitySubrole))]
        fn subrole(&self)->Option<Retained<NSString>>{
            self.node().and_then(|n|unsafe{if n.kind==Kind::Tab {Some(NSAccessibilityTabButtonSubrole.retain())}else{match n.field_kind{
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
            self.node().is_some_and(|n| n.focused)
        }
        #[unsafe(method(setAccessibilityFocused:))]
        fn focus(&self, value: bool) {
            if value {
                self.with(|m, s, n| {
                    s.focus();
                    m.host.focus(n.id)
                });
            }
        }
        #[unsafe(method_id(accessibilityValue))]
        fn value(&self)->Option<Retained<AnyObject>>{
            self.node().filter(|n|!n.secure).and_then(|n|{
                if n.kind==Kind::Tabs{return self.tab_elements(0).and_then(|tabs|tabs.into_iter().next())}
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
            self.with(|m, _, n| {
                if let Some(text) = value.and_then(|v| v.downcast_ref::<NSString>()) {
                    m.host.set_text(n.id, &text.to_string())
                } else if let Some(number) = value.and_then(|v| v.downcast_ref::<NSNumber>()) {
                    if matches!(n.kind,Kind::Slider|Kind::SplitDivider) {
                        m.host.set_number(n.id, number.doubleValue())
                    } else if n.value.selected() != number.boolValue() {
                        m.host.activate(n.id)
                    } else {
                        Ok(false)
                    }
                } else {
                    Ok(false)
                }
            });
        }
        #[unsafe(method(accessibilityOrientation))]
        fn orientation(&self)->NSAccessibilityOrientation{
            if self.node().is_some_and(|n|n.kind==Kind::Tabs){return NSAccessibilityOrientation::Horizontal;}
            self.node().and_then(|n|n.split.map(|s|if (s.orientation==ui::Orientation::Vertical)!=(n.kind==Kind::SplitDivider){NSAccessibilityOrientation::Vertical}else{NSAccessibilityOrientation::Horizontal})).unwrap_or(NSAccessibilityOrientation::Unknown)
        }
        #[unsafe(method_id(accessibilityValueDescription))]
        fn value_description(&self)->Option<Retained<NSString>>{
            self.node().filter(|n|n.kind==Kind::SplitDivider).and_then(|n|n.split).map(|s|ns(&match s.collapsed{ui::SplitPane::First=>"First pane collapsed".into(),ui::SplitPane::Second=>"Second pane collapsed".into(),ui::SplitPane::None=>format!("{:.0} points",if s.orientation==ui::Orientation::Horizontal{s.first.width}else{s.first.height})}))
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
        fn modal(&self) -> bool { self.node().is_some_and(|n| n.alert) }
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
            self.with(|m, _, n| m.host.activate(n.id)).unwrap_or(false)
        }
        #[unsafe(method(accessibilityPerformIncrement))]
        fn increment(&self) -> bool {
            self.adjust(1.)
        }
        #[unsafe(method(accessibilityPerformDecrement))]
        fn decrement(&self) -> bool {
            self.adjust(-1.)
        }
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
                    b"accessibilityPerformPress" => {
                        n.enabled
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
                            )
                    }
                    b"setAccessibilityValue:" => {
                        n.enabled
                            && !n.read_only
                            && matches!(
                                n.kind,
                                Kind::TextField
                                    | Kind::Slider
                                    | Kind::Toggle
                                    | Kind::CheckBox
                                    | Kind::Radio
                                    | Kind::Segment
                                    | Kind::Tab
                                    | Kind::Switch
                            )
                    }
                    b"accessibilityPerformIncrement" | b"accessibilityPerformDecrement" => {
                        n.enabled && !n.read_only && n.kind == Kind::Slider
                    }
                    _ => true,
                })
        }
    }
    impl Element {
        #[unsafe(method(accessibilityArrayAttributeCount:))]
        fn array_count(&self, attribute: &NSString) -> usize {
            if matches!(attribute.to_string().as_str(), "AXChildren" | "AXRows") {
                self.node().and_then(|n|n.collection).map_or_else(||self.accessibilityChildren().map_or(0,|v|v.len()),|m|m.len())
            } else {
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
            if matches!(attribute.to_string().as_str(), "AXChildren" | "AXRows") {
                self.row_array(index, count)
            } else {
                None
            }
        }

    }
);
impl Element {
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
            let node = m
                .host
                .snapshot()?
                .into_iter()
                .find(|n| n.id == self.ivars().id && semantic(n) && m.host.in_scope(n.id));
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
