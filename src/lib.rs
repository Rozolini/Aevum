//! Aevum: Lock-free concurrency framework.
//!
//! Provides thread-safe data structures and synchronization primitives
//! designed for low-latency concurrent environments. Memory management
//! is backed by Epoch-Based Reclamation (EBR).

extern crate alloc;

pub mod collections;
pub mod sync;

// Data structures
pub use crate::collections::flat_map::FlatLockFreeMap;
pub use crate::collections::object_pool::ObjectPool;
pub use crate::collections::queue::ArrayQueue;
pub use crate::collections::spsc::SpscQueue;
pub use crate::collections::treiber_stack::TreiberStack;

// Synchronization primitives
pub use crate::sync::cache_pad::CachePadded;
pub use crate::sync::thread_pool::LockFreeThreadPool;
pub use crate::sync::ticket_lock::{TicketLock, TicketLockGuard};

pub mod prelude {
    //! Prelude for commonly used Aevum types.
    pub use super::{
        ArrayQueue, CachePadded, FlatLockFreeMap, LockFreeThreadPool, ObjectPool, SpscQueue,
        TicketLock, TreiberStack,
    };
}
