// Copyright 2021 Gnosis Ltd.
// SPDX-License-Identifier: Apache-2.0

pub trait TransactionPool {

    /// Register new broadcaster that notifies when new transaction is added to pool
    fn register_broadcaster(&self);
    fn unregister_broadcaster(&self);

    // configs, updated when new block is mined.
    fn raise_min_gas_price(&mut self);
    fn raise_block_gas_limit(&mut self);

    // standard function for insert/find/filter/remove 
    fn insert(&mut self, tx: Vec<Tx>);
    fn find(&self);
    fn filter(&self);
    fn remove(&mut self, tx_hash_list: Vec<Hash>);


    // there is support for local transaction that needs to be inside poll all the time 
    // local transactions??? insert local

    //???
    fn worst_gas_price_tx(&self);
    fn next_account_nonce(&self);
    fn status(&self);
}