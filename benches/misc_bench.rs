use criterion::{criterion_group, criterion_main, Criterion};
use std::hint::black_box;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;

use aevum::collections::object_pool::ObjectPool;
use aevum::sync::thread_pool::LockFreeThreadPool;
use aevum::sync::ticket_lock::TicketLock;

fn bench_object_pool(c: &mut Criterion) {
    let mut group = c.benchmark_group("ObjectPool_Allocation");
    let iterations = 10_000;
    let pool_capacity = 16384; // Has to be a power of two for ArrayQueue

    group.bench_function("Standard_Box", |b| {
        b.iter(|| {
            for i in 0..iterations {
                let obj = black_box(Box::new(i));
                black_box(obj);
            }
        });
    });

    group.bench_function("Aevum_ObjectPool", |b| {
        let pool = Arc::new(ObjectPool::new(pool_capacity, || 0));
        b.iter(|| {
            for i in 0..iterations {
                let mut obj = pool.take();
                *obj = i;
                black_box(obj);
            }
        });
    });

    group.finish();
}

fn bench_ticket_lock(c: &mut Criterion) {
    let mut group = c.benchmark_group("TicketLock_Contention_8_Threads");
    let num_threads = 8;
    let ops_per_thread = 1_000;

    group.bench_function("Std_Mutex", |b| {
        b.iter(|| {
            let lock = Arc::new(Mutex::new(()));
            let mut handles = vec![];
            for _ in 0..num_threads {
                let l = Arc::clone(&lock);
                handles.push(thread::spawn(move || {
                    for _ in 0..ops_per_thread {
                        let _guard = l.lock().unwrap();
                        black_box(1);
                    }
                }));
            }
            for t in handles {
                t.join().unwrap();
            }
        });
    });

    group.bench_function("Aevum_TicketLock", |b| {
        b.iter(|| {
            let lock = Arc::new(TicketLock::new());
            let mut handles = vec![];
            for _ in 0..num_threads {
                let l = Arc::clone(&lock);
                handles.push(thread::spawn(move || {
                    for _ in 0..ops_per_thread {
                        let _guard = l.lock();
                        black_box(1);
                    }
                }));
            }
            for t in handles {
                t.join().unwrap();
            }
        });
    });

    group.finish();
}

fn bench_thread_pool(c: &mut Criterion) {
    let mut group = c.benchmark_group("ThreadPool_Task_Throughput");
    let tasks = 1_000;

    group.bench_function("Std_Thread_Spawn", |b| {
        b.iter(|| {
            let counter = Arc::new(AtomicUsize::new(0));
            let mut handles = vec![];
            for _ in 0..tasks {
                let c = Arc::clone(&counter);
                handles.push(thread::spawn(move || {
                    c.fetch_add(1, Ordering::Relaxed);
                }));
            }
            for t in handles {
                t.join().unwrap();
            }
        });
    });

    group.bench_function("Aevum_LockFreeThreadPool", |b| {
        // 4 threads, 2048 capacity (power of two)
        let pool = LockFreeThreadPool::new(4, 2048);

        b.iter(|| {
            let counter = Arc::new(AtomicUsize::new(0));
            for _ in 0..tasks {
                let c = Arc::clone(&counter);
                let _ = pool.execute(move || {
                    c.fetch_add(1, Ordering::Relaxed);
                });
            }
            while counter.load(Ordering::Acquire) < tasks {
                core::hint::spin_loop();
            }
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_object_pool,
    bench_ticket_lock,
    bench_thread_pool
);
criterion_main!(benches);
