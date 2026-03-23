#[cfg(not(loom))]
use core::hint::spin_loop;
#[cfg(not(loom))]
use core::sync::atomic::{AtomicUsize, Ordering};

#[cfg(loom)]
use loom::hint::spin_loop;
#[cfg(loom)]
use loom::sync::atomic::{AtomicUsize, Ordering};

use alloc::boxed::Box;
use alloc::vec::Vec;
use core::cell::UnsafeCell;
use core::mem::MaybeUninit;

/// Cache-line aligned wrapper to prevent false sharing.
#[repr(align(64))]
struct CachePadded<T> {
    value: T,
}

struct ArrayNode<T> {
    sequence: AtomicUsize,
    data: UnsafeCell<MaybeUninit<T>>,
}

/// Bounded, allocation-free, lock-free MPMC queue.
pub struct ArrayQueue<T> {
    buffer: Box<[ArrayNode<T>]>,
    mask: usize,
    head: CachePadded<AtomicUsize>,
    tail: CachePadded<AtomicUsize>,
}

unsafe impl<T: Send> Sync for ArrayQueue<T> {}
unsafe impl<T: Send> Send for ArrayQueue<T> {}

impl<T> ArrayQueue<T> {
    /// Creates a new queue. `capacity` must be a power of two.
    pub fn new(capacity: usize) -> Self {
        assert!(
            capacity.is_power_of_two(),
            "capacity must be a power of two"
        );

        let mut buffer = Vec::with_capacity(capacity);
        for i in 0..capacity {
            buffer.push(ArrayNode {
                sequence: AtomicUsize::new(i),
                data: UnsafeCell::new(MaybeUninit::uninit()),
            });
        }

        Self {
            buffer: buffer.into_boxed_slice(),
            mask: capacity - 1,
            head: CachePadded {
                value: AtomicUsize::new(0),
            },
            tail: CachePadded {
                value: AtomicUsize::new(0),
            },
        }
    }

    /// Enqueues an element. Returns `Err(data)` if the queue is full.
    pub fn push(&self, data: T) -> Result<(), T> {
        let mut pos = self.tail.value.load(Ordering::Relaxed);

        loop {
            let node = &self.buffer[pos & self.mask];
            let seq = node.sequence.load(Ordering::Acquire);
            let dif = seq as isize - pos as isize;

            if dif == 0 {
                match self.tail.value.compare_exchange_weak(
                    pos,
                    pos + 1,
                    Ordering::Relaxed,
                    Ordering::Relaxed,
                ) {
                    Ok(_) => {
                        unsafe {
                            (*node.data.get()).write(data);
                        }
                        node.sequence.store(pos + 1, Ordering::Release);
                        return Ok(());
                    }
                    Err(new_pos) => {
                        pos = new_pos;
                        spin_loop();
                    }
                }
            } else if dif < 0 {
                return Err(data);
            } else {
                pos = self.tail.value.load(Ordering::Relaxed);
                spin_loop();
            }
        }
    }

    /// Dequeues an element. Returns `None` if the queue is empty.
    pub fn pop(&self) -> Option<T> {
        let mut pos = self.head.value.load(Ordering::Relaxed);

        loop {
            let node = &self.buffer[pos & self.mask];
            let seq = node.sequence.load(Ordering::Acquire);
            let dif = seq as isize - (pos + 1) as isize;

            if dif == 0 {
                match self.head.value.compare_exchange_weak(
                    pos,
                    pos + 1,
                    Ordering::Relaxed,
                    Ordering::Relaxed,
                ) {
                    Ok(_) => {
                        let data = unsafe { (*node.data.get()).assume_init_read() };
                        node.sequence.store(pos + self.mask + 1, Ordering::Release);
                        return Some(data);
                    }
                    Err(new_pos) => {
                        pos = new_pos;
                        spin_loop();
                    }
                }
            } else if dif < 0 {
                return None;
            } else {
                pos = self.head.value.load(Ordering::Relaxed);
                spin_loop();
            }
        }
    }
}

impl<T> Drop for ArrayQueue<T> {
    fn drop(&mut self) {
        // Drain remaining elements.
        while self.pop().is_some() {}
    }
}
