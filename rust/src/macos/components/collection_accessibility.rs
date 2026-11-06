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
    let rows: Vec<_> = (start.min(end)..end)
        .filter_map(|index| {
            let item = model.item(index)?;
            let allocated = Row::alloc(MainThreadMarker::new().unwrap()).set_ivars(RowIvars {
                owner: owner.clone(),
                parent: node.id,
                token: model.item_token(&item.key).unwrap(),
                key: item.key,
            });
            let row: Retained<Row> = unsafe { msg_send![super(allocated), init] };
            Some(any(row))
        })
        .collect();
    NSArray::from_retained_slice(&rows)
}
pub(super) struct RowIvars {
    owner: Weak<Mounted>,
    parent: NodeId,
    key: String,
    token: u64,
}
define_class!(
    // SAFETY: NSAccessibilityElement is subclassable; all callbacks run on the main thread.
    #[unsafe(super=NSAccessibilityElement)]
    #[thread_kind=MainThreadOnly]
    #[ivars=RowIvars]
    struct Row;
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
        #[unsafe(method_id(accessibilityIdentifier))]
        fn identifier(&self) -> Option<Retained<NSString>> {
            self.with(|_, _, node, _, item| {
                ns(&format!("{}/{}/{}", node.id, item.key, self.ivars().token))
            })
        }
        #[unsafe(method_id(accessibilityLabel))]
        fn label(&self) -> Option<Retained<NSString>> {
            self.with(|_, _, _, _, item| ns(&item.label()))
        }
        #[unsafe(method(accessibilityIndex))]
        fn index(&self) -> isize {
            self.with(|_, _, _, model, item| model.index_of(&item.key).unwrap() as isize)
                .unwrap_or(-1)
        }
        #[unsafe(method(isAccessibilityEnabled))]
        fn enabled(&self) -> bool {
            self.with(|_, _, _, _, item| item.enabled).unwrap_or(false)
        }
        #[unsafe(method(isAccessibilitySelected))]
        fn selected(&self) -> bool {
            self.with(|_, _, _, model, item| model.is_selected(&item.key))
                .unwrap_or(false)
        }
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
                self.accessibilityPerformPress();
            }
        }
    }
);
impl Row {
    fn with<T>(
        &self,
        action: impl FnOnce(&Mounted, &Surface, &Node, &ListModel, &ListItem) -> T,
    ) -> Option<T> {
        let m = self.ivars().owner.upgrade()?;
        m.run(|m, s| {
            let node = m.host.snapshot()?.into_iter().find(|n| {
                n.id == self.ivars().parent && n.enabled && !n.hidden && m.host.in_scope(n.id)
            });
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
