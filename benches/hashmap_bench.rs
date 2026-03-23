use aevum::collections::flat_map::FlatLockFreeMap;
use criterion::{criterion_group, criterion_main, Criterion};
use std::collections::HashMap;
use std::hint::black_box;
use std::sync::{Arc, RwLock};
use std::thread;

/// Benchmarks `RwLock<HashMap>` against `FlatLockFreeMap` under high contention.
/// Simulates 16 concurrent threads with a 50/50 read/write workload.
fn bench_hashmap_high_contention(c: &mut Criterion) {
    let mut group = c.benchmark_group("HashMap_16_Threads_50_50");
    let num_threads = 16;
    let ops_per_thread = 1000;

    // Baseline: Standard library HashMap guarded by RwLock.
    group.bench_function("RwLock_HashMap", |b| {
        let map = Arc::new(RwLock::new(HashMap::with_capacity(16384)));

        b.iter(|| {
            let mut threads = vec![];
            for _ in 1..=num_threads {
                let m = Arc::clone(&map);
                threads.push(thread::spawn(move || {
                    for i in 1..=ops_per_thread {
                        if i % 2 == 0 {
                            let mut write_guard = m.write().unwrap();
                            write_guard.insert(i, i);
                        } else {
                            let read_guard = m.read().unwrap();
                            black_box(read_guard.get(&i));
                        }
                    }
                }));
            }
            for t in threads {
                t.join().unwrap();
            }
        });
    });

    // Lock-free implementation: FlatLockFreeMap.
    group.bench_function("FlatLockFreeMap", |b| {
        let map = Arc::new(FlatLockFreeMap::new(16384));

        b.iter(|| {
            let mut threads = vec![];
            for _ in 1..=num_threads {
                let m = Arc::clone(&map);
                threads.push(thread::spawn(move || {
                    for i in 1..=ops_per_thread {
                        if i % 2 == 0 {
                            m.insert(i, i);
                        } else {
                            black_box(m.get(i));
                        }
                    }
                }));
            }
            for t in threads {
                t.join().unwrap();
            }
        });
    });

    group.finish();
}

criterion_group!(benches, bench_hashmap_high_contention);
criterion_main!(benches);
