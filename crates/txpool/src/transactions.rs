// Copyright 2021 Gnosis Ltd.
// SPDX-License-Identifier: Apache-2.0

use core::{transaction::TypePayload, Address, Transaction, H256};
use std::{
    collections::{hash_map::Iter, HashSet},
    mem,
    sync::Arc,
    time::Instant,
};
use std::{
    collections::{BinaryHeap, HashMap},
    time::Duration,
};

use crate::{Error, Priority, ScoreTransaction, BUMP_SCORE_BY_12_5_PERC};
pub enum Find {
    LastAccountTx(Address),
    TxByHash(H256),
}

pub struct Transactions {
    /// transaction by account sorted in increasing order
    by_account: HashMap<Address, Vec<Arc<ScoreTransaction>>>,
    /// all transaction by its hash
    by_hash: HashMap<H256, Arc<ScoreTransaction>>,
    /// all transactions sorted by min/max value
    by_score: BinaryHeap<Arc<ScoreTransaction>>,
    /// pending hashes for removal. It is optimization for BinaryHeap because we dont want to recreate it every time.
    pending_removal: HashSet<H256>,

    config_max_tx_per_account: usize,
    config_max_tx_count: usize,
}

pub struct IterPool<'txlife> {
    pub tx: &'txlife [Arc<Transaction>],
}

pub struct IterPoolOrdered {
    pub tx: Vec<Arc<Transaction>>,
}

impl Transactions {
    pub fn new(max_tx_per_account: usize, max_tx_count: usize) -> Self {
        Self {
            by_account: HashMap::new(),
            by_hash: HashMap::new(),
            by_score: BinaryHeap::new(),
            pending_removal: HashSet::new(),
            config_max_tx_per_account: max_tx_per_account,
            config_max_tx_count: max_tx_count,
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
            Find::LastAccountTx(add) => self
                .by_account
                .get(&add)
                .map(|txlist| txlist.last().clone())
                .flatten()
                .cloned(),
            Find::TxByHash(hash) => self.by_hash.get(&hash).cloned(),
        }
    }

    /// insert transaction into pool
    /// Return Ok with some replaced transaction or none.  
    pub fn insert(
        &mut self,
        tx: Transaction,
        is_local: bool,
    ) -> Result<Option<Arc<ScoreTransaction>>, Error> {
        // check if we already have tx
        if self.by_hash.contains_key(&tx.hash()) {
            return Err(Error::AlreadyPresent);
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
        // placeholder for removed transaction
        let mut removed = None;

        // search tx in account transaction sorted by nonce
        let acc = self.by_account.entry(author).or_default();
        let index =
            acc.binary_search_by(|old| old.transaction.nonce.cmp(&scoredtx.transaction.nonce));

        let maxed_tx_per_account = acc.len() == self.config_max_tx_per_account;
        match index {
            // if nonce is greater then all present, just try to insert it to end.
            Err(index) if index == acc.len() => {
                // if transaction by account list is full, insert it only if it is local tx.
                if maxed_tx_per_account && !is_local {
                    return Err(Error::NotInsertedTxPerAccountFull);
                } else {
                    acc.push(scoredtx.clone());
                }
            }
            // if insertion is in middle (or beggining)
            Err(index) => {
                acc.insert(index, scoredtx.clone());
                //if there is max items, remove last one with lowest nonce
                if maxed_tx_per_account {
                    removed = acc.pop()
                }
            }
            // if there is tx match with same nonce. If new tx score is 12,5% greater then old score, replace it
            Ok(index) => {
                let old_score = acc[index].score;
                let bumped_old_score =
                    old_score.saturating_add(old_score >> BUMP_SCORE_BY_12_5_PERC);
                if scoredtx.score >= bumped_old_score {
                    acc.push(scoredtx.clone());
                    removed = Some(acc.swap_remove(index)); //swap_remove: The removed element is replaced by the last element of the vector.
                } else {
                    return Err(Error::NotReplacedIncreaseGas);
                }
            }
        }

        // remove it from removed list if present. (This is edge case, shoul not happen often).
        self.pending_removal.remove(&scoredtx.hash());

        // insert to other structures
        self.by_score.push(scoredtx.clone());
        self.by_hash.insert(scoredtx.hash(), scoredtx.clone());

        // remove replaced transaction
        if let Some(removed) = removed {
            self.by_hash.remove(&removed.hash);
            Ok(Some(removed))
        } else if self.by_score.len() > self.config_max_tx_count {
            // we dont check if it is local or not, because score for Local should be a lot higher. And max_tx_count should be hard limit.
            let worst_scoredtx = self.by_score.peek().unwrap().hash;
            let removed = self.remove(&worst_scoredtx);
            if worst_scoredtx == scoredtx.hash {
                //least scored tx is same as inserted tx.
                Err(Error::NotInsertedPoolFullIncreaseGas)
            } else {
                Ok(removed)
            }
        } else {
            Ok(None)
        }
    }

    pub fn remove(&mut self, txhash: &H256) -> Option<Arc<ScoreTransaction>> {
        // remove tx from by_hash
        if let Some(tx) = self.by_hash.remove(txhash) {
            // remove tx from by_accounts
            {
                let vec = self
                    .by_account
                    .get_mut(&tx.transaction.author().unwrap().0)
                    .unwrap();
                vec.remove(
                    vec.iter()
                        .position(|item| item.hash() == tx.hash())
                        .expect("expect to found tx in by_account struct"),
                );
            }
            // mark tx for removal from by_score
            self.pending_removal.insert(tx.hash);
            Some(tx.clone())
        } else {
            None
        }
    }

    fn recreate_heap(&mut self) {
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

    /// Return  all transactions in pool sorted from best to worst score.
    /// This is expensive opperation and it copies all transaction.
    pub fn sorted_vec(&mut self) -> Vec<Arc<ScoreTransaction>> {
        self.recreate_heap();
        self.by_score.clone().into_sorted_vec()
    }
}

#[cfg(test)]
mod tests {
    use core::transaction::LegacyPayload;

    use super::*;
    //use crate::Priority;
    use core::transaction::transaction::DUMMY_AUTHOR;

    fn new_tx(hash: H256, score: usize, nonce: usize, author: Address) -> Transaction {
        let mut tx = Transaction::default();
        tx.type_payload = TypePayload::Legacy(LegacyPayload {
            gas_price: score.into(),
        });
        tx = core::transaction::transaction::fake_sign(tx, author);
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

    #[test]
    fn insert_items_get_sorted() -> Result<(), Error> {
        //data
        let mut txpool = Transactions::new(10, 100);
        let (h1, h2, h3) = hashes();

        let tx1 = new_tx(h1, 10, 0, DUMMY_AUTHOR.0);
        let tx2 = new_tx(h2, 30, 1, DUMMY_AUTHOR.0);
        let tx3 = new_tx(h3, 20, 2, DUMMY_AUTHOR.0);

        //test
        txpool.insert(tx1, false)?;
        txpool.insert(tx2, false)?;
        txpool.insert(tx3, false)?;

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

    #[test]
    fn insert_remove() -> Result<(), Error> {
        //data
        let mut txpool = Transactions::new(10, 100);
        let (h1, h2, h3) = hashes();

        let tx1 = new_tx(h1, 10, 0, DUMMY_AUTHOR.0);
        let tx2 = new_tx(h2, 30, 1, DUMMY_AUTHOR.0);
        let tx3 = new_tx(h3, 20, 2, DUMMY_AUTHOR.0);

        //test
        txpool.insert(tx1, false)?;
        txpool.insert(tx2, false)?;
        txpool.insert(tx3, false)?;
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

    #[test]
    fn should_replace_nonce() -> Result<(), Error> {
        //data
        let mut txpool = Transactions::new(10, 100);
        let (h1, h2, h3) = hashes();

        let tx1 = new_tx(h1, 10, 0, DUMMY_AUTHOR.0);
        let tx2 = new_tx(h2, 30, 0, DUMMY_AUTHOR.0);
        let tx3 = new_tx(h3, 20, 0, DUMMY_AUTHOR.0);

        //test
        assert!(txpool.insert(tx1, false).is_ok());
        assert!(txpool.insert(tx2, false)?.is_some());
        assert_eq!(
            txpool.insert(tx3, false).unwrap_err(),
            Error::NotReplacedIncreaseGas
        );

        let sorted = txpool
            .sorted_vec()
            .into_iter()
            .map(|t: Arc<ScoreTransaction>| t.hash)
            .collect::<Vec<_>>();
        assert_eq!(sorted, vec![h2, h1]);

        Ok(())
    }

    #[test]
    fn check_max_tx_per_account() -> Result<(), Error> {
        let mut txpool = Transactions::new(2, 100);
        let (h1, h2, h3) = hashes();

        let tx1 = new_tx(h1, 10, 0, DUMMY_AUTHOR.0);
        let tx2 = new_tx(h2, 30, 1, DUMMY_AUTHOR.0);
        let tx3 = new_tx(h3, 20, 2, DUMMY_AUTHOR.0);

        assert!(txpool.insert(tx1, false).is_ok());
        assert!(txpool.insert(tx2, false).is_ok());
        assert_eq!(
            txpool.insert(tx3, false).unwrap_err(),
            Error::NotInsertedTxPerAccountFull
        );

        Ok(())
    }

    #[test]
    fn check_max_tx_count_not_enought_gas() -> Result<(), Error> {
        let mut txpool = Transactions::new(10, 2);
        let (h1, h2, h3) = hashes();

        let tx1 = new_tx(h1, 10, 0, DUMMY_AUTHOR.0);
        let tx2 = new_tx(h2, 30, 1, DUMMY_AUTHOR.0);
        let tx3 = new_tx(h3, 20, 2, DUMMY_AUTHOR.0);

        assert!(txpool.insert(tx3, false).is_ok());
        assert!(txpool.insert(tx2, false).is_ok());
        assert_eq!(
            txpool.insert(tx1, false).unwrap_err(),
            Error::NotInsertedPoolFullIncreaseGas
        );

        Ok(())
    }

    #[test]
    fn check_max_tx_count_enought_gas() -> Result<(), Error> {
        let mut txpool = Transactions::new(10, 2);
        let (h1, h2, h3) = hashes();

        let tx1 = new_tx(h1, 10, 0, DUMMY_AUTHOR.0);
        let tx2 = new_tx(h2, 30, 1, DUMMY_AUTHOR.0);
        let tx3 = new_tx(h3, 20, 2, DUMMY_AUTHOR.0);

        assert!(txpool.insert(tx1, false).is_ok());
        assert!(txpool.insert(tx2, false).is_ok());
        assert_eq!(txpool.insert(tx3, false).unwrap().unwrap().hash(), h1);

        Ok(())
    }

    #[test]
    fn remove_stalled() {
        let mut txpool = Transactions::new(10, 20);
        let (h1, h2, _) = hashes();
        let tx1 = new_tx(h1, 10, 0, DUMMY_AUTHOR.0);
        let tx2 = new_tx(h2, 30, 1, DUMMY_AUTHOR.0);

        assert!(txpool.insert(tx1, false).is_ok());
        std::thread::sleep(Duration::from_millis(500));
        assert!(txpool.insert(tx2, false).is_ok());
        assert_eq!(txpool._remove_stalled(Duration::from_millis(250)), 1);
        assert_eq!(*txpool.iter_unordered().next().unwrap().0, h2);
    }
}
