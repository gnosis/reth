// Copyright 2020-2021 Gnosis Ltd.
// SPDX-License-Identifier: Apache-2.0

use std::{
    collections::VecDeque,
    sync::{Condvar, Mutex},
};

/// AsyncQueue is queue that uses Condvar
/// to signal between two threads.
pub struct AsyncQueue<ITEM>
where
    ITEM: 'static + Send,
{
    items: Mutex<Option<VecDeque<ITEM>>>,
    cvar: Condvar,
    max_items: usize,
    batch_size: usize,
}

impl<ITEM> AsyncQueue<ITEM>
where
    ITEM: 'static + Send,
{
    /// Create new AsyncQueue. limit number of items with max_items
    pub fn new(max_items: usize, mut batch_size: usize) -> Self {
        if batch_size == 0 {
            batch_size = max_items;
        }
        AsyncQueue {
            items: Mutex::new(Some(VecDeque::new())),
            cvar: Condvar::new(),
            max_items,
            batch_size,
        }
    }

    /// Add items to qeuue and notify waiting `wait_for_item` calls
    pub fn enqueue(&self, item: ITEM) -> bool {
        let mut items = self.items.lock().unwrap();
        if let Some(items) = items.as_mut() {
            if items.len() >= self.max_items {
                return false;
            }
            items.push_back(item);
            self.cvar.notify_all();
            return true;
        }
        false
    }

    /// if there is items in queue return first item otherwise wait for condvar notification
    pub fn wait_for_batch(&self) -> Option<Vec<ITEM>> {
        let mut items_guarded = self.items.lock().unwrap();
        loop {
            let items = items_guarded.as_mut()?; // return if items_guarded is none.
            match items.len() {
                0 => items_guarded = self.cvar.wait(items_guarded).unwrap(),
                len if len < self.batch_size => {
                    return Some(items.drain(..len).collect());
                }
                _ => {
                    return Some(items.drain(..self.batch_size).collect());
                }
            }
        }
    }

    /// nulify items and notify waiting `wait_for_item` calls
    pub fn end(&self) {
        let mut items = self.items.lock().unwrap();
        *items = None;
        self.cvar.notify_all();
    }

    /// return lengs of queue if it is valid.
    pub fn len(&self) -> usize {
        match self.items.lock().unwrap().as_ref() {
            Some(items) => items.len(),
            None => 0,
        }
    }
}

#[cfg(test)]
mod tests {
    // it is better tested in ExecutionQueue with async logic behind
    use super::*;

    #[test]
    fn simple_test() {
        let aq = AsyncQueue::new(5, 2);
        assert_eq!(aq.enqueue(1), true);
        assert_eq!(aq.enqueue(2), true);
        assert_eq!(aq.enqueue(3), true);
        assert_eq!(aq.enqueue(4), true);
        assert_eq!(aq.enqueue(5), true);
        assert_eq!(aq.enqueue(10), false);
        assert_eq!(aq.enqueue(11), false);

        assert_eq!(aq.wait_for_batch(), Some(vec![1, 2]));
        assert_eq!(aq.wait_for_batch(), Some(vec![3, 4]));
        assert_eq!(aq.wait_for_batch(), Some(vec![5]));
    }

    #[test]
    fn simple_test_2() {
        let aq = AsyncQueue::new(5, 2);
        assert_eq!(aq.enqueue(1), true);
        assert_eq!(aq.wait_for_batch(), Some(vec![1]));
        assert_eq!(aq.enqueue(2), true);
        assert_eq!(aq.enqueue(3), true);
        assert_eq!(aq.wait_for_batch(), Some(vec![2, 3]));
        assert_eq!(aq.enqueue(4), true);
        assert_eq!(aq.enqueue(5), true);
        assert_eq!(aq.enqueue(10), true);
        assert_eq!(aq.wait_for_batch(), Some(vec![4, 5]));
        assert_eq!(aq.enqueue(11), true);
        assert_eq!(aq.wait_for_batch(), Some(vec![10, 11]));
    }

    #[test]
    fn stop_enqueue_after_end() {
        let aq = AsyncQueue::new(5, 2);
        assert_eq!(aq.enqueue(1), true);
        assert_eq!(aq.enqueue(2), true);
        assert_eq!(aq.enqueue(3), true);
        aq.end();
        assert_eq!(aq.wait_for_batch(), None);
        assert_eq!(aq.enqueue(5), false);
        assert_eq!(aq.enqueue(6), false);
    }
}
