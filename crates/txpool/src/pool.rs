// Copyright 2021 Gnosis Ltd.
// SPDX-License-Identifier: Apache-2.0

use core::{Address, U256};
use std::sync::Arc;

use crate::{Config, ScoreTransaction, Transactions};

pub trait AccountNonces {
    fn nonce(&self, account: Address) -> U256;
}

pub trait Consensus {
    // Use by Aura and Clique to add seal the block (sign block hash and append it to Seal, check if we have right to seal it)
    fn generate_seal(&self);
    // on seal it is called relativly close after generate_seal.
    // Used only by Clique
    fn on_seal_block(&self);

    // Used only by AuRa it is probably for service transactions and updates.
    fn generate_service_transactions(&self);

    // schedule for what EIP is enabled or not. It depends on current best block
    fn schedule(&self);

    fn verify_transaction(&self); // Basic/Unorderer/Full

    // functions for isEIP-168 and isEIP-169
}

pub struct PendingBlock {
    pub tx: Vec<ScoreTransaction>,
    pub gas_price: U256,
}

/// Transaction pool.
pub struct Pool {
    txs: Transactions,
    /// configuration of pool
    config: Arc<Config>,
    // Dependencty injected services from client.
    //account_nonces: Arc<dyn AccountNonces>,
    //consensus: Arc<dyn Consensus>,
}

impl Pool {
    // currently hardcoded
    pub fn new(config: Config) -> Pool {
        Pool {
            txs: Transactions::new(config.max_tx_per_account, config.max_tx_count_global),
            config: Arc::new(Default::default()),
        }
    }

    pub fn txs(&self) -> &Transactions {
        &self.txs
    }

    pub fn txs_mut(&mut self) -> &mut Transactions {
        &mut self.txs
    }

    pub fn config(&self) -> &Config {
        &self.config
    }
}
