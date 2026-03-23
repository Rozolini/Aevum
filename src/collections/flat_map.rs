#[cfg(not(loom))]
use core::sync::atomic::{AtomicUsize, Ordering};

#[cfg(loom)]
use loom::sync::atomic::{AtomicUsize, Ordering};

use alloc::vec::Vec;

const EMPTY: usize = 0;
const TOMBSTONE: usize = usize::MAX;

/// Bounded lock-free hash map using open addressing and linear probing.
/// Keys and values are stored adjacently in a flat array to maximize cache locality.
pub struct FlatLockFreeMap {
    table: Vec<AtomicUsize>,
    capacity: usize,
}

impl FlatLockFreeMap {
    /// Creates a new map with a fixed capacity.
    pub fn new(capacity: usize) -> Self {
        let mut table = Vec::with_capacity(capacity * 2);
        for _ in 0..(capacity * 2) {
            table.push(AtomicUsize::new(EMPTY));
        }

        Self { table, capacity }
    }

    /// FNV-1a hash function optimized for usize.
    fn hash_key(key: usize) -> usize {
        let mut hash = 0xcbf29ce484222325u64;
        for byte in key.to_ne_bytes() {
            hash ^= byte as u64;
            hash = hash.wrapping_mul(0x100000001b3);
        }
        hash as usize
    }

    /// Inserts a key-value pair. Returns false if the map is full or the key is invalid.
    ///
    /// Note: A concurrent reader might temporarily observe a default value (0)
    /// if `get` is called exactly between the key CAS and the value store.
    pub fn insert(&self, key: usize, value: usize) -> bool {
        if key == EMPTY || key == TOMBSTONE {
            return false;
        }

        let mut index = (Self::hash_key(key) % self.capacity) * 2;
        let start_index = index;

        loop {
            let current_key = self.table[index].load(Ordering::Acquire);

            if current_key == EMPTY || current_key == TOMBSTONE {
                if self.table[index]
                    .compare_exchange(current_key, key, Ordering::Release, Ordering::Relaxed)
                    .is_ok()
                {
                    self.table[index + 1].store(value, Ordering::Release);
                    return true;
                }
            } else if current_key == key {
                self.table[index + 1].store(value, Ordering::Release);
                return true;
            }

            index = (index + 2) % (self.capacity * 2);

            if index == start_index {
                return false; // Map is full
            }
        }
    }

    /// Retrieves the value associated with the key. Returns None if not found.
    pub fn get(&self, key: usize) -> Option<usize> {
        if key == EMPTY || key == TOMBSTONE {
            return None;
        }

        let mut index = (Self::hash_key(key) % self.capacity) * 2;
        let start_index = index;

        loop {
            let current_key = self.table[index].load(Ordering::Acquire);

            if current_key == key {
                return Some(self.table[index + 1].load(Ordering::Acquire));
            } else if current_key == EMPTY {
                return None;
            }

            index = (index + 2) % (self.capacity * 2);

            if index == start_index {
                return None;
            }
        }
    }

    /// Removes a key-value pair by replacing the key with a tombstone marker.
    pub fn remove(&self, key: usize) -> bool {
        if key == EMPTY || key == TOMBSTONE {
            return false;
        }

        let mut index = (Self::hash_key(key) % self.capacity) * 2;
        let start_index = index;

        loop {
            let current_key = self.table[index].load(Ordering::Acquire);

            if current_key == key {
                if self.table[index]
                    .compare_exchange(key, TOMBSTONE, Ordering::Release, Ordering::Relaxed)
                    .is_ok()
                {
                    return true;
                }
            } else if current_key == EMPTY {
                return false;
            }

            index = (index + 2) % (self.capacity * 2);

            if index == start_index {
                return false;
            }
        }
    }
}
