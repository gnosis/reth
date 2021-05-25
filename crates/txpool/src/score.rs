// Copyright 2021 Gnosis Ltd.
// SPDX-License-Identifier: Apache-2.0

use reth_core::{transaction::TypePayload, Transaction, H256, U256};
use std::{cmp, ops::Deref, sync::Arc, time::Instant};

pub type Score = U256;

#[derive(Debug)]
pub struct ScoreTransaction {
    pub score: Score, // mostly depends on gas_price but it is influenced by priority if tx is Local/Regular/Reinserted
    pub timestamp: Instant, // it it used to remove stale transaction
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

        ScoreTransaction {
            tx: tx,
            score,
            timestamp: Instant::now(),
        }
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
