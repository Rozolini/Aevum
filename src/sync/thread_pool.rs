use alloc::boxed::Box;
use alloc::vec::Vec;

#[cfg(not(loom))]
use core::sync::atomic::{AtomicBool, Ordering};

#[cfg(loom)]
use loom::sync::atomic::{AtomicBool, Ordering};

use std::sync::Arc;
use std::thread::{self, JoinHandle};

use crate::collections::queue::ArrayQueue;

/// Executable job submitted to the thread pool.
type Job = Box<dyn FnOnce() + Send + 'static>;

/// Lock-free thread pool for task execution.
pub struct LockFreeThreadPool {
    workers: Vec<JoinHandle<()>>,
    queue: Arc<ArrayQueue<Job>>,
    active: Arc<AtomicBool>,
}

impl LockFreeThreadPool {
    /// Initializes the thread pool.
    /// `queue_capacity` must be a power of two.
    pub fn new(threads: usize, queue_capacity: usize) -> Self {
        let queue = Arc::new(ArrayQueue::<Job>::new(queue_capacity));
        let active = Arc::new(AtomicBool::new(true));
        let mut workers = Vec::with_capacity(threads);

        for _ in 0..threads {
            let q = Arc::clone(&queue);
            let a = Arc::clone(&active);

            workers.push(thread::spawn(move || {
                while a.load(Ordering::Relaxed) {
                    if let Some(job) = q.pop() {
                        job();
                    } else {
                        core::hint::spin_loop();
                    }
                }

                // Drain remaining jobs after shutdown signal.
                while let Some(job) = q.pop() {
                    job();
                }
            }));
        }

        Self {
            workers,
            queue,
            active,
        }
    }

    /// Submits a job to the pool.
    /// Returns the job as an error if the underlying queue is full.
    pub fn execute<F>(&self, f: F) -> Result<(), Job>
    where
        F: FnOnce() + Send + 'static,
    {
        self.queue.push(Box::new(f))
    }
}

impl Drop for LockFreeThreadPool {
    fn drop(&mut self) {
        self.active.store(false, Ordering::Release);
        for worker in self.workers.drain(..) {
            let _ = worker.join();
        }
    }
}
