// Copyright 2021 Gnosis Ltd.
// SPDX-License-Identifier: Apache-2.0

use crate::{Announcer, Find, ScoreTransaction, Transactions, config::Config, Error};
use async_trait::async_trait;
use futures::future::join_all;
use interfaces::{txpool::{TransactionPool}, world_state::{BlockUpdate, WorldState}};
use reth_core::{Address, Transaction, H256, U256};
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};
use tokio::sync::RwLock;

pub struct PendingBlock {
    pub tx: Vec<ScoreTransaction>,
    pub gas_price: U256,
}

/// Transaction pool.
pub struct Pool {
    txs: Arc<RwLock<Transactions>>,
    /// configuration of pool
    config: Arc<Config>,

    announcer: Arc<dyn Announcer>,
}

impl Pool {
    // currently hardcoded
    pub fn new(config: Arc<Config>, world_state: Arc<dyn WorldState>, announcer: Arc<dyn Announcer>) -> Pool {
        Pool {
            txs: Arc::new(RwLock::new(Transactions::new(
                config.clone(),
                world_state,
            ))),
            config: config.clone(),
            announcer,
        }
    }

    pub fn config(&self) -> &Config {
        &self.config
    }

    /// Get transaction for pending blocks
    pub async fn new_pending_block(&self) -> Vec<Arc<Transaction>> {
        //iterate over sorted tx to create new pending block tx
        // Check max nonce
        let sorted = self.txs.write().await.sorted_vec();
        let mut out = Vec::new();
        let mut nonces: HashMap<Address, u64> = HashMap::new();
        for tx in sorted {
            let author = tx
                .author()
                .expect("Every inserted transaction has check if author is set")
                .0;
            let nonce = nonces
                .entry(author)
                .or_insert_with(|| 0 /* TODO self.world_state.account_info().nonce+1*/);
            if *nonce == tx.nonce.as_u64() {
                out.push(tx.tx.clone());
                *nonce = *nonce + 1;
            }
        }
        // TODO discuss if this use case needs to be covered:
        // If we have tx0 and tx1 from same author with nonces 0 and 1,
        // and tx1 has better score then tx0, that would mean that when iterating we are going to skip tx1
        // and include only tx0. Should we tranverse back and try to include tx1 again?
        // edge case: if including tx1 removes some tx from pending block (or even removes tx0 ).
        out
    }
}

#[async_trait]
impl TransactionPool for Pool {
    async fn filter_by_negative(&self, tx_hashes: &[H256]) -> Vec<H256> {
        let hashset: HashSet<H256> = tx_hashes.iter().cloned().collect::<HashSet<H256>>();
        self.txs
            .read()
            .await
            .iter_unordered()
            .filter(|&(hash, _)| !hashset.contains(hash))
            .map(|tx| tx.0.clone())
            .collect()
    }

    async fn import(&self, txs: &[Vec<u8>]) -> Vec<anyhow::Result<()>> {
        let mut handlers = Vec::with_capacity(txs.len());
        for raw_tx in txs.into_iter() {
            handlers.push(async move {
                let mut tx = Transaction::decode(&raw_tx)?;
                tx.recover_author()?;
                let tx = Arc::new(tx);
                let rem = self.txs.write().await.insert(tx.clone(), false).await?;
                // announce change in pool
                self.announcer.inserted(tx).await;
                for rem in rem {
                    self.announcer.removed(rem, Error::RemovedTxReplaced).await;
                }
                Ok(())
            });
        }

        join_all(handlers).await
    }

    async fn find(&self, hashes: &[H256]) -> Vec<Option<Vec<u8>>> {
        let mut out = Vec::with_capacity(hashes.len());
        for hash in hashes.into_iter() {
            out.push(
                self.txs
                    .read()
                    .await
                    .find(Find::TxByHash(*hash))
                    .map(|tx| tx.encode()),
            )
        }
        out
    }

    async fn remove(&self, hashes: &[H256]) {
        for hash in hashes.iter() {
            if let Some(tx) = self.txs.write().await.remove(&hash) {
                self.announcer.removed(tx, Error::RemovedTxOnDemand).await;
            }
        }
    }

    async fn block_update(&self, update: &BlockUpdate) {
        self.txs.write().await.block_update(update).await;
        //TODO announce imported/removed tx
    }
}

#[cfg(test)]
#[allow(non_snake_case)]
mod tests {
    use super::*;
    use interfaces::world_state::helper::WorldStateTest;
    use crate::announcer::test::AnnouncerTest;

    #[tokio::test]
    async fn smoke_test() {
        //Create objects
        let pool = Arc::new(Pool::new(
            Arc::new(Config::default()),
            WorldStateTest::new_dummy(),
            Arc::new(AnnouncerTest::new()),
        ));
        {
            pool.import(&vec![]).await;
        }
    }
}
