// Copyright 2021 Gnosis Ltd.
// SPDX-License-Identifier: Apache-2.0

use interfaces::world_state::{BlockUpdate, WorldState};
use log::*;
use reth_core::{Address, BlockId, Transaction, H256};
use std::{
    collections::{
        hash_map::{Entry, Iter},
        BinaryHeap, HashMap, HashSet,
    },
    mem,
    sync::Arc,
    time::{Duration, Instant},
};

use anyhow::Result;

use crate::{account::Account, Config, Error, Priority, ScoreTransaction};
pub enum Find {
    LastAccountTx(Address),
    BestAccountTx(Address),
    TxByHash(H256),
}

pub struct Transactions {
    /// transaction by account sorted in increasing order
    by_account: HashMap<Address, Account>,
    /// all transaction by its hash
    by_hash: HashMap<H256, Arc<Transaction>>,
    /// all transactions sorted by min/max value
    by_score: BinaryHeap<ScoreTransaction>,
    /// pending hashes for removal. It is optimization for BinaryHeap because we dont want to recreate it every time.
    pending_removal: HashSet<H256>,

    config: Arc<Config>,

    /// needed to get nonce/balance from db
    world_state: Arc<dyn WorldState>,
}

impl Transactions {
    pub fn new(config: Arc<Config>, world_state: Arc<dyn WorldState>) -> Self {
        Self {
            by_account: HashMap::new(),
            by_hash: HashMap::new(),
            by_score: BinaryHeap::new(),
            pending_removal: HashSet::new(),
            config,
            world_state,
        }
    }

    /// find one particular transaction
    pub fn find(&self, cond: Find) -> Option<Arc<Transaction>> {
        match cond {
            Find::LastAccountTx(address) => self
                .by_account
                .get(&address)
                .map(|account| account.txs().last().cloned())
                .flatten(),
            Find::BestAccountTx(address) => self
                .by_account
                .get(&address)
                .map(|account| account.txs().first().cloned())
                .flatten(),
            Find::TxByHash(hash) => self.by_hash.get(&hash).cloned(),
        }
    }

    /// insert transaction into pool
    /// Return Ok with list of removed transaction.
    /// Reasons why transaction can be replaced by this tx:
    /// 1. Tx with same nonce
    /// 2. Account does not have enought balance for transaction with higher nonce
    /// 3. Limit on Max transaction hit.
    pub async fn insert(
        &mut self,
        tx: Arc<Transaction>,
        is_local: bool,
    ) -> Result<Vec<Arc<Transaction>>> {
        // check if we already have tx
        if self.by_hash.contains_key(&tx.hash()) {
            return Err(Error::AlreadyPresent.into());
        }

        // check if transaction is signed and author is present
        let author = tx.author().ok_or(Error::TxAuthorUnknown)?.0;
        let priority = if is_local {
            Priority::Local
        } else {
            Priority::Regular
        };
        // create scored transaction
        let scoredtx = ScoreTransaction::new(tx, priority);

        // check if we are at pool max limit and if our tx has better score then the worst one
        if self.by_hash.len() >= self.config.max
            && scoredtx.score <= self.by_score.peek().unwrap().score
        {
            return Err(Error::NotInsertedPoolFullIncreaseGas.into());
        }

        // search tx in by_account sorted by nonce
        let acc = match self.by_account.entry(author) {
            Entry::Occupied(entry) => entry.into_mut(),
            Entry::Vacant(entry) => {
                // if account info is not present, get data from world_state database.
                // TODO this can be potential bottleneck
                let info = self
                    .world_state
                    .account_info(BlockId::Latest, author.clone())
                    .await
                    .ok_or(Error::NotInsertedAccountUnknown)?;
                // Check nonces
                if info.nonce > scoredtx.nonce.as_u64() {
                    return Err(Error::NotInsertedWrongNonce.into());
                }
                entry.insert(Account::new(info))
            }
        };

        let (replaced, mut removed) = acc.insert(&scoredtx, priority, self.config.per_account)?;

        // if there is replaced tx remove it here from rest of structures.
        if let Some(ref rem) = replaced {
            let hash = rem.hash();
            self.by_hash.remove(&hash);
            self.by_score_remove(hash)
        }

        // remove it from removed list if present. (This is edge case, should happen only very rarely).
        self.pending_removal.remove(&scoredtx.hash());

        // insert to other structures
        self.by_score.push(scoredtx.clone());
        self.by_hash.insert(scoredtx.hash(), scoredtx.tx.clone());

        for tx in removed.iter() {
            self.remove(&tx.hash());
        }

        // add replaced tx to list of removed tx.
        if let Some(ref rep) = replaced {
            removed.push(rep.clone());
        }

        // remove transaction if we hit limit
        if self.by_hash.len() > self.config.max {
            // we dont check if it is local or not, because score for Local should be a lot higher.
            // and max_tx_count should be hard limit.
            let worst_scoredtx = self.by_score.peek().unwrap().tx.hash();
            let rem = self.remove(&worst_scoredtx);
            if let Some(rem) = rem {
                removed.push(rem.clone());
            }
        }

        Ok(removed)
    }

    #[inline]
    fn by_score_remove(&mut self, hash: H256) {
        if self.by_score.peek().unwrap().hash() == hash {
            self.by_score.pop();
        } else {
            // mark tx for removal from by_score
            self.pending_removal.insert(hash);
        }
    }

    pub fn remove(&mut self, txhash: &H256) -> Option<Arc<Transaction>> {
        // remove tx from by_hash
        if let Some(tx) = self.by_hash.remove(txhash) {
            // remove tx from by_accounts
            let author = tx.author().unwrap().0;
            if self
                .by_account
                .get_mut(&author)
                .expect("Expect to account to contain specific tx")
                .remove(txhash)
            {
                self.by_account.remove(&author);
            }
            // if transaction is at end of binaryheap just remove it.
            self.by_score_remove(*txhash);
            Some(tx.clone())
        } else {
            None
        }
    }

    pub fn recreate_heap(&mut self) {
        // we can use retain from BinaryHeap but it is currently experimental feature:
        // https://doc.rust-lang.org/std/collections/struct.BinaryHeap.html#method.retain
        let fresh_tx: Vec<_> = mem::replace(&mut self.by_score, BinaryHeap::new())
            .into_vec()
            .into_iter()
            .filter(|tx| !self.pending_removal.contains(&tx.hash()))
            .collect();

        self.pending_removal.clear();
        self.by_score = BinaryHeap::from(fresh_tx);
    }

    /// remove all stalled transactions
    /// Iterates over all transactions and remove all that is inserted 600s ago.
    fn remove_stalled(&mut self, threshold: Duration) -> usize {
        let removal_threshold = Instant::now() - threshold;

        let remove: Vec<H256> = self
            .by_score
            .iter()
            .filter(|&tx| tx.timestamp < removal_threshold)
            .map(|tx| tx.hash())
            .collect();
        let cnt_removed = remove.len();
        for hash in remove {
            self.remove(&hash).unwrap();
        }
        self.recreate_heap();
        cnt_removed
    }

    /// Iterates over all transactions in pool.
    pub fn iter_unordered(&self) -> Iter<'_, H256, Arc<Transaction>> {
        self.by_hash.iter()
    }

    /// Return all transactions in pool sorted from best to worst score.
    /// This is expensive opperation and it copies all transaction.
    pub fn sorted_vec(&mut self) -> Vec<ScoreTransaction> {
        self.recreate_heap();
        self.by_score.clone().into_sorted_vec()
    }

    pub async fn block_update(&mut self, update: &BlockUpdate) {
        // reverted nonces
        for (address, info) in update.reverted_accounts.iter() {
            if let Some(account) = self.by_account.get_mut(&address) {
                // we are okay to just replace it. Tx in list allready had enought balance and revert.
                account.set_info(*info);
            }
        }

        // applied nonces. We need to check nonce and gas of all transaction in list and remove ones with insufficient gas or nonce.
        for (address, info) in update.applied_accounts.iter() {
            let mut rem_old_nonce = Vec::new();
            let mut rem_balance = Vec::new();
            if let Some(account) = self.by_account.get_mut(&address) {
                account.set_info(*info);
                // remove tx with lower nonce
                account
                    .txs()
                    .iter()
                    .take_while(|tx| tx.nonce.as_u64() <= info.nonce)
                    .for_each(|tx| rem_old_nonce.push(tx.hash()));
                //remove all hashes
                let mut balance = info.balance;
                for tx in account.txs().iter().skip(rem_old_nonce.len()) {
                    let cost = tx.cost();
                    if cost > balance {
                        rem_balance.push(tx.hash());
                    }
                    balance = balance.saturating_sub(cost);
                }
                // change to new balance and nonce. We are okay to set it here, because remove is donny and does not chage account info.
                account.set_info(*info);
            }

            rem_old_nonce.iter().for_each(|hash| {
                self.remove(hash);
            });
            rem_balance.iter().for_each(|hash| {
                self.remove(hash);
            });
        }

        // reinsert new tx
        for rawtx in update.reverted_tx.iter() {
            if let Ok(mut tx) = Transaction::decode(rawtx) {
                if tx.recover_author().is_ok() {
                    if self.insert(Arc::new(tx), true).await.is_err() {
                        warn!("We got bad transaction in reverted block:{:?}", rawtx);
                    }
                }
            }
        }

        // this is big change to pool. Lest recreashe binary heap
        self.recreate_heap();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use interfaces::world_state::helper::WorldStateTest;
    use reth_core::transaction::{LegacyPayload, TypePayload, transaction::{fake_sign, DUMMY_AUTHOR, DUMMY_AUTHOR1}};
    use tokio_test::assert_ok;

    /// 1000 gs per account that can be found in WorldStateTest round up to around 22 score.
    ///
    fn new_tx(hash: H256, score: usize, nonce: usize, author: Address) -> Arc<Transaction> {
        let mut tx = Transaction::default();
        tx.gas_limit = (score * 4).into();
        tx.type_payload = TypePayload::Legacy(LegacyPayload {
            gas_price: score.into(),
        });
        tx = fake_sign(tx, author);
        tx.set_hash(hash);
        tx.nonce = nonce.into();
        Arc::new(tx)
    }

    fn hashes() -> (H256, H256, H256) {
        (
            H256::from_low_u64_be(1),
            H256::from_low_u64_be(2),
            H256::from_low_u64_be(3),
        )
    }

    #[tokio::test]
    async fn insert_items_get_sorted() -> Result<()> {
        //data
        let config = Arc::new(Config {
            max: 100,
            per_account: 10,
        });
        let mut txpool = Transactions::new(config, WorldStateTest::new_dummy());
        let (h1, h2, h3) = hashes();

        let tx1 = new_tx(h1, 2, 1, DUMMY_AUTHOR.0);
        let tx2 = new_tx(h2, 4, 2, DUMMY_AUTHOR.0);
        let tx3 = new_tx(h3, 3, 3, DUMMY_AUTHOR.0);

        //test
        txpool.insert(tx1, false).await?;
        txpool.insert(tx2, false).await?;
        txpool.insert(tx3, false).await?;

        //check
        assert_eq!(txpool.iter_unordered().len(), 3);

        //check ordered
        let sorted = txpool
            .sorted_vec()
            .into_iter()
            .map(|t| t.hash())
            .collect::<Vec<_>>();
        assert_eq!(sorted, vec![h2, h3, h1]);

        Ok(())
    }

    #[tokio::test]
    async fn insert_remove() -> Result<()> {
        //data
        let config = Arc::new(Config {
            max: 100,
            per_account: 10,
        });
        let mut txpool = Transactions::new(config, WorldStateTest::new_dummy());
        let (h1, h2, h3) = hashes();

        let tx1 = new_tx(h1, 2, 1, DUMMY_AUTHOR.0);
        let tx2 = new_tx(h2, 4, 2, DUMMY_AUTHOR.0);
        let tx3 = new_tx(h3, 3, 3, DUMMY_AUTHOR.0);

        //test
        txpool.insert(tx1, false).await?;
        txpool.insert(tx2, false).await?;
        txpool.insert(tx3, false).await?;
        txpool.remove(&h2);

        //check
        assert_eq!(txpool.iter_unordered().len(), 2);

        let sorted = txpool
            .sorted_vec()
            .into_iter()
            .map(|t| t.hash())
            .collect::<Vec<_>>();
        assert_eq!(sorted, vec![h3, h1]);
        Ok(())
    }

    #[tokio::test]
    async fn should_replace_nonce() -> Result<()> {
        //data
        let config = Arc::new(Config {
            max: 100,
            per_account: 10,
        });
        let mut txpool = Transactions::new(config, WorldStateTest::new_dummy());
        let (h1, h2, h3) = hashes();

        let tx1 = new_tx(h1, 2, 1, DUMMY_AUTHOR.0);
        let tx2 = new_tx(h2, 4, 1, DUMMY_AUTHOR.0);
        let tx3 = new_tx(h3, 3, 1, DUMMY_AUTHOR.0);

        //test
        assert_ok!(txpool.insert(tx1, false).await);
        assert_eq!(txpool.insert(tx2, false).await?[0].hash(), h1);
        assert_eq!(
            txpool.insert(tx3, false).await.unwrap_err().to_string(),
            Error::NotReplacedIncreaseGas.to_string()
        );

        let sorted = txpool
            .sorted_vec()
            .into_iter()
            .map(|t| t.hash())
            .collect::<Vec<_>>();
        assert_eq!(sorted, vec![h2]);

        Ok(())
    }

    #[tokio::test]
    async fn check_max_tx_per_account() -> Result<()> {
        let config = Arc::new(Config {
            max: 100,
            per_account: 2,
        });
        let mut txpool = Transactions::new(config, WorldStateTest::new_dummy());
        let (h1, h2, h3) = hashes();

        let tx1 = new_tx(h1, 2, 1, DUMMY_AUTHOR.0);
        let tx2 = new_tx(h2, 4, 2, DUMMY_AUTHOR.0);
        let tx3 = new_tx(h3, 3, 3, DUMMY_AUTHOR.0);

        assert!(txpool.insert(tx1, false).await.is_ok());
        assert!(txpool.insert(tx2, false).await.is_ok());
        assert_eq!(
            txpool.insert(tx3, false).await.unwrap_err().to_string(),
            Error::NotInsertedTxPerAccountFull.to_string()
        );

        Ok(())
    }

    #[tokio::test]
    async fn check_max_tx_count_not_enought_gas() -> Result<()> {
        let config = Arc::new(Config {
            max: 2,
            per_account: 4,
        });
        let mut txpool = Transactions::new(config, WorldStateTest::new_dummy());
        let (h1, h2, h3) = hashes();
        // max 22 score per account
        let tx1 = new_tx(h1, 4, 1, DUMMY_AUTHOR.0);
        let tx2 = new_tx(h2, 4, 2, DUMMY_AUTHOR.0);
        let tx3 = new_tx(h3, 1, 3, DUMMY_AUTHOR.0);

        assert!(txpool.insert(tx1, false).await.is_ok());
        assert!(txpool.insert(tx2, false).await.is_ok());
        assert_eq!(
            txpool.insert(tx3, false).await.unwrap_err().to_string(),
            Error::NotInsertedPoolFullIncreaseGas.to_string()
        );

        Ok(())
    }

    #[tokio::test]
    async fn check_max_tx_count_enought_gas() -> Result<()> {
        let config = Arc::new(Config {
            max: 10,
            per_account: 4,
        });
        let mut txpool = Transactions::new(config, WorldStateTest::new_dummy());
        let (h1, h2, h3) = hashes();

        let tx1 = new_tx(h1, 2, 1, DUMMY_AUTHOR.0);
        let tx2 = new_tx(h2, 4, 2, DUMMY_AUTHOR.0);
        let tx3 = new_tx(h3, 16, 3, DUMMY_AUTHOR1.0);

        assert!(txpool.insert(tx1, false).await.is_ok());
        assert!(txpool.insert(tx2, false).await.is_ok());
        assert_eq!(
            txpool.insert(tx3, false).await.unwrap_err().to_string(),
            Error::NotInsertedBalanceInsufficient.to_string()
        );

        Ok(())
    }

    #[tokio::test]
    async fn remove_stalled() {
        let config = Arc::new(Config {
            max: 20,
            per_account: 10,
        });
        let mut txpool = Transactions::new(config, WorldStateTest::new_dummy());
        let (h1, h2, _) = hashes();
        let tx1 = new_tx(h1, 5, 1, DUMMY_AUTHOR.0);
        let tx2 = new_tx(h2, 10, 2, DUMMY_AUTHOR.0);

        assert!(txpool.insert(tx1, false).await.is_ok());
        tokio::time::sleep(Duration::from_millis(500)).await;
        assert!(txpool.insert(tx2, false).await.is_ok());
        assert_eq!(txpool.remove_stalled(Duration::from_millis(250)), 1);
        assert_eq!(*txpool.iter_unordered().next().unwrap().0, h2);
    }

    #[tokio::test]
    async fn decline_unfunded_single_tx() {
        let config = Arc::new(Config {
            max: 20,
            per_account: 10,
        });
        let mut txpool = Transactions::new(config, WorldStateTest::new_dummy());
        let (h1, _, _) = hashes();
        let tx1 = new_tx(h1, 16, 1, DUMMY_AUTHOR.0);

        assert_eq!(
            txpool.insert(tx1, false).await.unwrap_err().to_string(),
            Error::NotInsertedBalanceInsufficient.to_string()
        );
    }

    #[tokio::test]
    async fn decline_unfunded_second_tx() {
        let config = Arc::new(Config {
            max: 20,
            per_account: 10,
        });
        let mut txpool = Transactions::new(config, WorldStateTest::new_dummy());
        let (h1, h2, _) = hashes();
        let tx1 = new_tx(h1, 15, 1, DUMMY_AUTHOR.0);
        let tx2 = new_tx(h2, 15, 2, DUMMY_AUTHOR.0);

        assert_ok!(txpool.insert(tx1, false).await);
        assert_eq!(
            txpool.insert(tx2, false).await.unwrap_err().to_string(),
            Error::NotInsertedBalanceInsufficient.to_string()
        );
    }
}
