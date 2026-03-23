use core::ops::{Deref, DerefMut};

/// Cache-line aligned wrapper to prevent false sharing in concurrent environments.
/// Assumes a typical cache line size of 64 bytes.
#[repr(align(64))]
#[derive(Debug, Default, Clone, Copy)]
pub struct CachePadded<T> {
    pub value: T,
}

impl<T> CachePadded<T> {
    /// Creates a new cache-padded value.
    pub const fn new(value: T) -> Self {
        Self { value }
    }

    /// Unwraps the value, discarding the padding.
    pub fn into_inner(self) -> T {
        self.value
    }
}

impl<T> Deref for CachePadded<T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &T {
        &self.value
    }
}

impl<T> DerefMut for CachePadded<T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut T {
        &mut self.value
    }
}
