// Copyright 2021 Gnosis Ltd.
// SPDX-License-Identifier: Apache-2.0

use reth_core::{transaction::TypePayload, Transaction, H256, U256};
use std::{
    cmp,
    collections::{BinaryHeap, HashSet},
    mem,
    ops::Deref,
    sync::Arc,
    time::Instant,
};

pub type Score = U256;

pub struct ByScore {
    /// all transactions sorted by min/max value
    sorting: BinaryHeap<ScoreTransaction>,
    /// pending hashes for removal. It is optimization for BinaryHeap because we dont want to recreate it every time.
    pending_removal: HashSet<H256>,
}

impl ByScore {
    pub fn new() -> Self {
        Self {
            sorting: BinaryHeap::new(),
            pending_removal: HashSet::new(),
        }
    }

    pub fn peek(&self) -> Option<&ScoreTransaction> {
        self.sorting.peek()
    }

    pub fn push(&mut self, tx: ScoreTransaction) {
        self.sorting.push(tx);
    }

    pub fn clone_heap(&self) -> BinaryHeap<ScoreTransaction> {
        self.sorting.clone()
    }

    pub fn remove(&mut self, hash: H256) {
        if self.sorting.peek().unwrap().hash() == hash {
            self.sorting.pop();
            loop {
                // pop last tx if there are pending for removal
                if let Some(tx) = self.sorting.peek().cloned() {
                    if self.pending_removal.contains(&tx.hash()) {
                        self.sorting.pop();
                        continue;
                    }
                }
                break;
            }
        } else {
            // mark tx for removal from by_score
            self.pending_removal.insert(hash);
        }
    }

    pub fn pending_removal(&self) -> usize {
        self.pending_removal.len()
    }

    pub fn recreate_heap(&mut self) {
        // we can use retain from BinaryHeap but it is currently experimental feature:
        // https://doc.rust-lang.org/std/collections/struct.BinaryHeap.html#method.retain
        let fresh_tx: Vec<_> = mem::take(&mut self.sorting)
            .into_vec()
            .into_iter()
            .filter(|tx| !self.pending_removal.contains(&tx.hash()))
            .collect();

        self.pending_removal.clear();
        self.sorting = BinaryHeap::from(fresh_tx);
    }

    pub fn pending_removal_remove(&mut self, hash: &H256) -> bool {
        self.pending_removal.remove(hash)
    }
}

#[derive(Debug)]
pub struct ScoreTransaction {
    pub score: Score, // mostly depends on gas_price but it is influenced by priority if tx is Local/Regular/Reinserted
    pub tx: Arc<Transaction>, // Transaction payload.
}

impl ScoreTransaction {
    pub fn hash(&self) -> H256 {
        self.tx.hash()
    }

    pub fn new(tx: Arc<Transaction>) -> ScoreTransaction {
        let score = match tx.type_payload {
            // TODO effective_gas_price();
            TypePayload::AccessList(ref al) => al.legacy_payload.gas_price,
            TypePayload::Legacy(ref legacy) => legacy.gas_price,
        };

        ScoreTransaction { tx: tx, score }
    }
}

impl Deref for ScoreTransaction {
    type Target = Transaction;

    fn deref(&self) -> &Self::Target {
        &self.tx
    }
}

impl Clone for ScoreTransaction {
    fn clone(&self) -> Self {
        ScoreTransaction {
            score: self.score.clone(),
            tx: self.tx.clone(),
        }
    }
}

// order by nonce then by time of insertion and tie break it with hash if needed.
impl Ord for ScoreTransaction {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        other
            .score
            .cmp(&self.score)
            .then(other.hash().cmp(&self.hash()))
    }
}

impl PartialOrd for ScoreTransaction {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for ScoreTransaction {
    fn eq(&self, other: &Self) -> bool {
        self.hash() == other.hash()
    }
}

impl Eq for ScoreTransaction {}
