// Copyright 2021 Gnosis Ltd.
// SPDX-License-Identifier: Apache-2.0

use interfaces::world_state::{AccountInfo, BlockUpdate, WorldState};
use reth_core::{transaction::TypePayload, Address, BlockId, Transaction, H256};
use std::{
    collections::{
        hash_map::{Entry, Iter},
        BinaryHeap, HashMap, HashSet,
    },
    mem,
    sync::Arc,
    time::{Duration, Instant},
};
use log::warn;

use anyhow::Result;

use crate::{Error, Priority, ScoreTransaction, BUMP_SCORE_BY_12_5_PERC};
pub enum Find {
    LastAccountTx(Address),
    BestAccountTx(Address),
    TxByHash(H256),
}

pub struct Account {
    info: AccountInfo,
    transactions: Vec<Arc<ScoreTransaction>>,
}

impl Account {
    pub fn new(info: AccountInfo) -> Account {
        Account {
            info,
            transactions: Vec::new(),
        }
    }

    pub fn tx(&self) -> &[Arc<ScoreTransaction>] {
        &self.transactions
    }
    pub fn tx_mut(&mut self) -> &mut Vec<Arc<ScoreTransaction>> {
        &mut self.transactions
    }
}

pub struct Transactions {
    /// transaction by account sorted in increasing order
    by_account: HashMap<Address, Account>,
    /// all transaction by its hash
    by_hash: HashMap<H256, Arc<ScoreTransaction>>,
    /// all transactions sorted by min/max value
    by_score: BinaryHeap<Arc<ScoreTransaction>>,
    /// pending hashes for removal. It is optimization for BinaryHeap because we dont want to recreate it every time.
    pending_removal: HashSet<H256>,

    config_max_tx_per_account: usize,
    config_max_tx_count: usize,

    /// needed to get nonce/balance from db
    world_state: Arc<dyn WorldState>,
}

pub struct IterPool<'txlife> {
    pub tx: &'txlife [Arc<Transaction>],
}

pub struct IterPoolOrdered {
    pub tx: Vec<Arc<Transaction>>,
}

impl Transactions {
    pub fn new(
        max_tx_per_account: usize,
        max_tx_count: usize,
        world_state: Arc<dyn WorldState>,
    ) -> Self {
        Self {
            by_account: HashMap::new(),
            by_hash: HashMap::new(),
            by_score: BinaryHeap::new(),
            pending_removal: HashSet::new(),
            config_max_tx_per_account: max_tx_per_account,
            config_max_tx_count: max_tx_count,
            world_state,
        }
    }

    fn new_scored_transaction(tx: Transaction, priority: Priority) -> Arc<ScoreTransaction> {
        let score = match tx.type_payload {
            TypePayload::AccessList(ref al) => al.legacy_payload.gas_price,
            TypePayload::Legacy(ref legacy) => legacy.gas_price,
        };
        let score = match priority {
            Priority::Local => score << 15,
            Priority::Regular => score << 10,
            Priority::Retracted => score,
        };
        Arc::new(ScoreTransaction {
            hash: tx.hash(),
            transaction: tx,
            priority,
            score,
            timestamp: Instant::now(),
        })
    }

    /// find one particular transaction
    pub fn find(&self, cond: Find) -> Option<Arc<ScoreTransaction>> {
        match cond {
            Find::LastAccountTx(address) => self
                .by_account
                .get(&address)
                .map(|account| account.transactions.last().cloned())
                .flatten(),
            Find::BestAccountTx(address) => self
                .by_account
                .get(&address)
                .map(|account| account.transactions.first().cloned())
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
        tx: Transaction,
        is_local: bool,
    ) -> Result<Vec<Arc<ScoreTransaction>>> {
        // check if we already have tx
        if self.by_hash.contains_key(&tx.hash()) {
            return Err(Error::AlreadyPresent.into());
        }

        // check if transaction is signed and author is present
        let author = tx.author().ok_or(Error::TxAuthorUnknown)?.0;

        // create scored transaction
        let scoredtx = Self::new_scored_transaction(
            tx,
            if is_local {
                Priority::Local
            } else {
                Priority::Regular
            },
        );

        // search tx in account transaction sorted by nonce
        let acc = match self.by_account.entry(author) {
            Entry::Occupied(entry) => entry.into_mut(),
            Entry::Vacant(entry) => {
                // if account info is not present, get data from world_state database.
                let info = self
                    .world_state
                    .account_info(BlockId::Latest, &author)
                    .await
                    .ok_or(Error::NotInsertedAccountUnknown)?;
                // Check nonces
                if info.nonce > scoredtx.nonce.as_u64() {
                    return Err(Error::NotInsertedWrongNonce.into());
                }
                entry.insert(Account::new(info))
            }
        };

        //find place where to insert new tx.
        let acctx = &mut acc.transactions;
        let index =
            acctx.binary_search_by(|old| old.transaction.nonce.cmp(&scoredtx.transaction.nonce));

        let insert_index = match index {
            Ok(id) => id,
            Err(id) => id,
        };
        // check if we have enought gas for new tx
        let balance = acctx[0..insert_index]
            .iter()
            .fold(acc.info.balance, |acum, tx| acum.saturating_sub(tx.cost()));
        if scoredtx.cost() > balance {
            return Err(Error::NotInsertedBalanceInsufficient.into());
        }
        // placeholder for removed transaction
        let mut replaced = None;

        let maxed_tx_per_account = acctx.len() == self.config_max_tx_per_account;
        match index {
            // if nonce is greater then all present, just try to insert it to end.
            Err(index) if index == acctx.len() => {
                // if transaction by account list is full, insert it only if it is local tx.
                if maxed_tx_per_account && !is_local {
                    return Err(Error::NotInsertedTxPerAccountFull.into());
                } else {
                    acctx.push(scoredtx.clone());
                }
            }
            // if insertion is in middle (or beggining)
            Err(index) => {
                acctx.insert(index, scoredtx.clone());
                //if there is max items, remove last one with lowest nonce
                if maxed_tx_per_account {
                    //TODO check if it is local tx
                    replaced = acctx.pop();
                }
            }
            // if there is tx match with same nonce and if new tx score is 12,5% greater then old score, replace it
            Ok(index) => {
                let old_score = acctx[index].score;
                let bumped_old_score =
                    old_score.saturating_add(old_score >> BUMP_SCORE_BY_12_5_PERC);
                if scoredtx.score > bumped_old_score {
                    acctx.push(scoredtx.clone());
                    replaced = Some(acctx.swap_remove(index)); //swap_remove: The removed element is replaced by the last element of the vector.
                } else {
                    return Err(Error::NotReplacedIncreaseGas.into());
                }
            }
        }
        let mut to_rem = vec![];
        // if there is replaced tx remove it here.
        if let Some(ref rem) = replaced {
            self.by_hash.remove(&rem.hash);
            self.pending_removal.insert(rem.hash);
        }

        // remove it from removed list if present. (This is edge case, should happen only very rarely).
        self.pending_removal.remove(&scoredtx.hash());

        // insert to other structures
        self.by_score.push(scoredtx.clone());
        self.by_hash.insert(scoredtx.hash(), scoredtx.clone());

        // now, with sorted tx inside our by_account struct
        // We can calculate cost and check if we disrupted cost for tx with greater nonce.
        // if there is not enought gas, remove transactions.
        let mut left_balance = balance;
        for tx in acctx[insert_index..].iter() {
            let cost = tx.cost();
            if cost > left_balance {
                to_rem.push(tx.clone());
            }
            left_balance = left_balance.saturating_sub(cost);
        }

        for tx in to_rem.iter() {
            self.remove(&tx.hash());
        }

        if let Some(ref rep) = replaced {
            to_rem.push(rep.clone());
        }

        // remove transaction if we hit limit
        if self.by_hash.len() > self.config_max_tx_count {
            // we dont check if it is local or not, because score for Local should be a lot higher.
            // and max_tx_count should be hard limit.
            let worst_scoredtx = self.by_score.peek().unwrap().hash;
            let removed = self.remove(&worst_scoredtx);
            if worst_scoredtx == scoredtx.hash {
                //least scored tx is same as inserted tx. Should not happen.
                return Err(Error::NotInsertedPoolFullIncreaseGas.into());
            } else {
                if let Some(rem) = removed {
                    to_rem.push(rem);
                }
            }
        }

        Ok(to_rem)
    }

    pub fn remove(&mut self, txhash: &H256) -> Option<Arc<ScoreTransaction>> {
        // remove tx from by_hash
        if let Some(tx) = self.by_hash.remove(txhash) {
            // remove tx from by_accounts
            let author = tx.transaction.author().unwrap().0;
            let rem_account = {
                let vec = &mut self
                    .by_account
                    .get_mut(&author)
                    .expect("Expect to account to contain specific tx")
                    .transactions;
                vec.remove(
                    vec.iter()
                        .position(|item| item.hash() == tx.hash())
                        .expect("expect to found tx in by_account struct"),
                );
                vec.is_empty()
            };
            if rem_account {
                self.by_account.remove(&author);
            }
            // mark tx for removal from by_score
            self.pending_removal.insert(tx.hash);
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
            .filter(|tx| !self.pending_removal.contains(&tx.hash))
            .collect();

        self.pending_removal.clear();
        self.by_score = BinaryHeap::from(fresh_tx);
    }

    /// remove all stalled transactions
    /// Iterates over all transactions and remove all that is inserted 600s ago.
    fn _remove_stalled(&mut self, threshold: Duration) -> usize {
        let removal_threshold = Instant::now() - threshold;

        let remove: Vec<H256> = self
            .by_hash
            .iter()
            .filter(|(_, tx)| tx.timestamp < removal_threshold)
            .map(|(hash, _)| hash.clone())
            .collect();
        let removed_count = remove.len();
        for hash in remove {
            self.remove(&hash).unwrap();
        }
        self.recreate_heap();
        removed_count
    }

    /// Iterates over all transactions in pool.
    pub fn iter_unordered(&self) -> Iter<'_, H256, Arc<ScoreTransaction>> {
        self.by_hash.iter()
    }

    /// Return all transactions in pool sorted from best to worst score.
    /// This is expensive opperation and it copies all transaction.
    pub fn sorted_vec(&mut self) -> Vec<Arc<ScoreTransaction>> {
        self.recreate_heap();
        self.by_score.clone().into_sorted_vec()
    }

    pub async fn block_update(&mut self, update: &BlockUpdate) {
        // reverted nonces
        for (address, info) in update.reverted_accounts.iter() {
            if let Some(account) = self.by_account.get_mut(&address) {
                // we are okay to just replace it. Tx in list allready had enought balance and revert.
                account.info = *info;
            }
        }

        // applied nonces. We need to check nonce and gas of all transaction in list and remove ones with insufficient gas or nonce.
        for (address, info) in update.appliend_accounts.iter() {
            let mut rem_old_nonce = Vec::new();
            let mut rem_balance = Vec::new();
            if let Some(account) = self.by_account.get_mut(&address) {
                account.info = *info;
                // remove tx with lower nonce
                account
                    .tx()
                    .iter()
                    .take_while(|tx| tx.nonce.as_u64() <= info.nonce)
                    .for_each(|tx| rem_old_nonce.push(tx.hash));
                //remove all hashes
                let mut balance = info.balance;
                for tx in account.tx().iter().skip(rem_old_nonce.len()) {
                    let cost = tx.cost();
                    if cost > balance {
                        rem_balance.push(tx.hash);
                    }
                    balance = balance.saturating_sub(cost);
                }
                // change to new balance and nonce. We are okay to set it here, because remove is donny and does not chage account info.
                account.info = *info;
            } else {

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
                    if self.insert(tx, true).await.is_err() {
                        warn!("We got bad transaction in reverted block:{:?}",rawtx);
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
    use reth_core::transaction::{
        transaction::{fake_sign, DUMMY_AUTHOR},
        LegacyPayload,
    };
    use tokio_test::assert_ok;

    fn new_tx(hash: H256, score: usize, nonce: usize, author: Address) -> Transaction {
        let mut tx = Transaction::default();
        tx.gas_limit = (score * 4).into();
        tx.type_payload = TypePayload::Legacy(LegacyPayload {
            gas_price: score.into(),
        });
        tx = fake_sign(tx, author);
        tx.set_hash(hash);
        tx.nonce = nonce.into();
        tx
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
        let mut txpool = Transactions::new(10, 100, WorldStateTest::new_dummy());
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
            .map(|t: Arc<ScoreTransaction>| t.hash)
            .collect::<Vec<_>>();
        assert_eq!(sorted, vec![h2, h3, h1]);

        Ok(())
    }

    #[tokio::test]
    async fn insert_remove() -> Result<()> {
        //data
        let mut txpool = Transactions::new(10, 100, WorldStateTest::new_dummy());
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
            .map(|t: Arc<ScoreTransaction>| t.hash)
            .collect::<Vec<_>>();
        assert_eq!(sorted, vec![h3, h1]);
        Ok(())
    }

    #[tokio::test]
    async fn should_replace_nonce() -> Result<()> {
        //data
        let mut txpool = Transactions::new(10, 100, WorldStateTest::new_dummy());
        let (h1, h2, h3) = hashes();

        let tx1 = new_tx(h1, 2, 1, DUMMY_AUTHOR.0);
        let tx2 = new_tx(h2, 4, 1, DUMMY_AUTHOR.0);
        let tx3 = new_tx(h3, 3, 1, DUMMY_AUTHOR.0);

        //test
        assert_ok!(txpool.insert(tx1, false).await);
        assert_eq!(txpool.insert(tx2, false).await?[0].hash, h1);
        assert_eq!(
            txpool.insert(tx3, false).await.unwrap_err().to_string(),
            Error::NotReplacedIncreaseGas.to_string()
        );

        let sorted = txpool
            .sorted_vec()
            .into_iter()
            .map(|t: Arc<ScoreTransaction>| t.hash)
            .collect::<Vec<_>>();
        assert_eq!(sorted, vec![h2]);

        Ok(())
    }

    #[tokio::test]
    async fn check_max_tx_per_account() -> Result<()> {
        let mut txpool = Transactions::new(2, 100, WorldStateTest::new_dummy());
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
        let mut txpool = Transactions::new(10, 2, WorldStateTest::new_dummy());
        let (h1, h2, h3) = hashes();

        let tx1 = new_tx(h1, 2, 1, DUMMY_AUTHOR.0);
        let tx2 = new_tx(h2, 4, 2, DUMMY_AUTHOR.0);
        let tx3 = new_tx(h3, 3, 3, DUMMY_AUTHOR.0);

        assert!(txpool.insert(tx3, false).await.is_ok());
        assert!(txpool.insert(tx2, false).await.is_ok());
        assert_eq!(
            txpool.insert(tx1, false).await.unwrap_err().to_string(),
            Error::NotInsertedPoolFullIncreaseGas.to_string()
        );

        Ok(())
    }

    #[tokio::test]
    async fn check_max_tx_count_enought_gas() -> Result<()> {
        let mut txpool = Transactions::new(10, 2, WorldStateTest::new_dummy());
        let (h1, h2, h3) = hashes();

        let tx1 = new_tx(h1, 2, 1, DUMMY_AUTHOR.0);
        let tx2 = new_tx(h2, 4, 2, DUMMY_AUTHOR.0);
        let tx3 = new_tx(h3, 3, 3, DUMMY_AUTHOR.0);

        assert!(txpool.insert(tx1, false).await.is_ok());
        assert!(txpool.insert(tx2, false).await.is_ok());
        assert_eq!(txpool.insert(tx3, false).await.unwrap()[0].hash(), h1);

        Ok(())
    }

    #[tokio::test]
    async fn remove_stalled() {
        let mut txpool = Transactions::new(10, 20, WorldStateTest::new_dummy());
        let (h1, h2, _) = hashes();
        let tx1 = new_tx(h1, 5, 1, DUMMY_AUTHOR.0);
        let tx2 = new_tx(h2, 10, 2, DUMMY_AUTHOR.0);

        assert!(txpool.insert(tx1, false).await.is_ok());
        std::thread::sleep(Duration::from_millis(500));
        assert!(txpool.insert(tx2, false).await.is_ok());
        assert_eq!(txpool._remove_stalled(Duration::from_millis(250)), 1);
        assert_eq!(*txpool.iter_unordered().next().unwrap().0, h2);
    }

    #[tokio::test]
    async fn decline_unfunded_single_tx() {
        let mut txpool = Transactions::new(10, 20, WorldStateTest::new_dummy());
        let (h1, _, _) = hashes();
        let tx1 = new_tx(h1, 16, 1, DUMMY_AUTHOR.0);

        assert_eq!(
            txpool.insert(tx1, false).await.unwrap_err().to_string(),
            Error::NotInsertedBalanceInsufficient.to_string()
        );
    }

    #[tokio::test]
    async fn decline_unfunded_second_tx() {
        let mut txpool = Transactions::new(10, 20, WorldStateTest::new_dummy());
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
