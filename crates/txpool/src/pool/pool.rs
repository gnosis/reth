// Copyright 2021 Gnosis Ltd.
// SPDX-License-Identifier: Apache-2.0

use super::{transactions::BlockInfo, ScoreTransaction, Transactions};
use crate::{config::Config, Announcer, Error};
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
    time::Duration,
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
    /// and when are we going to officially start receiving tx from sentry
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
            announcer: announcer.clone(),
        };

        // periodic check for timing out tx and checking to recreate binary_heap.
        let txs = pool.txs.clone();
        let annon = announcer.clone();
        let _ = tokio::spawn(async move {
            loop {
                let rem = txs.write().periodic_check();
                for rem in rem {
                    annon.removed(rem, Error::RemovedTxTimeout).await;
                }
                tokio::time::sleep(Duration::from_secs(30)).await;
            }
        });

        pool
    }

    pub fn config(&self) -> &Config {
        &self.config
    }

    /// Get transaction for pending blocks
    pub async fn new_pending_block(&self) -> (Vec<Arc<Transaction>>, BlockInfo) {
        //iterate over sorted tx to create new pending block tx
        let (binary_heap, infos, block) = self.txs.write().binary_heap_and_accounts();
        let sorted = binary_heap.into_sorted_vec();
        let mut out = Vec::new();
        let mut nonces: HashMap<Address, u64> = HashMap::new();
        for tx in sorted.into_iter().rev() {
            if tx.score >= block.base_fee {
                break;
            }
            let author = tx
                .author()
                .expect("Every inserted transaction has check if author is set")
                .0;
            let nonce = nonces.entry(author).or_insert_with(|| {
                infos
                    .get(&author)
                    .expect("Acc info should be present")
                    .nonce
            });
            if *nonce == tx.nonce.as_u64() {
                out.push(tx.tx.clone());
                *nonce = *nonce + 1;
            }
        }
        // TODO discuss. if this use case needs to be covered:
        // If we have tx0 and tx1 from same author with nonces 0 and 1,
        // and tx1 has better score then tx0, that would mean that when iterating we are going to skip tx1
        // and include only tx0. Should we tranverse back and try to include tx1 again?
        // edge case: if including tx1 removes some tx from pending block (or even removes tx0 ).
        (out, block)
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

    async fn import(&self, txs: Vec<Arc<Transaction>>) -> Vec<anyhow::Result<()>> {
        let mut handlers = Vec::with_capacity(txs.len());
        for tx in txs.into_iter() {
            let tx = tx.clone();
            handlers.push(async move {
                if !tx.has_author() {
                    return Err(Error::TxAuthorUnknown.into());
                }
                let (address, _) = tx.author().unwrap();
                let replaced;

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
                    match self.txs.write().insert(tx.clone(), acc_and_block_hash) {
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
                            // there is error on inclusion of tx.
                            return Err(err);
                        }
                    };
                }
                // announce change in pool
                for (tx, reason) in replaced {
                    self.announcer.removed(tx, reason).await;
                }
                self.announcer.inserted(tx).await;
                Ok(())
            });
        }

        join_all(handlers).await
    }

    async fn find(&self, hashes: &[H256]) -> Vec<Option<Arc<Transaction>>> {
        let mut out = Vec::with_capacity(hashes.len());
        let txs = self.txs.read();
        for hash in hashes.into_iter() {
            out.push(txs.find_by_hash(hash))
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
        let (removed, reinserted) = self.txs.write().block_update(update);
        for (rem, reason) in removed {
            self.announcer.removed(rem, reason).await;
        }
        for reinsert in reinserted {
            self.announcer.reinserted(reinsert).await;
        }
    }
}

#[cfg(test)]
#[allow(non_snake_case)]
mod tests {
    use super::{super::announcer::test::AnnouncerTest, *};
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
            pool.import(vec![]).await;
        }
    }
}
