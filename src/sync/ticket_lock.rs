#[cfg(not(loom))]
use core::hint::spin_loop;
#[cfg(not(loom))]
use core::sync::atomic::{AtomicUsize, Ordering};

#[cfg(loom)]
use loom::hint::spin_loop;
#[cfg(loom)]
use loom::sync::atomic::{AtomicUsize, Ordering};

use crate::sync::CachePadded;

/// Fair, FIFO-ordered spinlock based on the ticket lock algorithm.
/// Note: This is a raw synchronization primitive without data wrapping.
pub struct TicketLock {
    next_ticket: CachePadded<AtomicUsize>,
    now_serving: CachePadded<AtomicUsize>,
}

/// Guard that releases the TicketLock when dropped.
pub struct TicketLockGuard<'a> {
    lock: &'a TicketLock,
}

impl TicketLock {
    /// Creates a new unlocked TicketLock.
    #[cfg(not(loom))]
    pub const fn new() -> Self {
        Self {
            next_ticket: CachePadded::new(AtomicUsize::new(0)),
            now_serving: CachePadded::new(AtomicUsize::new(0)),
        }
    }

    /// Creates a new unlocked TicketLock (Loom compatible).
    #[cfg(loom)]
    pub fn new() -> Self {
        Self {
            next_ticket: CachePadded::new(AtomicUsize::new(0)),
            now_serving: CachePadded::new(AtomicUsize::new(0)),
        }
    }

    /// Acquires the lock, spinning until the assigned ticket becomes active.
    pub fn lock(&self) -> TicketLockGuard<'_> {
        let my_ticket = self.next_ticket.fetch_add(1, Ordering::Relaxed);

        while self.now_serving.load(Ordering::Acquire) != my_ticket {
            spin_loop();
        }

        TicketLockGuard { lock: self }
    }

    /// Attempts to acquire the lock without spinning.
    pub fn try_lock(&self) -> Option<TicketLockGuard<'_>> {
        let next = self.next_ticket.load(Ordering::Relaxed);
        let serving = self.now_serving.load(Ordering::Acquire);

        if serving == next
            && self
                .next_ticket
                .compare_exchange(
                    next,
                    next.wrapping_add(1),
                    Ordering::Relaxed,
                    Ordering::Relaxed,
                )
                .is_ok()
        {
            return Some(TicketLockGuard { lock: self });
        }

        None
    }

    /// Releases the lock to the next ticket.
    fn unlock(&self) {
        let current = self.now_serving.load(Ordering::Relaxed);
        self.now_serving
            .store(current.wrapping_add(1), Ordering::Release);
    }
}

impl Default for TicketLock {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for TicketLockGuard<'_> {
    fn drop(&mut self) {
        self.lock.unlock();
    }
}
