use crate::collections::queue::ArrayQueue;
use core::ops::{Deref, DerefMut};

/// Lock-free object pool for reusing expensive resources.
pub struct ObjectPool<T> {
    queue: ArrayQueue<T>,
    factory: fn() -> T,
}

impl<T> ObjectPool<T> {
    /// Creates a new pool. `capacity` must be a power of two.
    /// `factory` initializes a new object when the pool is empty.
    pub fn new(capacity: usize, factory: fn() -> T) -> Self {
        Self {
            queue: ArrayQueue::new(capacity),
            factory,
        }
    }

    /// Acquires an object from the pool. Creates a new one via `factory` if empty.
    pub fn take(&self) -> PooledObject<'_, T> {
        let data = self.queue.pop().unwrap_or_else(|| (self.factory)());
        PooledObject {
            pool: self,
            data: Some(data),
        }
    }
}

/// Smart pointer that automatically returns the object to the pool on drop.
pub struct PooledObject<'a, T> {
    pool: &'a ObjectPool<T>,
    data: Option<T>,
}

impl<T> Deref for PooledObject<'_, T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.data.as_ref().unwrap()
    }
}

impl<T> DerefMut for PooledObject<'_, T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.data.as_mut().unwrap()
    }
}

impl<T> Drop for PooledObject<'_, T> {
    fn drop(&mut self) {
        if let Some(data) = self.data.take() {
            // Silently drop the object if the pool is full.
            let _ = self.pool.queue.push(data);
        }
    }
}
