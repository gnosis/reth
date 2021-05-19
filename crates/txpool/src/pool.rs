// Copyright 2021 Gnosis Ltd.
// SPDX-License-Identifier: Apache-2.0

use crate::{config::Config, Find, ScoreTransaction, Transactions};
use async_trait::async_trait;
use futures::future::join_all;
use interfaces::{txpool::{TransactionPool}, world_state::{BlockUpdate, WorldState}};
use reth_core::{Address, Transaction, H256, U256};
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};
use tokio::sync::RwLock;
//use parking_lot::RwLock;

pub struct PendingBlock {
    pub tx: Vec<ScoreTransaction>,
    pub gas_price: U256,
}

/// Transaction pool.
pub struct Pool {
    txs: Arc<RwLock<Transactions>>,
    /// configuration of pool
    config: Arc<Config>,

    //pending_block: RwLock<Option<PendingBlock>>,
}

impl Pool {
    // currently hardcoded
    pub fn new(config: Arc<Config>, world_state: Arc<dyn WorldState>) -> Pool {
        Pool {
            txs: Arc::new(RwLock::new(Transactions::new(
                config.clone(),
                world_state,
            ))),
            config: config.clone(),
            //pending_block: RwLock::new(None),
        }
    }

    pub fn config(&self) -> &Config {
        &self.config
    }

    /// Create pending blocks
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
    async fn filter_by_negative(&self, tx_hashes: Vec<H256>) -> Vec<H256> {
        let hashset: HashSet<H256> = tx_hashes.into_iter().collect::<HashSet<H256>>();
        self.txs
            .read()
            .await
            .iter_unordered()
            .filter(|&(hash, _)| !hashset.contains(hash))
            .map(|tx| tx.0.clone())
            .collect()
    }

    async fn import(&self, txs: Vec<Vec<u8>>) -> Vec<anyhow::Result<()>> {
        let mut handlers = Vec::with_capacity(txs.len());
        for raw_tx in txs.into_iter() {
            let txpool = self.txs.clone();
            handlers.push(async move {
                let mut tx = Transaction::decode(&raw_tx)?;
                tx.recover_author()?;
                txpool.write().await.insert(Arc::new(tx), false).await?;
                Ok(())
            });
        }

        join_all(handlers).await
    }

    async fn find(&self, hashes: Vec<H256>) -> Vec<Option<Vec<u8>>> {
        let mut out = Vec::with_capacity(hashes.len());
        for hash in hashes.into_iter() {
            out.push(
                self.txs
                    .read()
                    .await
                    .find(Find::TxByHash(hash))
                    .map(|tx| tx.encode()),
            )
        }
        out
    }

    async fn remove(&self, hashes: Vec<H256>) {
        for hash in hashes.iter() {
            self.txs.write().await.remove(&hash);
        }
    }

    async fn block_update(&self, update: &BlockUpdate) {
        self.txs.write().await.block_update(update).await;
    }
}

#[cfg(test)]
#[allow(non_snake_case)]
mod tests {
    use super::*;
    use interfaces::world_state::helper::WorldStateTest;

    #[tokio::test]
    async fn smoke_test() {
        //Create objects
        let pool = Arc::new(Pool::new(
            Arc::new(Config::default()),
            WorldStateTest::new_dummy(),
        ));
        {
            pool.import(vec![]).await;
        }
    }
}
