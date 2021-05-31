use std::sync::Arc;

use crate::{Error, BUMP_SCORE_BY_12_5_PERC};
use super::score::ScoreTransaction;
use anyhow::Result;
use interfaces::world_state::AccountInfo;
use reth_core::{Transaction, H256};

pub struct Account {
    pub info: AccountInfo,
    transactions: Vec<Arc<Transaction>>,
}

impl Account {
    pub fn new(info: AccountInfo) -> Account {
        Account {
            info,
            transactions: Vec::new(),
        }
    }

    pub fn txs(&self) -> &[Arc<Transaction>] {
        &self.transactions
    }

    pub fn info(&self) -> &AccountInfo {
        &self.info
    }

    // remove transaction and return is_empty
    pub fn remove(&mut self, hash: &H256) -> bool {
        let index = self
            .transactions
            .iter()
            .position(|item| item.hash() == *hash)
            .expect("expect to found tx in by_account struct");
        self.transactions.remove(index);
        self.transactions.is_empty()
    }

    /// if okay, return pair of replaced and removed transaction with unsuficient fund.
    pub fn insert(
        &mut self,
        tx: &ScoreTransaction,
        max_per_account: usize,
    ) -> Result<(Option<Arc<Transaction>>, Vec<Arc<Transaction>>)> {
        // placeholder for replaced transaction
        let mut replaced = None;

        //find place where to insert new tx.
        let index = self
            .transactions
            .binary_search_by(|old| old.nonce.cmp(&tx.tx.nonce));

        let insert_index = match index {
            Ok(id) => id,
            Err(id) => id,
        };
        // check if we have enought gas for new tx
        let balance = self.transactions[0..insert_index]
            .iter()
            .fold(self.info.balance, |acum, tx| acum.saturating_sub(tx.cost()));
        if tx.cost() > balance {
            return Err(Error::NotInsertedBalanceInsufficient.into());
        }

        // insert tx in by_account
        let maxed_tx_per_account = self.transactions.len() == max_per_account;
        match index {
            // if insertion is in middle (or beginning)
            Err(index) => {
                // if nonce is greater then all present
                // and if transaction by account list is full.
                if index == self.transactions.len() && maxed_tx_per_account {
                    return Err(Error::NotInsertedTxPerAccountFull.into());
                }

                self.transactions.insert(index, tx.tx.clone());
                // if there is max items, remove last one with lowest nonce
                if maxed_tx_per_account {
                    // check if it is local tx
                    replaced = self.transactions.pop();
                }
            }
            // if there is tx match with same nonce and if new tx score is 12,5% greater then old score, replace it
            Ok(index) => {
                let old_score = self.transactions[index].gas_limit; // TODO get effective_gas
                let bumped_old_score =
                    old_score.saturating_add(old_score >> BUMP_SCORE_BY_12_5_PERC);
                if tx.gas_limit > bumped_old_score {
                    //TODO get effective_gas
                    println!(
                        "Replacing tx with nonce:{:?}, new_gas:{:?} old_gas:{:?}",
                        tx.tx.nonce, tx.gas_limit, bumped_old_score
                    );
                    self.transactions.push(tx.tx.clone());
                    replaced = Some(self.transactions.swap_remove(index)); //swap_remove: The removed element is replaced by the last element of the vector.
                } else {
                    return Err(Error::NotReplacedIncreaseGas.into());
                }
            }
        }

        let mut unsuficient_fund = Vec::new();
        // now, with sorted tx inside our by_account struct
        // We can calculate cost and check if we disrupted cost for tx with greater nonce.
        // if there is not enought gas for those transactions remove them.
        let mut left_balance = balance;
        for tx in self.transactions[insert_index..].iter() {
            let cost = tx.cost();
            if cost > left_balance {
                unsuficient_fund.push(tx.clone());
            }
            left_balance = left_balance.saturating_sub(cost);
        }

        Ok((replaced, unsuficient_fund))
    }
}
