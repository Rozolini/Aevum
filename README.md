# Aevum

Aevum is a lock-free concurrency framework with epoch-based reclamation written in Rust. It provides foundational data structures and synchronization primitives designed for low-latency, high-throughput concurrent systems.

---

## Architecture

The library exposes several isolated synchronization primitives and collections:

### 1. Queues
*   **`ArrayQueue`**: A bounded, allocation-free, lock-free MPMC (Multi-Producer Multi-Consumer) queue. Uses atomic sequence locks to manage enqueue/dequeue operations.
  

* **`SpscQueue`**: A bounded, wait-free SPSC (Single-Producer Single-Consumer) queue.
  
    * Utilizes strict cache-line padding (`#[repr(align(64))]`) for `head` and `tail` pointers to eliminate false sharing across CPU cores.

### 2. Maps & Stacks
*   **`FlatLockFreeMap`**: A bounded lock-free hash map.
    *   Employs open addressing and linear probing to maximize cache locality and avoid pointer chasing.
  

* **`TreiberStack`**: A lock-free stack.
    *   Mitigates the ABA problem and handles safe memory deallocation using Epoch-Based Reclamation (EBR) via the `crossbeam-epoch` backend.

### 3. Synchronization & Execution
*   **`TicketLock`**: A fair, FIFO-ordered spinlock.
    *   Provides deterministic lock acquisition order under high contention.
  

* **`LockFreeThreadPool`**: A minimalist thread pool implementation built on top of `ArrayQueue` for task scheduling without mutex contention.
  

* **`ObjectPool`**: A concurrent pool for object reuse, minimizing global allocator pressure during peak loads.

---

## Getting Started

### Prerequisites
*   Rust toolchain (stable)
  

* Rust nightly (optional, strictly for Miri verification)

### Installation
Add the repository to your `Cargo.toml`:

```toml
[dependencies]
aevum = { git = "https://github.com/Rozolini/aevum" }
```
## Verification & Benchmarks

Due to the complexity of lock-free memory orderings, the library relies on strict automated verification rather than standard unit tests.

### 1. Exhaustive Concurrency Testing (Loom)
Simulates all possible thread interleavings and memory barrier executions to mathematically prove the absence of data races and deadlocks.

```powershell
$env:RUSTFLAGS="--cfg loom"; cargo test --test loom_tests --release
```
### 2. Undefined Behavior Detection (Miri)
Validates memory safety and strict pointer rules. Isolation is disabled to support high-concurrency primitives.

```powershell
$env:MIRIFLAGS="-Zmiri-disable-isolation -Zmiri-tree-borrows -Zmiri-ignore-leaks"; cargo +nightly miri test
```

### 3. Throughput Benchmarking (Criterion)
Measures instruction footprint and latency per operation under high concurrency.

```powershell
cargo bench
```
## Design Considerations

* **Memory Ordering:** Uses `Relaxed` ordering wherever possible for optimal CPU cache utilization, escalating to `Acquire/Release` strictly for establishing happen-before relationships. `SeqCst` is avoided to prevent full memory barrier stalls.


* **Zero-Cost Abstractions:** The structures are designed to be allocation-free post-initialization. All memory bounds must be provided upfront.


* **Unsafe Code Isolation:** `unsafe` blocks are strictly confined to raw pointer dereferencing and atomic memory initialization.

## License

This project is licensed under the MIT License. See the [LICENSE](LICENSE) file for details.