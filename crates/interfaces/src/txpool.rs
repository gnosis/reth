

// Copyright 2020-2021 Gnosis Ltd.
// SPDX-License-Identifier: Apache-2.0

use reth_core::{H256};
use async_trait::async_trait;
use super::world_state::BlockUpdate;

/// Trait that allows getting blocks data
#[async_trait]
pub trait TransactionPool: 'static {
    /// preserves incoming order, changes amount, unknown hashes will be omitted
    async fn filter_by_negative(&self, hashes: Vec<H256>) -> Vec<H256>;
    /// preserves incoming order and amount
    async fn import(&self, tx: Vec<Vec<u8>>) -> Vec<anyhow::Result<()>>;
    /// preserves incoming order and amount, if some transaction doesn't exists in pool - returns nil in this slot
    async fn find(&self, hashes: Vec<H256>) -> Vec<Option<Vec<u8>>>;
    /// Remove transaction from tx
    async fn remove(&self, hashes: Vec<H256>);

    /// When we are fully synced, we should get all transaction
    /// from reverted blocks and reinclude them in pool and remove all tx that we found in new block.
    async fn block_update(&self, chain: &BlockUpdate);

    // support for config set/get? Support for status rpc?
    // support for local tx?

}