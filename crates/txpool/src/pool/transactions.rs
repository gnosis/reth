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

use super::{account::Account, score::ByScore, ScoreTransaction};
use crate::{Config, Error, MAX_PENDING_TX_REMOVALS};
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
    /// all transaction by its hash and timeout.
    by_hash: HashMap<H256, (Arc<Transaction>, Instant)>,
    /// Sorted by score.
    by_score: ByScore,
    /// currently known block hash and base_fee
    block: BlockInfo,
    /// configuration
    config: Arc<Config>,
}

impl Transactions {
    pub fn new(config: Arc<Config>, block: BlockInfo) -> Self {
        Self {
            by_account: HashMap::new(),
            by_hash: HashMap::new(),
            by_score: ByScore::new(),
            config,
            block,
        }
    }

    #[inline]
    pub fn find_by_hash(&self, hash: &H256) -> Option<Arc<Transaction>> {
        let (tx, _) = self.by_hash.get(hash)?;
        Some(tx.clone())
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
            Find::TxByHash(hash) => self.find_by_hash(&hash),
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
        account: Option<(AccountInfo, H256)>, // account and block hash of block
    ) -> Result<Vec<(Arc<Transaction>, Error)>> {
        // check if we already have tx
        if self.by_hash.contains_key(&tx.hash()) {
            return Err(Error::AlreadyPresent.into());
        }

        // check if transaction is signed and author is present
        let author = tx.author().ok_or(Error::TxAuthorUnknown)?.0;
        // create scored transaction
        let scoredtx = ScoreTransaction::new(tx, &self.block.base_fee);

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

        let (replaced, unfunded) = acc.insert(&scoredtx, self.config.per_account)?;

        // if there is replaced tx remove it here from rest of structures.
        if let Some(ref rem) = replaced {
            let hash = rem.hash();
            self.by_hash.remove(&hash);
            self.by_score.remove(hash)
        }

        // remove it from removed list if present. (This is edge case, should happen only very rarely).
        self.by_score.pending_removal_remove(&scoredtx.hash());

        // insert to other structures
        self.by_score.push(scoredtx.clone());
        self.by_hash
            .insert(scoredtx.hash(), (scoredtx.tx.clone(), Instant::now()));

        // output
        let mut removed = Vec::new();

        for tx in unfunded.iter() {
            self.remove(&tx.hash());
            removed.push((tx.clone(), Error::RemovedTxUnfunded));
        }

        // add replaced tx to list of removed tx.
        if let Some(ref rep) = replaced {
            removed.push((rep.clone(), Error::RemovedTxReplaced));
        }

        // remove transaction if we hit limit
        if self.by_hash.len() > self.config.max {
            // and max_tx_count should be hard limit.
            let worst_scoredtx = self.by_score.peek().unwrap().tx.hash();
            let rem = self.remove(&worst_scoredtx);
            if let Some(rem) = rem {
                removed.push((rem.clone(), Error::RemovedTxLimitHit));
            }
        }

        Ok(removed)
    }

    pub fn remove(&mut self, txhash: &H256) -> Option<Arc<Transaction>> {
        // remove tx from by_hash
        if let Some((tx, _)) = self.by_hash.remove(txhash) {
            // TODO discussion. Check if we want to remove this tx if there are tx from same account but greater nonce?
            // we will be making gaps

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
            self.by_score.remove(*txhash);
            Some(tx.clone())
        } else {
            None
        }
    }

    /// periodically remove stalled transactions and recreate heap if needed.
    pub fn periodic_check(&mut self) -> Vec<Arc<Transaction>> {
        let rem = self.remove_stalled(self.config.timeout);

        if self.by_score.pending_removal() > MAX_PENDING_TX_REMOVALS {
            self.by_score.recreate_heap(None);
        }
        rem
    }

    /// remove all stalled transactions
    /// Iterates over all transactions and remove all that is inserted 600s ago.
    fn remove_stalled(&mut self, threshold: Duration) -> Vec<Arc<Transaction>> {
        let removal_threshold = Instant::now() - threshold;

        let remove: Vec<H256> = self
            .by_hash
            .iter()
            .filter(|(_, (_, timestamp))| *timestamp < removal_threshold)
            .map(|(hash, _)| hash.clone())
            .collect();
        let mut rem_tx = Vec::new();
        for hash in remove {
            rem_tx.push(self.remove(&hash).unwrap());
        }
        rem_tx
    }

    /// Iterates over all transactions in pool.
    pub fn iter_unordered(&self) -> Iter<'_, H256, (Arc<Transaction>, Instant)> {
        self.by_hash.iter()
    }

    /// Return all transactions in pool sorted from best to worst score.
    /// additionally return account info.
    /// This is expensive opperation, use it only when needed.
    pub fn binary_heap_and_accounts(
        &mut self,
    ) -> (
        BinaryHeap<ScoreTransaction>,
        HashMap<Address, AccountInfo>,
        H256,
    ) {
        self.by_score.recreate_heap(None);
        let binary_heap = self.by_score.clone_heap();

        let mut infos = HashMap::with_capacity(self.by_account.len());

        for (address, acc) in self.by_account.iter() {
            infos.insert(*address, acc.info().clone());
        }

        (binary_heap, infos, self.block.hash)
    }

    /// update block with new account state and reinsert transaction from reverted block
    /// return pair of (removed tx, reinserted tx)
    pub fn block_update(
        &mut self,
        update: &BlockUpdate,
    ) -> (Vec<(Arc<Transaction>, Error)>, Vec<Arc<Transaction>>) {
        let mut removed = Vec::new();
        if self.block.hash != update.old_hash {
            error!("We are incosistent with world_state. txpool should be restart");
        }
        self.block.hash = update.new_hash;
        self.block.base_fee = update.base_fee;
        // iterate on changes on accounts
        for (address, info) in update.changed_accounts.iter() {
            // placeholder for removed tx;
            let mut rem_tx = Vec::new();
            if let Some(account) = self.by_account.get_mut(&address) {
                // If new nonce is greater that our current, remove all tx with obsolete nonce.
                if info.nonce > account.info.nonce {
                    account
                        .txs()
                        .iter()
                        .take_while(|tx| tx.nonce.as_u64() <= info.nonce)
                        .for_each(|tx| {
                            rem_tx.push(tx.hash());
                            removed.push((tx.clone(), Error::OnNewBlockNonce));
                        });
                }
                account.info.nonce = info.nonce;

                // if new balance is decreased, check if we have unfunded tx.
                if info.balance < account.info.balance {
                    let mut balance = info.balance;
                    // iterate over rest of tx, skipping removed ones.
                    for tx in account.txs().iter().skip(rem_tx.len()) {
                        let cost = tx.max_cost();
                        if cost > balance {
                            rem_tx.push(tx.hash());
                            removed.push((tx.clone(), Error::RemovedTxUnfunded));
                        }
                        balance = balance.saturating_sub(cost);
                    }
                }
                account.info.balance = info.balance;
            }
            for remove in rem_tx {
                self.remove(&remove);
            }
        }

        // reinsert tx that are not included in new block.
        // account info for reverted tx can be found in reverted_accounts or applied_accounts.

        // All aggregated:
        let accounts = update
            .changed_accounts
            .iter()
            .cloned()
            .collect::<HashMap<Address, AccountInfo>>();
        let mut reinserted = Vec::new();
        for rawtx in update.reverted_tx.iter() {
            if let Ok(mut tx) = Transaction::decode(rawtx) {
                if let Ok((address, _)) = tx.recover_author() {
                    if let Some(&account) = accounts.get(&address) {
                        let tx = Arc::new(tx);
                        match self.insert(tx.clone(), Some((account, update.new_hash))) {
                            Err(err) => {
                                warn!(
                                "We got error while reinserting tx[{:?}] from reverted block:{:?}",
                                rawtx,
                                err.to_string()
                            );
                            }
                            Ok(_) => {
                                reinserted.push(tx);
                            }
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

        // update effective_gas_price for all transacitions and let us recreate binary heap
        self.by_score.recreate_heap(Some(self.block.base_fee));
        (removed, reinserted)
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
        tx.gas_price = score.into();
        tx.type_payload = TypePayload::Legacy(LegacyPayload {});
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
            ..Default::default()
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
        txpool.insert(tx1, info)?;
        txpool.insert(tx2, info)?;
        txpool.insert(tx3, info)?;

        //check
        assert_eq!(txpool.iter_unordered().len(), 3);

        //check ordered
        let sorted = txpool
            .binary_heap_and_accounts()
            .0
            .into_sorted_vec()
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
            ..Default::default()
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
        txpool.insert(tx1, info)?;
        txpool.insert(tx2, info)?;
        txpool.insert(tx3, info)?;
        txpool.remove(&h2);

        //check
        assert_eq!(txpool.iter_unordered().len(), 2);

        let sorted = txpool
            .binary_heap_and_accounts()
            .0
            .into_sorted_vec()
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
            ..Default::default()
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
        assert_ok!(txpool.insert(tx1, info));
        assert_eq!(txpool.insert(tx2, info)?[0].0.hash(), h1);
        assert_eq!(
            txpool.insert(tx3, info).unwrap_err().to_string(),
            Error::NotReplacedIncreaseGas.to_string()
        );

        let sorted = txpool
            .binary_heap_and_accounts()
            .0
            .into_sorted_vec()
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
            ..Default::default()
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

        assert!(txpool.insert(tx1, info).is_ok());
        assert!(txpool.insert(tx2, info).is_ok());
        assert_eq!(
            txpool.insert(tx3, info).unwrap_err().to_string(),
            Error::NotInsertedTxPerAccountFull.to_string()
        );

        Ok(())
    }

    #[tokio::test]
    async fn check_max_tx_count_not_enought_gas() -> Result<()> {
        let config = Arc::new(Config {
            max: 2,
            per_account: 4,
            ..Default::default()
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

        assert!(txpool.insert(tx1, info).is_ok());
        assert!(txpool.insert(tx2, info).is_ok());
        assert_eq!(
            txpool.insert(tx3, info).unwrap_err().to_string(),
            Error::NotInsertedPoolFullIncreaseGas.to_string()
        );

        Ok(())
    }

    #[tokio::test]
    async fn check_max_tx_count_enought_gas() -> Result<()> {
        let config = Arc::new(Config {
            max: 10,
            per_account: 4,
            ..Default::default()
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

        assert!(txpool.insert(tx1, info).is_ok());
        assert!(txpool.insert(tx2, info).is_ok());
        assert_eq!(
            txpool.insert(tx3, info).unwrap_err().to_string(),
            Error::NotInsertedBalanceInsufficient.to_string()
        );

        Ok(())
    }

    #[tokio::test]
    async fn remove_stalled() {
        let config = Arc::new(Config {
            max: 20,
            per_account: 10,
            ..Default::default()
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

        assert!(txpool.insert(tx1, info).is_ok());
        tokio::time::sleep(Duration::from_millis(500)).await;
        assert!(txpool.insert(tx2, info).is_ok());
        assert_eq!(txpool.remove_stalled(Duration::from_millis(250)).len(), 1);
        assert_eq!(*txpool.iter_unordered().next().unwrap().0, h2);
    }

    #[tokio::test]
    async fn decline_unfunded_single_tx() {
        let config = Arc::new(Config {
            max: 20,
            per_account: 10,
            ..Default::default()
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
            txpool.insert(tx1, info).unwrap_err().to_string(),
            Error::NotInsertedBalanceInsufficient.to_string()
        );
    }

    #[tokio::test]
    async fn decline_unfunded_second_tx() {
        let config = Arc::new(Config {
            max: 20,
            per_account: 10,
            ..Default::default()
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

        assert_ok!(txpool.insert(tx1, info));
        assert_eq!(
            txpool.insert(tx2, info).unwrap_err().to_string(),
            Error::NotInsertedBalanceInsufficient.to_string()
        );
    }
}
