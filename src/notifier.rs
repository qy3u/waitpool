use crate::spin_lock::SpinLock;

use std::cmp::{Eq, PartialEq};
use std::sync::Arc;
use std::thread::{self, Thread};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Event {
    OneElemDisconnected,
}

pub struct Waiter {
    thread: Thread,
}

impl PartialEq for Waiter {
    fn eq(&self, other: &Self) -> bool {
        self.thread.id() == other.thread.id()
    }
}

impl Waiter {
    pub fn wait(notifier: &Notifier, event: Event) {
        let thread = thread::current();
        let waiter = Waiter { thread };
        notifier.regist(waiter, event);
        thread::park()
    }

    pub fn awake(&self) {
        self.thread.unpark();
    }
}

impl Eq for Waiter {}

#[derive(Clone)]
pub struct Notifier {
    waiters: Arc<SpinLock<Vec<(Waiter, Event)>>>,
}

impl Default for Notifier {
    fn default() -> Self {
        Self {
            waiters: Arc::new(SpinLock::new(Vec::new())),
        }
    }
}

impl Notifier {
    pub fn regist(&self, waiter: Waiter, event: Event) {
        self.waiters.lock().push((waiter, event));
    }

    pub fn notify_one(&self, event: Event) -> Option<Waiter> {
        let mut waiters = self.waiters.lock();
        waiters.iter().position(|(_, e)| e == &event).map(|i| {
            waiters[i].0.awake();
            waiters.remove(i).0
        })
    }
}
