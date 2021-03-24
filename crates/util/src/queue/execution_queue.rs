// Copyright 2020-2021 Gnosis Ltd.
// SPDX-License-Identifier: Apache-2.0

use super::async_queue::AsyncQueue;
use std::{sync::Arc, thread};

/// Creates thread and uses AsyncQueue to queue and execute
/// items that are received in queue function.
/// Queue is restricted by max_items and will return false if
/// we want enqueue items on full queue.
pub struct ExecutionQueue<ITEM>
where
    ITEM: 'static + Send,
{
    async_queue: Arc<AsyncQueue<ITEM>>,
    thread_handle: Option<thread::JoinHandle<()>>,
}

impl<ITEM> ExecutionQueue<ITEM>
where
    ITEM: 'static + Send,
{
    /// Create and spawn one thread that executed `exec` every time item is queue-ed.
    pub fn new<EXEC: FnMut(Vec<ITEM>) + Send + 'static>(
        max_items: usize,
        batch_size: usize,
        mut exec: EXEC,
        symbolic_name: &str,
    ) -> Self {
        let mut queue = ExecutionQueue {
            async_queue: Arc::new(AsyncQueue::new(max_items, batch_size)),
            thread_handle: None,
        };

        let async_queue = queue.async_queue.clone();
        // main thread logic is in here
        queue.thread_handle = Some(
            thread::Builder::new()
                .name(symbolic_name.to_string())
                .spawn(move || {
                    while let Some(item) = async_queue.wait_for_batch() {
                        exec(item)
                    }
                })
                .expect("Expect to run thread"),
        );
        queue
    }

    /// Add item to queue
    pub fn enqueue(&self, item: ITEM) -> bool {
        self.async_queue.enqueue(item)
    }

    /// End execution and close thread. Pending items will be abandoned.
    pub fn end(&mut self) {
        self.async_queue.end();
        if let Some(handle) = self.thread_handle.take() {
            handle.join().expect("Join handle should not panic");
        }
    }

    /// Items currently in queue.
    pub fn len(&self) -> usize {
        self.async_queue.len()
    }
}

#[cfg(test)]
mod tests {
    use std::{
        sync::{
            mpsc,
            mpsc::{Receiver, Sender},
        },
        time::Duration,
    };

    use super::*;

    #[test]
    fn simple_example_execution_queue() {
        let (tx, rx): (Sender<()>, Receiver<()>) = mpsc::channel();
        let ev = vec![0, 1, 2, 5, 10, 12, 13, 34, 45];
        let iv = ev.clone();
        let mut index = 0;
        let mut q = ExecutionQueue::new(
            10,
            1,
            move |item: Vec<u32>| {
                assert!(ev[index] == item[0]);
                index += 1;
                if index == ev.len() {
                    tx.send(()).expect("Expect to work for testing");
                }
            },
            "symb_name",
        );
        for i in iv.iter() {
            assert!(q.enqueue(*i))
        }
        rx.recv_timeout(Duration::from_secs(3))
            .expect("Expect to work for testing");
        q.end()
    }

    #[test]
    fn execute_batches_execution_queue() {
        let (tx, rx): (Sender<()>, Receiver<()>) = mpsc::channel();
        let ev = vec![0, 1, 2, 5, 10, 12, 13, 34, 45, 1, 2, 3, 4, 5, 6, 7];
        let iv = ev.clone();
        let mut index = 0;
        let mut at_least_one_3batch = false;
        let mut q = ExecutionQueue::new(
            20,
            3,
            move |item: Vec<u32>| {
                if !at_least_one_3batch {
                    at_least_one_3batch = item.len() == 3;
                }
                for i in item {
                    assert!(ev[index] == i);
                    index += 1;
                }
                if index == ev.len() {
                    assert!(
                        at_least_one_3batch,
                        "We expect at least one batch of 3 items"
                    );
                    tx.send(()).expect("Expect to work for testing");
                }
            },
            "symb_name",
        );
        for i in iv.iter() {
            assert!(q.enqueue(*i), "should insert all items")
        }
        rx.recv_timeout(Duration::from_secs(3))
            .expect("Expect to work for testing");
        q.end()
    }

    #[test]
    fn start_stop_execution_queue() {
        let mut q = ExecutionQueue::new(10, 1, move |_: Vec<u32>| {}, "");
        q.end()
    }

    #[test]
    fn overfill_execution_queue() {
        let (tx, rx): (Sender<()>, Receiver<()>) = mpsc::channel();

        let mut q = ExecutionQueue::new(
            1,
            1,
            move |_: Vec<u32>| {
                rx.recv_timeout(Duration::from_secs(3))
                    .expect("Expect to work for testing");
            },
            "",
        );

        assert_eq!(q.enqueue(10), true); // one to be currently executed
        thread::sleep(Duration::from_millis(100)); // wait for queue to take that item for execution
        assert_eq!(q.enqueue(15), true); // one queue in VecQueue
        assert_eq!(q.enqueue(15), false); // queue is full
        tx.send(()).expect("Expect to work for testing"); // process one item
        thread::sleep(Duration::from_millis(100)); // sleep so that queue continue processing current item and take next one.
        assert_eq!(q.enqueue(15), true); // one item added
        assert_eq!(q.enqueue(15), false); // queue is full

        // clean queue so that it does not panic in closure.
        tx.send(()).expect("Expect to work for testing");
        tx.send(()).expect("Expect to work for testing");
        tx.send(()).expect("Expect to work for testing");

        // close thread
        q.end()
    }
}
