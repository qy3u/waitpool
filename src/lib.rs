mod notifier;
mod spin_lock;

use std::collections::VecDeque;
use std::fmt::{self, Debug};

use std::ops::{Deref, DerefMut};

use notifier::{Event, Notifier, Waiter};
use spin_lock::SpinLock;

pub struct Pool<T> {
    inner: SpinLock<PoolInner<T>>,
}

impl<T> Pool<T> {
    pub fn new() -> Pool<T> {
        Self {
            inner: SpinLock::new(PoolInner::default()),
        }
    }

    pub fn with_capacity(n: usize) -> Pool<T> {
        Self {
            inner: SpinLock::new(PoolInner::with_capacity(n)),
        }
    }

    pub fn get(&self) -> Pooled<T> {
        loop {
            if let Some(p) = self.try_get() {
                return p;
            }
        }
    }

    pub fn try_get(&self) -> Option<Pooled<T>> {
        let mut inner = self.inner.lock();
        if inner.count == 0 {
            panic!("get before pooled any element");
        }

        inner.pooled.pop_front().map(|elem| Pooled {
            pool: self as *const Pool<T> as *mut Pool<T>,
            elem: Some(elem),
        })
    }

    pub fn pool(&self, elem: T) {
        let mut inner = self.inner.lock();
        inner.count += 1;
        inner.pooled.push_back(elem)
    }

    fn back(&mut self, elem: T) {
        let mut inner = self.inner.lock();
        if inner.destory_start {
            inner.count -= 1;
            inner.notifier.notify_one(Event::OneElemDisconnected);

            drop(inner);
            drop(elem);
            return;
        }

        inner.pooled.push_back(elem)
    }
}

impl<T> Drop for Pool<T> {
    fn drop(&mut self) {
        loop {
            let mut inner = self.inner.lock();
            inner.destory_start = true;

            let n = inner.pooled.len();
            inner.pooled.clear();
            inner.count -= n;

            if inner.count == 0 {
                break;
            }

            let notifier = inner.notifier.clone();
            drop(inner);
            Waiter::wait(&notifier, Event::OneElemDisconnected)
        }
    }
}

unsafe impl<T> Sync for Pool<T> {}
unsafe impl<T> Send for Pool<T> {}

struct PoolInner<T> {
    pooled: VecDeque<T>,
    destory_start: bool,
    count: usize,
    notifier: Notifier,
}

impl<T> PoolInner<T> {
    pub fn with_capacity(n: usize) -> Self {
        Self {
            pooled: VecDeque::with_capacity(n),
            destory_start: false,
            count: 0,
            notifier: Notifier::default(),
        }
    }
}

impl<T> Default for PoolInner<T> {
    fn default() -> Self {
        Self::with_capacity(0)
    }
}

pub struct Pooled<T> {
    pool: *mut Pool<T>,
    elem: Option<T>,
}

impl<T> Pooled<T> {
    pub fn into_inner(self) -> T {
        let mut pooled = self;
        pooled.disconnect();
        pooled.elem.take().unwrap()
    }

    fn disconnect(&mut self) {
        let mut inner = self.source().inner.lock();
        inner.count -= 1;

        if inner.destory_start {
            let notifier = inner.notifier.clone();
            notifier.notify_one(Event::OneElemDisconnected);
        }

        drop(inner);
        self.pool = 0 as *const Pool<T> as *mut Pool<T>;
    }

    fn source(&self) -> &mut Pool<T> {
        assert_ne!(self.pool as usize, 0);
        unsafe { &mut *self.pool }
    }
}

impl<T: Debug> Debug for Pooled<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Pooled").field("elem", &self.elem).finish()
    }
}

impl<T> Deref for Pooled<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.elem.as_ref().unwrap()
    }
}

impl<T> DerefMut for Pooled<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.elem.as_mut().unwrap()
    }
}

impl<T> Drop for Pooled<T> {
    fn drop(&mut self) {
        if self.pool as usize != 0 {
            let elem = self.elem.take().unwrap();
            self.source().back(elem);
        }
    }
}

unsafe impl<T: Send> Send for Pooled<T> {}
unsafe impl<T: Sync> Sync for Pooled<T> {}
