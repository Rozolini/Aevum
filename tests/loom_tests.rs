#![cfg(loom)]

use loom::sync::Arc;
use loom::thread;

use aevum::collections::{ArrayQueue, SpscQueue};
use aevum::sync::TicketLock;

#[test]
fn test_array_queue_loom() {
    loom::model(|| {
        // Use minimal capacity to force edge cases (wrap-around) quickly.
        let queue = Arc::new(ArrayQueue::<usize>::new(2));
        let q1 = Arc::clone(&queue);
        let q2 = Arc::clone(&queue);

        let t1 = thread::spawn(move || {
            let _ = q1.push(1);
            q1.pop()
        });

        let t2 = thread::spawn(move || {
            let _ = q2.push(2);
            q2.pop()
        });

        t1.join().unwrap();
        t2.join().unwrap();
    });
}

#[test]
fn test_spsc_queue_loom() {
    loom::model(|| {
        let queue = Arc::new(SpscQueue::<usize>::new(2));
        let q_prod = Arc::clone(&queue);
        let q_cons = Arc::clone(&queue);

        let producer = thread::spawn(move || {
            let _ = q_prod.push(1);
            let _ = q_prod.push(2);
        });

        let consumer = thread::spawn(move || {
            let _ = q_cons.pop();
            let _ = q_cons.pop();
        });

        producer.join().unwrap();
        consumer.join().unwrap();
    });
}

#[test]
fn test_ticket_lock_loom() {
    loom::model(|| {
        let lock = Arc::new(TicketLock::new());
        let l1 = Arc::clone(&lock);
        let l2 = Arc::clone(&lock);

        let t1 = thread::spawn(move || {
            let _guard = l1.lock();
        });

        let t2 = thread::spawn(move || {
            let _guard = l2.lock();
        });

        t1.join().unwrap();
        t2.join().unwrap();
    });
}
