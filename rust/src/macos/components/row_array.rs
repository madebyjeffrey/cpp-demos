//! Immutable NSArray primitives backed by shared Rust data, allocating proxies on demand.
use super::{
    collection_accessibility::{Row, row},
    *,
};
use objc2::Message;
use std::{collections::HashMap, ops::Range};
pub(super) enum Source {
    Range(ui::ListSnapshot, Range<usize>),
    Children(ui::TreeChildrenSnapshot, Range<usize>),
    Indices(ui::ListSnapshot, Vec<usize>),
}
impl Source {
    fn len(&self) -> usize {
        match self {
            Self::Range(_, r) | Self::Children(_, r) => r.len(),
            Self::Indices(_, v) => v.len(),
        }
    }
    fn identity(&self, index: usize) -> Option<(String, u64)> {
        if index >= self.len() {
            return None;
        }
        match self {
            Self::Range(s, r) => {
                let key = &s.item(r.start + index)?.key;
                Some((key.clone(), s.item_token(key)?))
            }
            Self::Children(s, r) => {
                Some((s.key(r.start + index)?.into(), s.token(r.start + index)?))
            }
            Self::Indices(s, v) => {
                let key = &s.item(v[index])?.key;
                Some((key.clone(), s.item_token(key)?))
            }
        }
    }
}
pub(super) fn array(
    owner: Weak<Mounted>,
    parent: ui::NodeId,
    source: Source,
) -> Retained<NSArray<AnyObject>> {
    let allocated = RowsArray::alloc(MainThreadMarker::new().unwrap()).set_ivars(RowsIvars {
        owner,
        parent,
        source,
        cache: RefCell::new(HashMap::new()),
    });
    // SAFETY: NSArray documents init as a designated initializer for primitive subclasses.
    let value: Retained<RowsArray> = unsafe { msg_send![super(allocated), init] };
    unsafe { Retained::cast_unchecked(value) }
}
struct RowsIvars {
    owner: Weak<Mounted>,
    parent: ui::NodeId,
    source: Source,
    cache: RefCell<HashMap<usize, Retained<Row>>>,
}
define_class!(
    // SAFETY: An immutable NSArray subclass implements its documented count/objectAtIndex primitives.
    #[unsafe(super=NSArray<AnyObject>)]
    #[thread_kind=MainThreadOnly]
    #[ivars=RowsIvars]
    struct RowsArray;
    unsafe impl NSObjectProtocol for RowsArray {}
    impl RowsArray {
        #[unsafe(method(scarsRealizedRowCount))]
        fn realized_count(&self)->usize{self.ivars().cache.borrow().len()}
        #[unsafe(method(count))]
        fn count(&self)->usize{self.ivars().source.len()}
        #[unsafe(method_id(objectAtIndex:))]
        fn object(&self,index:usize)->Retained<AnyObject>{self.object_at(index)}
        #[unsafe(method_id(copyWithZone:))]
        unsafe fn copy_array(&self,_zone:*mut NSZone)->Retained<AnyObject>{unsafe{Retained::cast_unchecked(self.retain())}}
    }
);

impl RowsArray {
    fn object_at(&self, index: usize) -> Retained<AnyObject> {
        if let Some(value) = self.ivars().cache.borrow().get(&index) {
            return unsafe { Retained::cast_unchecked(value.clone()) };
        }
        let Some((key, token)) = self.ivars().source.identity(index) else {
            return NSArray::<AnyObject>::new().objectAtIndex(index);
        };
        let value = row(self.ivars().owner.clone(), self.ivars().parent, key, token);
        self.ivars().cache.borrow_mut().insert(index, value.clone());
        unsafe { Retained::cast_unchecked(value) }
    }
}
