// Copyright 2020-2021 Gnosis Ltd.
// SPDX-License-Identifier: Apache-2.0

use super::world_state::BlockUpdate;
use async_trait::async_trait;
use reth_core::H256;

/// Trait that allows getting blocks data
#[async_trait]
pub trait TransactionPool: 'static {
    /// preserves incoming order, changes amount, unknown hashes will be omitted
    async fn filter_by_negative(&self, hashes: &[H256]) -> Vec<H256>;
    /// preserves incoming order and amount
    async fn import(&self, tx: &[Vec<u8>]) -> Vec<anyhow::Result<()>>;
    /// preserves incoming order and amount, if some transaction doesn't exists in pool - returns nil in this slot
    async fn find(&self, hashes: &[H256]) -> Vec<Option<Vec<u8>>>;
    /// Remove transaction from tx
    async fn remove(&self, hashes: &[H256]);

    /// When we are fully synced, we should get all transaction from reverted blocks
    /// to reinclude them in pool and remove all tx that were included in new block.
    async fn block_update(&self, chain: &BlockUpdate);

    /* config setters:
    fn raise_min_gas_price(&mut self);
    fn raise_block_gas_limit(&mut self);
    */

    /* info getters
    fn worst_gas_price_tx(&self);
    fn next_account_nonce(&self);
    fn status(&self);
    */
}
