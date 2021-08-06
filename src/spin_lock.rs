use std::cell::UnsafeCell;
use std::ops::{Deref, DerefMut};
use std::sync::atomic::{AtomicBool, Ordering};

pub struct SpinLock<T> {
    flag: AtomicBool,
    value: UnsafeCell<T>,
}

impl<T> SpinLock<T> {
    pub fn new(value: T) -> SpinLock<T> {
        Self {
            flag: AtomicBool::new(false),
            value: UnsafeCell::new(value),
        }
    }

    pub fn lock(&self) -> SpinLockGuard<'_, T> {
        loop {
            if self
                .flag
                .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
                .is_ok()
            {
                return SpinLockGuard { lock: &self };
            }

            for _ in 0..32 {
                std::hint::spin_loop()
            }
        }
    }
}

pub struct SpinLockGuard<'guard, T> {
    lock: &'guard SpinLock<T>,
}

impl<T> Drop for SpinLockGuard<'_, T> {
    fn drop(&mut self) {
        self.lock.flag.store(false, Ordering::SeqCst);
    }
}

impl<T> Deref for SpinLockGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.lock.value.get() }
    }
}

impl<T> DerefMut for SpinLockGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.lock.value.get() }
    }
}
