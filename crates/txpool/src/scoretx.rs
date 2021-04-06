// Copyright 2021 Gnosis Ltd.
// SPDX-License-Identifier: Apache-2.0

use core::{Transaction, H256, U256};
use std::{cmp, time::Instant};

#[derive(Debug, Clone, Copy)]
pub enum Priority {
    Local,
    Retracted,
    Regular,
}

pub type Score = U256;

#[derive(Debug)]
pub struct ScoreTransaction {
    pub score: Score, // mostly depends on gas_price but it is influenced by priority if tx is Local/Regular/Reinserted
    pub hash: H256, // identifier of transaction
    pub timestamp: Instant, // it it used to remove stale transaction
    pub priority: Priority, // Priority of transaction
    
    pub transaction: Transaction, // Transaction payload.
}

impl ScoreTransaction {

    pub fn hash(&self) -> H256 {
        self.transaction.hash()
    }
}

impl Clone for ScoreTransaction {
    fn clone(&self) -> Self {
        ScoreTransaction {
            priority: self.priority,
            score: self.score.clone(),
            hash: self.hash,
            transaction: self.transaction.clone(),
            timestamp: Instant::now(),
        }
    }
}

// order by nonce then by time of insertion and tie break it with hash if needed.
impl Ord for ScoreTransaction {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        other.score.cmp(&self.score).then(
            other
                .timestamp
                .cmp(&self.timestamp)
                .then(other.hash().cmp(&self.hash())),
        )
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
