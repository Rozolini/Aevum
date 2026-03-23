use alloc::boxed::Box;
use alloc::vec::Vec;
use core::cell::UnsafeCell;
use core::mem::MaybeUninit;
#[cfg(not(loom))]
use core::sync::atomic::{AtomicUsize, Ordering};

#[cfg(loom)]
use loom::sync::atomic::{AtomicUsize, Ordering};

use crate::sync::CachePadded;

/// Wait-free Single-Producer Single-Consumer (SPSC) queue.
pub struct SpscQueue<T> {
    buffer: Box<[UnsafeCell<MaybeUninit<T>>]>,
    mask: usize,
    // Cache-line alignment prevents false sharing between producer and consumer.
    head: CachePadded<AtomicUsize>,
    tail: CachePadded<AtomicUsize>,
}

unsafe impl<T: Send> Sync for SpscQueue<T> {}
unsafe impl<T: Send> Send for SpscQueue<T> {}

impl<T> SpscQueue<T> {
    /// Creates a new SPSC queue. `capacity` must be a power of two.
    pub fn new(capacity: usize) -> Self {
        assert!(
            capacity.is_power_of_two(),
            "capacity must be a power of two"
        );

        let mut buffer = Vec::with_capacity(capacity);
        for _ in 0..capacity {
            buffer.push(UnsafeCell::new(MaybeUninit::uninit()));
        }

        Self {
            buffer: buffer.into_boxed_slice(),
            mask: capacity - 1,
            head: CachePadded::new(AtomicUsize::new(0)),
            tail: CachePadded::new(AtomicUsize::new(0)),
        }
    }

    /// Enqueues an element. Must be called by a single producer thread.
    pub fn push(&self, value: T) -> Result<(), T> {
        let tail = self.tail.load(Ordering::Relaxed);

        // Acquire head to observe consumer progress.
        let head = self.head.load(Ordering::Acquire);

        // Queue is full.
        if tail.wrapping_sub(head) >= self.buffer.len() {
            return Err(value);
        }

        unsafe {
            (*self.buffer[tail & self.mask].get()).write(value);
        }

        // Release tail to ensure the data write is visible before the index update.
        self.tail.store(tail.wrapping_add(1), Ordering::Release);
        Ok(())
    }

    /// Dequeues an element. Must be called by a single consumer thread.
    pub fn pop(&self) -> Option<T> {
        let head = self.head.load(Ordering::Relaxed);

        // Acquire tail to observe producer progress.
        let tail = self.tail.load(Ordering::Acquire);

        // Queue is empty.
        if head == tail {
            return None;
        }

        let value = unsafe { (*self.buffer[head & self.mask].get()).assume_init_read() };

        // Release head to ensure the data read completes before freeing the slot.
        self.head.store(head.wrapping_add(1), Ordering::Release);
        Some(value)
    }
}

impl<T> Drop for SpscQueue<T> {
    fn drop(&mut self) {
        while self.pop().is_some() {}
    }
}
