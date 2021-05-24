// Copyright 2021 Gnosis Ltd.
// SPDX-License-Identifier: Apache-2.0

use interfaces::world_state::{AccountInfo, BlockUpdate};
use log::*;
use reth_core::{Address, Transaction, H256, U256};
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

#[derive(Copy, Clone, Default)]
pub struct BlockInfo {
    pub base_fee: U256,
    pub hash: H256,
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

    block: BlockInfo,
}

impl Transactions {
    pub fn new(config: Arc<Config>, block: BlockInfo) -> Self {
        Self {
            by_account: HashMap::new(),
            by_hash: HashMap::new(),
            by_score: BinaryHeap::new(),
            pending_removal: HashSet::new(),
            config,
            block,
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

    pub fn block(&self) -> &BlockInfo {
        &self.block
    }

    /// Return account if present and current best known block hash.
    pub fn account(&self, address: &Address) -> Option<&AccountInfo> {
        self.by_account.get(address).map(|acc| acc.info())
    }

    /// insert transaction into pool
    /// Return Ok with list of removed transaction.
    ///
    /// Reasons why transaction can be replaced by this tx:
    /// 1. Tx with same nonce but 12.5% less gas.
    /// 2. Account does not have enought balance for transactions with higher nonce
    /// 3. Limit on Max transaction.
    ///
    /// account is here to allow us to prefetch account info from world_state without blocking this function.
    /// we are comparing given block hash and our current block hash to check if we have latest and greated account info.
    /// if not, we are returning error and expect caller to fetch account info again from world_state.
    pub fn insert(
        &mut self,
        tx: Arc<Transaction>,
        is_local: bool,
        account: Option<(AccountInfo, H256)>, // account and block hash of block
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
                if let Some(info) = account {
                    if info.1 != self.block.hash {
                        return Err(Error::InternalAccountObsolete.into());
                    }
                    if scoredtx.nonce.as_u64() <= info.0.nonce {
                        return Err(Error::NotInsertedWrongNonce.into());
                    }
                    entry.insert(Account::new(info.0))
                } else {
                    return Err(Error::InternalAccountNotFound.into());
                }
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

    fn by_score_remove(&mut self, hash: H256) {
        if self.by_score.peek().unwrap().hash() == hash {
            self.by_score.pop();
            loop {
                // pop last tx if there are pending for removal
                if let Some(tx) = self.by_score.peek().cloned() {
                    if self.pending_removal.contains(&tx.hash()) {
                        self.by_score.pop();
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
    /// additionally return account info.
    /// This is expensive opperation, use it only when needed.
    pub fn sorted_vec_and_accounts(
        &mut self,
    ) -> (Vec<ScoreTransaction>, HashMap<Address, AccountInfo>, H256) {
        self.recreate_heap();
        let sorted = self.by_score.clone().into_sorted_vec();

        let mut infos = HashMap::with_capacity(self.by_account.len());

        for (address, acc) in self.by_account.iter() {
            infos.insert(*address, acc.info().clone());
        }

        (sorted, infos, self.block.hash)
    }

    pub fn block_update(&mut self, update: &BlockUpdate) {
        // reverted nonces
        for (address, info) in update.reverted_accounts.iter() {
            if let Some(account) = self.by_account.get_mut(&address) {
                // we are okay to just replace it. Tx in list allready had enought balance.
                account.set_info(*info);
            }
        }

        // applied nonces. We need to check nonce and gas of all transaction in list and remove ones with insufficient nonce or gas.
        for (address, info) in update.applied_accounts.iter() {
            let mut rem_old_nonce = Vec::new();
            let mut rem_balance = Vec::new();
            if let Some(account) = self.by_account.get_mut(&address) {
                // change to new balance and nonce. We are okay to set it here, because we dont use it.
                account.set_info(*info);
                // remove tx with lower nonce
                account
                    .txs()
                    .iter()
                    .take_while(|tx| tx.nonce.as_u64() <= info.nonce)
                    .for_each(|tx| rem_old_nonce.push(tx.hash()));
                // remove tx with not enought balance
                let mut balance = info.balance;
                for tx in account.txs().iter().skip(rem_old_nonce.len()) {
                    let cost = tx.cost();
                    if cost > balance {
                        rem_balance.push(tx.hash());
                    }
                    balance = balance.saturating_sub(cost);
                }
            }

            rem_old_nonce.iter().for_each(|hash| {
                self.remove(hash);
            });
            rem_balance.iter().for_each(|hash| {
                self.remove(hash);
            });
        }

        self.block.hash = update.new_hash;

        // reinsert tx that are not included in new block.
        // account info for reverted tx can be found in reverted_accounts or applied_accounts.

        // All aggregated:
        let mut accounts = update
            .applied_accounts
            .iter()
            .cloned()
            .collect::<HashMap<Address, AccountInfo>>();
        accounts.extend(update.reverted_accounts.iter().cloned());

        for rawtx in update.reverted_tx.iter() {
            if let Ok(mut tx) = Transaction::decode(rawtx) {
                if let Ok((address, _)) = tx.recover_author() {
                    if let Some(&account) = accounts.get(&address) {
                        if let Err(err) =
                            self.insert(Arc::new(tx), true, Some((account, update.new_hash)))
                        {
                            warn!(
                                "We got error while reinserting tx[{:?}] from reverted block:{:?}",
                                rawtx,
                                err.to_string()
                            );
                        }
                    } else {
                        warn!("Missing account for reverted tx in block:{:?}", rawtx);
                    }
                } else {
                    warn!(
                        "couldn't recover author from tx in reverted block:{:?}",
                        rawtx
                    );
                }
            }
        }

        // this is big change to pool. Let us recreashe binary heap
        self.recreate_heap();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use reth_core::transaction::{
        transaction::{fake_sign, DUMMY_AUTHOR, DUMMY_AUTHOR1},
        LegacyPayload, TypePayload,
    };
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
        let info = Some((
            AccountInfo {
                nonce: 0,
                balance: 1_000.into(),
            },
            H256::zero(),
        ));
        let mut txpool = Transactions::new(config, Default::default());
        let (h1, h2, h3) = hashes();

        let tx1 = new_tx(h1, 2, 1, DUMMY_AUTHOR.0);
        let tx2 = new_tx(h2, 4, 2, DUMMY_AUTHOR.0);
        let tx3 = new_tx(h3, 3, 3, DUMMY_AUTHOR.0);

        //test
        txpool.insert(tx1, false, info)?;
        txpool.insert(tx2, false, info)?;
        txpool.insert(tx3, false, info)?;

        //check
        assert_eq!(txpool.iter_unordered().len(), 3);

        //check ordered
        let sorted = txpool
            .sorted_vec_and_accounts()
            .0
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
        let info = Some((
            AccountInfo {
                nonce: 0,
                balance: 1_000.into(),
            },
            H256::zero(),
        ));
        let mut txpool = Transactions::new(config, Default::default());
        let (h1, h2, h3) = hashes();

        let tx1 = new_tx(h1, 2, 1, DUMMY_AUTHOR.0);
        let tx2 = new_tx(h2, 4, 2, DUMMY_AUTHOR.0);
        let tx3 = new_tx(h3, 3, 3, DUMMY_AUTHOR.0);

        //test
        txpool.insert(tx1, false, info)?;
        txpool.insert(tx2, false, info)?;
        txpool.insert(tx3, false, info)?;
        txpool.remove(&h2);

        //check
        assert_eq!(txpool.iter_unordered().len(), 2);

        let sorted = txpool
            .sorted_vec_and_accounts()
            .0
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
        let info = Some((
            AccountInfo {
                nonce: 0,
                balance: 1_000.into(),
            },
            H256::zero(),
        ));
        let mut txpool = Transactions::new(config, Default::default());
        let (h1, h2, h3) = hashes();

        let tx1 = new_tx(h1, 2, 1, DUMMY_AUTHOR.0);
        let tx2 = new_tx(h2, 4, 1, DUMMY_AUTHOR.0);
        let tx3 = new_tx(h3, 3, 1, DUMMY_AUTHOR.0);

        //test
        assert_ok!(txpool.insert(tx1, false, info));
        assert_eq!(txpool.insert(tx2, false, info)?[0].hash(), h1);
        assert_eq!(
            txpool.insert(tx3, false, info).unwrap_err().to_string(),
            Error::NotReplacedIncreaseGas.to_string()
        );

        let sorted = txpool
            .sorted_vec_and_accounts()
            .0
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
        let info = Some((
            AccountInfo {
                nonce: 0,
                balance: 1_000.into(),
            },
            H256::zero(),
        ));
        let mut txpool = Transactions::new(config, Default::default());
        let (h1, h2, h3) = hashes();

        let tx1 = new_tx(h1, 2, 1, DUMMY_AUTHOR.0);
        let tx2 = new_tx(h2, 4, 2, DUMMY_AUTHOR.0);
        let tx3 = new_tx(h3, 3, 3, DUMMY_AUTHOR.0);

        assert!(txpool.insert(tx1, false, info).is_ok());
        assert!(txpool.insert(tx2, false, info).is_ok());
        assert_eq!(
            txpool.insert(tx3, false, info).unwrap_err().to_string(),
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
        let info = Some((
            AccountInfo {
                nonce: 0,
                balance: 1_000.into(),
            },
            H256::zero(),
        ));
        let mut txpool = Transactions::new(config, Default::default());
        let (h1, h2, h3) = hashes();
        // max 22 score per account
        let tx1 = new_tx(h1, 4, 1, DUMMY_AUTHOR.0);
        let tx2 = new_tx(h2, 4, 2, DUMMY_AUTHOR.0);
        let tx3 = new_tx(h3, 1, 3, DUMMY_AUTHOR.0);

        assert!(txpool.insert(tx1, false, info).is_ok());
        assert!(txpool.insert(tx2, false, info).is_ok());
        assert_eq!(
            txpool.insert(tx3, false, info).unwrap_err().to_string(),
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
        let info = Some((
            AccountInfo {
                nonce: 0,
                balance: 1_000.into(),
            },
            H256::zero(),
        ));
        let mut txpool = Transactions::new(config, Default::default());
        let (h1, h2, h3) = hashes();

        let tx1 = new_tx(h1, 2, 1, DUMMY_AUTHOR.0);
        let tx2 = new_tx(h2, 4, 2, DUMMY_AUTHOR.0);
        let tx3 = new_tx(h3, 16, 3, DUMMY_AUTHOR1.0);

        assert!(txpool.insert(tx1, false, info).is_ok());
        assert!(txpool.insert(tx2, false, info).is_ok());
        assert_eq!(
            txpool.insert(tx3, false, info).unwrap_err().to_string(),
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
        let info = Some((
            AccountInfo {
                nonce: 0,
                balance: 1_000.into(),
            },
            H256::zero(),
        ));
        let mut txpool = Transactions::new(config, Default::default());
        let (h1, h2, _) = hashes();
        let tx1 = new_tx(h1, 5, 1, DUMMY_AUTHOR.0);
        let tx2 = new_tx(h2, 10, 2, DUMMY_AUTHOR.0);

        assert!(txpool.insert(tx1, false, info).is_ok());
        tokio::time::sleep(Duration::from_millis(500)).await;
        assert!(txpool.insert(tx2, false, info).is_ok());
        assert_eq!(txpool.remove_stalled(Duration::from_millis(250)), 1);
        assert_eq!(*txpool.iter_unordered().next().unwrap().0, h2);
    }

    #[tokio::test]
    async fn decline_unfunded_single_tx() {
        let config = Arc::new(Config {
            max: 20,
            per_account: 10,
        });
        let info = Some((
            AccountInfo {
                nonce: 0,
                balance: 1_000.into(),
            },
            H256::zero(),
        ));
        let mut txpool = Transactions::new(config, Default::default());
        let (h1, _, _) = hashes();
        let tx1 = new_tx(h1, 16, 1, DUMMY_AUTHOR.0);

        assert_eq!(
            txpool.insert(tx1, false, info).unwrap_err().to_string(),
            Error::NotInsertedBalanceInsufficient.to_string()
        );
    }

    #[tokio::test]
    async fn decline_unfunded_second_tx() {
        let config = Arc::new(Config {
            max: 20,
            per_account: 10,
        });
        let info = Some((
            AccountInfo {
                nonce: 0,
                balance: 1_000.into(),
            },
            H256::zero(),
        ));
        let mut txpool = Transactions::new(config, Default::default());
        let (h1, h2, _) = hashes();
        let tx1 = new_tx(h1, 15, 1, DUMMY_AUTHOR.0);
        let tx2 = new_tx(h2, 15, 2, DUMMY_AUTHOR.0);

        assert_ok!(txpool.insert(tx1, false, info));
        assert_eq!(
            txpool.insert(tx2, false, info).unwrap_err().to_string(),
            Error::NotInsertedBalanceInsufficient.to_string()
        );
    }
}
