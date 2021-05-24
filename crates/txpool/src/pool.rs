// Copyright 2021 Gnosis Ltd.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    config::Config, transactions::BlockInfo, Announcer, Error, Find, ScoreTransaction, Transactions,
};
use async_trait::async_trait;
use futures::future::join_all;
use interfaces::{
    txpool::TransactionPool,
    world_state::{BlockUpdate, WorldState},
};
use parking_lot::RwLock;
use reth_core::{Address, BlockId, Transaction, H256, U256};
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

pub struct PendingBlock {
    pub tx: Vec<ScoreTransaction>,
    pub gas_price: U256,
}

/// Transaction pool.
pub struct Pool {
    txs: Arc<RwLock<Transactions>>,
    /// configuration of pool
    config: Arc<Config>,

    /// World state
    world_state: Arc<dyn WorldState>,

    announcer: Arc<dyn Announcer>,
}

impl Pool {
    /// TODO check how we are going to get best block from world_state
    /// and when we are going to officially start receiving tx from sentry
    pub fn new(
        config: Arc<Config>,
        world_state: Arc<dyn WorldState>,
        announcer: Arc<dyn Announcer>,
    ) -> Pool {
        //TODO let best_block = world_state.best_block().await
        let best_block = BlockInfo {
            base_fee: 0.into(),
            hash: H256::zero(),
        };

        let pool = Pool {
            txs: Arc::new(RwLock::new(Transactions::new(config.clone(), best_block))),
            config: config.clone(),
            world_state,
            announcer,
        };

        //TODO spinup checkers. One timer and one for recreating BinaryHeap

        pool
    }

    pub fn config(&self) -> &Config {
        &self.config
    }

    /// Get transaction for pending blocks
    pub async fn new_pending_block(&self) -> (Vec<Arc<Transaction>>, H256) {
        //iterate over sorted tx to create new pending block tx
        let (sorted, infos, parent_hash) = self.txs.write().sorted_vec_and_accounts();
        let mut out = Vec::new();
        let mut nonces: HashMap<Address, u64> = HashMap::new();
        for tx in sorted.into_iter().rev() {
            let author = tx
                .author()
                .expect("Every inserted transaction has check if author is set")
                .0;
            let nonce = nonces
                .entry(author)
                .or_insert_with(|| infos.get(&author).expect("Acc info should be present").nonce);
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
        (out, parent_hash)
    }
}

#[async_trait]
impl TransactionPool for Pool {
    async fn filter_by_negative(&self, tx_hashes: &[H256]) -> Vec<H256> {
        let hashset: HashSet<H256> = tx_hashes.iter().cloned().collect::<HashSet<H256>>();
        self.txs
            .read()
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
                let (address, _) = tx.recover_author()?;
                let tx = Arc::new(tx);
                let mut replaced = Vec::new();

                // Loop below is a way to avoid calling world_state.account_info from rwlocked pool and with that
                // blocking all operation for extended period while we are waiting for response.
                // In best (most used) case it should loop only once.
                // Block hash is used as kind of identifier to check if pool is changed or not.
                loop {
                    // pool read lock. Get account info and block hash from pool.
                    let (info, block_hash) = {
                        let txs_pool = self.txs.read();
                        (txs_pool.account(&address).cloned(), txs_pool.block().hash)
                    };
                    // if there is account present in pool use it.
                    let acc_and_block_hash = if info.is_some() {
                        info.map(|t| Some((t, block_hash))).flatten()
                    } else {
                        // if there is no account known fetch account info from world_state.
                        let info = self
                            .world_state
                            .account_info(BlockId::Hash(block_hash), address)
                            .await
                            .unwrap_or_default();
                        Some((info, block_hash))
                    };
                    // pool write lock. Insert tx into pool with provided account info.
                    match self
                        .txs
                        .write()
                        .insert(tx.clone(), false, acc_and_block_hash)
                    {
                        // Hurray, we included tx into pool
                        Ok(rem) => {
                            replaced = rem;
                            break;
                        }
                        Err(err) => {
                            // account info got obsolete, that means that
                            // new block was included/retracted and block hash is changed.
                            // loop over and ping world_state for new info
                            if let Some(Error::InternalAccountObsolete) =
                                err.downcast_ref::<Error>()
                            {
                                continue;
                            }
                            return Err(err);
                        }
                    };
                }
                // announce change in pool
                for rem in replaced {
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
                    .find(Find::TxByHash(*hash))
                    .map(|tx| tx.encode()),
            )
        }
        out
    }

    async fn remove(&self, hashes: &[H256]) {
        for hash in hashes.iter() {
            let tx = self.txs.write().remove(&hash);
            if let Some(tx) = tx {
                self.announcer.removed(tx, Error::RemovedTxOnDemand).await;
            }
        }
    }

    async fn block_update(&self, update: &BlockUpdate) {
        self.txs.write().block_update(update);
        //TODO announce imported/removed tx
    }
}

#[cfg(test)]
#[allow(non_snake_case)]
mod tests {
    use super::*;
    use crate::announcer::test::AnnouncerTest;
    use interfaces::world_state::helper::WorldStateTest;

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
