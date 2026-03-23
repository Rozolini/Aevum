use aevum::collections::queue::ArrayQueue;
use aevum::collections::spsc::SpscQueue;
use aevum::collections::treiber_stack::TreiberStack;
use criterion::{criterion_group, criterion_main, Criterion};
use std::collections::VecDeque;
use std::hint::black_box;
use std::sync::Arc;
use std::sync::Mutex;
use std::thread;

// Simulates workload to induce realistic thread contention.
fn simulate_work(n: u32) {
    for i in 0..n {
        black_box(i);
    }
}

// Benchmark for Multi-Producer Multi-Consumer (MPMC) scenario.
fn bench_heavy_load(c: &mut Criterion) {
    let mut group = c.benchmark_group("MPMC_Heavy_Load_16_Threads");
    let num_producers = 8;
    let num_consumers = 8;
    let items_per_thread = 5_000;
    let work_iterations = 100;

    // 1. Lock-free ArrayQueue (MPMC).
    group.bench_function("ArrayQueue", |b| {
        b.iter(|| {
            let queue = Arc::new(ArrayQueue::new(65536));
            let mut handles = vec![];

            for _ in 0..num_consumers {
                let q = Arc::clone(&queue);
                handles.push(thread::spawn(move || {
                    for _ in 0..items_per_thread {
                        while q.pop().is_none() {
                            core::hint::spin_loop();
                        }
                        simulate_work(work_iterations);
                    }
                }));
            }

            for _ in 0..num_producers {
                let q = Arc::clone(&queue);
                handles.push(thread::spawn(move || {
                    for i in 0..items_per_thread {
                        simulate_work(work_iterations);
                        while q.push(i).is_err() {
                            core::hint::spin_loop();
                        }
                    }
                }));
            }
            for t in handles {
                t.join().unwrap();
            }
        });
    });

    // 2. Lock-free TreiberStack (MPMC).
    group.bench_function("TreiberStack", |b| {
        b.iter(|| {
            let stack = Arc::new(TreiberStack::new());
            let mut handles = vec![];

            for _ in 0..num_consumers {
                let s = Arc::clone(&stack);
                handles.push(thread::spawn(move || {
                    for _ in 0..items_per_thread {
                        while s.pop().is_none() {
                            core::hint::spin_loop();
                        }
                        simulate_work(work_iterations);
                    }
                }));
            }

            for _ in 0..num_producers {
                let s = Arc::clone(&stack);
                handles.push(thread::spawn(move || {
                    for i in 0..items_per_thread {
                        simulate_work(work_iterations);
                        s.push(i);
                    }
                }));
            }
            for t in handles {
                t.join().unwrap();
            }
        });
    });

    // 3. Baseline: Mutex-protected VecDeque.
    group.bench_function("StdMutexQueue", |b| {
        b.iter(|| {
            let queue = Arc::new(Mutex::new(VecDeque::new()));
            let mut handles = vec![];

            for _ in 0..num_consumers {
                let q = Arc::clone(&queue);
                handles.push(thread::spawn(move || {
                    for _ in 0..items_per_thread {
                        loop {
                            let mut lock = q.lock().unwrap();
                            if lock.pop_front().is_some() {
                                break;
                            }
                            drop(lock);
                            core::hint::spin_loop();
                        }
                        simulate_work(work_iterations);
                    }
                }));
            }

            for _ in 0..num_producers {
                let q = Arc::clone(&queue);
                handles.push(thread::spawn(move || {
                    for i in 0..items_per_thread {
                        simulate_work(work_iterations);
                        let mut lock = q.lock().unwrap();
                        lock.push_back(i);
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

// Benchmark for Single-Producer Single-Consumer (SPSC) scenario.
fn bench_spsc_load(c: &mut Criterion) {
    let mut group = c.benchmark_group("SPSC_1_Producer_1_Consumer");
    let items_per_thread = 50_000;
    let work_iterations = 100;

    // 1. Wait-free SpscQueue (Specialized).
    group.bench_function("SpscQueue", |b| {
        b.iter(|| {
            let queue = Arc::new(SpscQueue::new(65536));

            let q_cons = Arc::clone(&queue);
            let consumer = thread::spawn(move || {
                for _ in 0..items_per_thread {
                    while q_cons.pop().is_none() {
                        core::hint::spin_loop();
                    }
                    simulate_work(work_iterations);
                }
            });

            let q_prod = Arc::clone(&queue);
            let producer = thread::spawn(move || {
                for i in 0..items_per_thread {
                    simulate_work(work_iterations);
                    while q_prod.push(i).is_err() {
                        core::hint::spin_loop();
                    }
                }
            });

            producer.join().unwrap();
            consumer.join().unwrap();
        });
    });

    // 2. Baseline: Mutex-protected VecDeque (SPSC context).
    group.bench_function("StdMutexQueue", |b| {
        b.iter(|| {
            let queue = Arc::new(Mutex::new(VecDeque::new()));

            let q_cons = Arc::clone(&queue);
            let consumer = thread::spawn(move || {
                for _ in 0..items_per_thread {
                    loop {
                        let mut lock = q_cons.lock().unwrap();
                        if lock.pop_front().is_some() {
                            break;
                        }
                        drop(lock);
                        core::hint::spin_loop();
                    }
                    simulate_work(work_iterations);
                }
            });

            let q_prod = Arc::clone(&queue);
            let producer = thread::spawn(move || {
                for i in 0..items_per_thread {
                    simulate_work(work_iterations);
                    let mut lock = q_prod.lock().unwrap();
                    lock.push_back(i);
                }
            });

            producer.join().unwrap();
            consumer.join().unwrap();
        });
    });

    group.finish();
}

criterion_group!(benches, bench_heavy_load, bench_spsc_load);
criterion_main!(benches);
