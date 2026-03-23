use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;

use aevum::collections::{ArrayQueue, FlatLockFreeMap, ObjectPool, SpscQueue, TreiberStack};
use aevum::sync::LockFreeThreadPool;

#[test]
fn test_array_queue_mpmc() {
    let iters = if cfg!(miri) { 10 } else { 1000 };
    let queue = Arc::new(ArrayQueue::new(4096));
    let counter = Arc::new(AtomicUsize::new(0));
    let mut handles = vec![];

    // 4 Producers.
    for _ in 0..4 {
        let q = Arc::clone(&queue);
        handles.push(thread::spawn(move || {
            for _ in 0..iters {
                while q.push(1).is_err() {
                    core::hint::spin_loop();
                }
            }
        }));
    }

    // 4 Consumers.
    for _ in 0..4 {
        let q = Arc::clone(&queue);
        let c = Arc::clone(&counter);
        handles.push(thread::spawn(move || {
            for _ in 0..iters {
                loop {
                    if let Some(val) = q.pop() {
                        c.fetch_add(val, Ordering::Relaxed);
                        break;
                    }
                    core::hint::spin_loop();
                }
            }
        }));
    }

    for h in handles {
        h.join().unwrap();
    }

    assert_eq!(counter.load(Ordering::Relaxed), 4 * iters);
}

#[test]
fn test_thread_pool_execution() {
    let iters = if cfg!(miri) { 10 } else { 1000 };
    let pool = LockFreeThreadPool::new(4, 1024);
    let counter = Arc::new(AtomicUsize::new(0));

    for _ in 0..iters {
        loop {
            let c = Arc::clone(&counter);
            if pool
                .execute(move || {
                    c.fetch_add(1, Ordering::Relaxed);
                })
                .is_ok()
            {
                break;
            }
            core::hint::spin_loop();
        }
    }

    // Dropping the pool blocks until all workers drain the queue and exit.
    drop(pool);

    assert_eq!(counter.load(Ordering::Relaxed), iters);
}

#[test]
fn test_spsc_queue() {
    let iters = if cfg!(miri) { 10 } else { 1000 };
    let queue = Arc::new(SpscQueue::new(1024));
    let q_prod = Arc::clone(&queue);
    let q_cons = Arc::clone(&queue);

    let producer = thread::spawn(move || {
        for i in 0..iters {
            while q_prod.push(i).is_err() {
                core::hint::spin_loop();
            }
        }
    });

    let consumer = thread::spawn(move || {
        for i in 0..iters {
            loop {
                if let Some(val) = q_cons.pop() {
                    assert_eq!(val, i);
                    break;
                }
                core::hint::spin_loop();
            }
        }
    });

    producer.join().unwrap();
    consumer.join().unwrap();
}

#[test]
fn test_treiber_stack_concurrent() {
    let iters = if cfg!(miri) { 10 } else { 1000 };
    let stack = Arc::new(TreiberStack::new());
    let mut handles = vec![];

    // 4 threads pushing items.
    for _ in 0..4 {
        let s = Arc::clone(&stack);
        handles.push(thread::spawn(move || {
            for i in 0..iters {
                s.push(i);
            }
        }));
    }

    for h in handles {
        h.join().unwrap();
    }

    let mut total_popped = 0;
    while stack.pop().is_some() {
        total_popped += 1;
    }

    assert_eq!(total_popped, 4 * iters);
}

#[test]
fn test_flat_map_concurrent() {
    let iters = if cfg!(miri) { 10 } else { 1000 };
    let map = Arc::new(FlatLockFreeMap::new(4096));
    let mut handles = vec![];

    // 4 threads inserting unique key ranges.
    for t in 0..4 {
        let m = Arc::clone(&map);
        handles.push(thread::spawn(move || {
            for i in 1..=iters {
                let key = t * 10000 + i;
                assert!(m.insert(key, key * 2));
            }
        }));
    }

    for h in handles {
        h.join().unwrap();
    }

    // Verify all inserts.
    for t in 0..4 {
        for i in 1..=iters {
            let key = t * 10000 + i;
            assert_eq!(map.get(key), Some(key * 2));
        }
    }
}

#[test]
fn test_object_pool_concurrent() {
    let iters = if cfg!(miri) { 10 } else { 1000 };
    let pool = Arc::new(ObjectPool::new(1024, || 0));
    let mut handles = vec![];

    // 4 threads concurrently taking and dropping objects.
    for _ in 0..4 {
        let p = Arc::clone(&pool);
        handles.push(thread::spawn(move || {
            for _ in 0..iters {
                let mut obj = p.take();
                *obj += 1;
                // The object is automatically returned to the pool here via Drop.
            }
        }));
    }

    for h in handles {
        h.join().unwrap();
    }
}