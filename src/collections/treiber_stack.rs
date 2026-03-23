use core::ptr;
use core::sync::atomic::Ordering;
use crossbeam_epoch::{self as epoch, Atomic, Owned};

struct Node<T> {
    data: T,
    next: Atomic<Node<T>>,
}

/// Lock-free Treiber stack with Epoch-Based Reclamation (EBR).
pub struct TreiberStack<T> {
    head: Atomic<Node<T>>,
}

unsafe impl<T: Send> Send for TreiberStack<T> {}
unsafe impl<T: Send> Sync for TreiberStack<T> {}

impl<T> TreiberStack<T> {
    /// Creates a new empty stack.
    pub fn new() -> Self {
        Self {
            head: Atomic::null(),
        }
    }

    /// Pushes an element onto the top of the stack.
    pub fn push(&self, data: T) {
        let mut node = Owned::new(Node {
            data,
            next: Atomic::null(),
        });
        let guard = &epoch::pin();

        loop {
            let head = self.head.load(Ordering::Relaxed, guard);
            node.next.store(head, Ordering::Relaxed);

            match self.head.compare_exchange(
                head,
                node,
                Ordering::Release,
                Ordering::Relaxed,
                guard,
            ) {
                Ok(_) => break,
                Err(e) => node = e.new,
            }
        }
    }

    /// Pops an element from the top of the stack.
    pub fn pop(&self) -> Option<T> {
        let guard = &epoch::pin();

        loop {
            let head = self.head.load(Ordering::Acquire, guard);
            if head.is_null() {
                return None;
            }

            let next = unsafe { head.deref() }.next.load(Ordering::Relaxed, guard);

            if self
                .head
                .compare_exchange(head, next, Ordering::Acquire, Ordering::Relaxed, guard)
                .is_ok()
            {
                unsafe {
                    // Read data before scheduling the node for deferred destruction.
                    let data = ptr::read(&(*head.as_raw()).data);
                    guard.defer_destroy(head);
                    return Some(data);
                }
            }
        }
    }
}

impl<T> Default for TreiberStack<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Drop for TreiberStack<T> {
    fn drop(&mut self) {
        while self.pop().is_some() {}
    }
}
