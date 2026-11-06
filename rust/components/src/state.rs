use std::{
    cell::{Cell, RefCell},
    rc::{Rc, Weak},
};

type Wake = Rc<dyn Fn()>;

pub(crate) struct Observer {
    pub dirty: Cell<bool>,
    pub epoch: Cell<u64>,
    wake: RefCell<Option<Wake>>,
}
impl Observer {
    pub fn new() -> Rc<Self> {
        Rc::new(Self {
            dirty: Cell::new(true),
            epoch: Cell::new(0),
            wake: RefCell::new(None),
        })
    }
    pub fn set_wake(&self, wake: Wake) {
        let old = self.wake.replace(Some(wake));
        drop(old);
    }
    pub fn redraw(&self) {
        let wake = self.wake.borrow().clone();
        if let Some(wake) = wake {
            wake();
        }
    }
    fn invalidate(&self) -> Option<Wake> {
        if !self.dirty.replace(true) {
            self.wake.borrow().clone()
        } else {
            None
        }
    }
}

pub(crate) trait Dependency {
    fn subscribe(&self, observer: &Rc<Observer>, epoch: u64);
}

thread_local! {
    static TRACKING: RefCell<Vec<Vec<Rc<dyn Dependency>>>> = const { RefCell::new(Vec::new()) };
}

/// A stack guard restores dependency tracking even when a builder unwinds.
pub(crate) struct Tracking;
impl Tracking {
    pub fn begin() -> Self {
        TRACKING.with(|t| t.borrow_mut().push(Vec::new()));
        Self
    }
    pub fn finish(self) -> Vec<Rc<dyn Dependency>> {
        let result = TRACKING.with(|t| t.borrow_mut().pop().unwrap());
        std::mem::forget(self);
        result
    }
}
impl Drop for Tracking {
    fn drop(&mut self) {
        TRACKING.with(|t| {
            t.borrow_mut().pop();
        });
    }
}
pub(crate) fn check_write() {
    assert!(
        TRACKING.with(|t| t.borrow().is_empty()),
        "State cannot change during view construction or a read-only query"
    );
}

/// Run pure user queries while preserving dependency tracking in outer builders.
pub(crate) fn read_only<T>(query: impl FnOnce() -> T) -> T {
    let tracking = Tracking::begin();
    let result = query();
    let deps = tracking.finish();
    TRACKING.with(|stack| {
        if let Some(parent) = stack.borrow_mut().last_mut() {
            for dep in deps {
                if !parent.iter().any(|p| Rc::ptr_eq(p, &dep)) {
                    parent.push(dep);
                }
            }
        }
    });
    result
}

struct CellState<T> {
    value: RefCell<T>,
    observers: RefCell<Vec<(Weak<Observer>, u64)>>,
}
impl<T> Dependency for CellState<T> {
    fn subscribe(&self, observer: &Rc<Observer>, epoch: u64) {
        let mut observers = self.observers.borrow_mut();
        observers.retain(|(weak, _)| weak.strong_count() > 0);
        if let Some(item) = observers
            .iter_mut()
            .find(|(weak, _)| weak.ptr_eq(&Rc::downgrade(observer)))
        {
            item.1 = epoch;
        } else {
            observers.push((Rc::downgrade(observer), epoch));
        }
    }
}

/// Cloneable reactive value. Cloning shares the value, not its contents.
pub struct State<T>(Rc<CellState<T>>);
impl<T> Clone for State<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}
impl<T: 'static> State<T> {
    pub fn new(value: T) -> Self {
        Self(Rc::new(CellState {
            value: RefCell::new(value),
            observers: RefCell::new(Vec::new()),
        }))
    }
    pub fn get(&self) -> T
    where
        T: Clone,
    {
        self.with(Clone::clone)
    }
    pub(crate) fn with<R>(&self, read: impl FnOnce(&T) -> R) -> R {
        TRACKING.with(|tracking| {
            if let Some(deps) = tracking.borrow_mut().last_mut() {
                let cell: Rc<dyn Dependency> = self.0.clone();
                if !deps.iter().any(|dep| Rc::ptr_eq(dep, &cell)) {
                    deps.push(cell);
                }
            }
        });
        read(&self.0.value.borrow())
    }
    /// Publish a changed value. Equal writes do not invalidate hosts.
    pub fn set(&self, value: T)
    where
        T: PartialEq,
    {
        check_write();
        {
            let mut previous = self.0.value.borrow_mut();
            if *previous == value {
                return;
            }
            *previous = value;
        }
        let observers = self.0.observers.borrow().clone();
        let mut wakes = Vec::new();
        for (weak, epoch) in observers {
            if let Some(observer) = weak.upgrade()
                && observer.epoch.get() == epoch
                && let Some(wake) = observer.invalidate()
            {
                wakes.push(wake);
            }
        }
        // Mark every dependent host dirty before invoking any external scheduler.
        for wake in wakes {
            wake();
        }
    }
    /// Transform a snapshot, then publish it. No state borrow spans the closure.
    pub fn update(&self, change: impl FnOnce(&mut T))
    where
        T: Clone + PartialEq,
    {
        check_write();
        let mut value = self.get();
        change(&mut value);
        self.set(value);
    }
    pub fn binding(&self) -> Binding<T>
    where
        T: Clone + PartialEq,
    {
        let read = self.clone();
        let write = self.clone();
        Binding::new(move || read.get(), move |value| write.set(value))
    }
    /// Capture a state handle in a callback that receives an event value.
    pub fn handler<A: 'static>(&self, action: impl Fn(&Self, A) + 'static) -> impl Fn(A) + 'static {
        let state = self.clone();
        move |value| action(&state, value)
    }
    /// Capture an owned state handle in an event action.
    pub fn action(&self, action: impl Fn(&Self) + 'static) -> impl Fn() + 'static {
        let state = self.clone();
        move || action(&state)
    }
}

/// Owned two-way access. A custom setter may reject or normalize an input;
/// `get` always reads the accepted value.
pub struct Binding<T> {
    read: Rc<dyn Fn() -> T>,
    write: Rc<dyn Fn(T)>,
}
impl<T> Clone for Binding<T> {
    fn clone(&self) -> Self {
        Self {
            read: self.read.clone(),
            write: self.write.clone(),
        }
    }
}
impl<T> Binding<T> {
    pub fn binding(&self) -> Self {
        self.clone()
    }
    pub fn new(read: impl Fn() -> T + 'static, write: impl Fn(T) + 'static) -> Self {
        Self {
            read: Rc::new(read),
            write: Rc::new(write),
        }
    }
    pub fn get(&self) -> T {
        (self.read)()
    }
    pub fn set(&self, value: T) {
        check_write();
        (self.write)(value);
    }
}
