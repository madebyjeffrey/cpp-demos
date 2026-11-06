//! Shared table data with lazy objectAtIndex, including full logical row/column arrays.
use super::{
    table_accessibility::{Part, entry},
    *,
};
use objc2::Message;
use std::{collections::HashMap, ops::Range};
pub(super) enum Source {
    Children(Range<usize>),
    Rows(Range<usize>),
    Columns(Range<usize>),
    Headers(Range<usize>),
    Cells(Range<usize>, Vec<usize>),
    Explicit(Vec<Part>),
}
impl Source {
    fn len(&self) -> usize {
        match self {
            Self::Children(r) | Self::Rows(r) | Self::Columns(r) | Self::Headers(r) => r.len(),
            Self::Cells(r, c) => r.len().saturating_mul(c.len()),
            Self::Explicit(p) => p.len(),
        }
    }
    fn part(&self, snapshot: &ui::TableSnapshot, index: usize) -> Option<Part> {
        if index >= self.len() {
            return None;
        }
        let row = |i| {
            let item = snapshot.rows().item(i)?;
            Some(Part::Row(
                item.key.clone(),
                snapshot.rows().item_token(&item.key)?,
            ))
        };
        let col = |i, header| {
            let c = snapshot.column(i)?;
            Some(if header {
                Part::Header(c.id.clone(), c.token)
            } else {
                Part::Column(c.id.clone(), c.token)
            })
        };
        match self {
            Self::Children(r) => {
                let i = r.start + index;
                let n = snapshot.rows().len();
                if i < n {
                    row(i)
                } else if i < n + snapshot.column_count() {
                    col(i - n, false)
                } else {
                    Some(Part::Headers)
                }
            }
            Self::Rows(r) => row(r.start + index),
            Self::Columns(r) => col(r.start + index, false),
            Self::Headers(r) => col(r.start + index, true),
            Self::Cells(r, c) => {
                let row = r.start + index / c.len();
                let column = c[index % c.len()];
                let cell = snapshot.cell(row, column)?;
                let rt = snapshot.rows().item_token(&cell.row)?;
                Some(Part::Cell(cell, rt, snapshot.column(column)?.token))
            }
            Self::Explicit(p) => p.get(index).cloned(),
        }
    }
}
pub(super) fn array(
    owner: Weak<Mounted>,
    root: ui::NodeId,
    snapshot: ui::TableSnapshot,
    source: Source,
) -> Retained<NSArray<AnyObject>> {
    let allocated = TableArray::alloc(MainThreadMarker::new().unwrap()).set_ivars(ArrayIvars {
        owner,
        root,
        snapshot,
        source,
        cache: RefCell::new(HashMap::new()),
    });
    // SAFETY: The documented NSArray primitive-subclass designated initializer.
    let value: Retained<TableArray> = unsafe { msg_send![super(allocated), init] };
    unsafe { Retained::cast_unchecked(value) }
}
struct ArrayIvars {
    owner: Weak<Mounted>,
    root: ui::NodeId,
    snapshot: ui::TableSnapshot,
    source: Source,
    cache: RefCell<HashMap<usize, Retained<AnyObject>>>,
}
define_class!(
    // SAFETY: Immutable NSArray subclass implementing count and objectAtIndex.
    #[unsafe(super=NSArray<AnyObject>)] #[thread_kind=MainThreadOnly] #[ivars=ArrayIvars]
    struct TableArray;
    unsafe impl NSObjectProtocol for TableArray {}
    impl TableArray {
        #[unsafe(method(count))] fn count(&self)->usize{self.ivars().source.len()}
        #[unsafe(method(scarsRealizedEntryCount))] fn realized(&self)->usize{self.ivars().cache.borrow().len()}
        #[unsafe(method_id(objectAtIndex:))] fn object(&self,index:usize)->Retained<AnyObject>{self.object_at(index)}
        #[unsafe(method_id(copyWithZone:))] unsafe fn copy(&self,_zone:*mut NSZone)->Retained<AnyObject>{unsafe{Retained::cast_unchecked(self.retain())}}
    }
);
impl TableArray {
    fn object_at(&self, index: usize) -> Retained<AnyObject> {
        if let Some(v) = self.ivars().cache.borrow().get(&index) {
            return v.clone();
        }
        let Some(part) = self.ivars().source.part(&self.ivars().snapshot, index) else {
            return NSArray::<AnyObject>::new().objectAtIndex(index);
        };
        let value = entry(self.ivars().owner.clone(), self.ivars().root, part);
        self.ivars().cache.borrow_mut().insert(index, value.clone());
        value
    }
}
