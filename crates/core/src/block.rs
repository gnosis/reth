// Copyright 2021 Gnosis Ltd.
// SPDX-License-Identifier: Apache-2.0

use crate::{Address, Bloom, Keccak, Transaction, U256};

use serde::{Deserialize, Serialize};

/// https://ethereum.stackexchange.com/questions/268/ethereum-block-architecture
#[derive(Debug, Eq, Clone, PartialEq, Serialize, Deserialize)]
pub struct BlockHeader {
    number: u64,
    timestamp: u64,
    author: Address,
    parent_hash: Keccak,
    ommers_hash: Keccak,

    state_root: Keccak,
    receipts_root: Keccak,
    transactions_root: Keccak,
    logs_bloom: Bloom,

    gas_used: U256,
    gas_limit: U256,
    difficulty: U256,
    mix_hash: Keccak,
    nonce: u64,

    extra_data: [u8; 32],
}

/// https://ethereum.stackexchange.com/questions/268/ethereum-block-architecture
//#[derive(Debug, Eq, Clone, PartialEq, Serialize, Deserialize)]
pub struct Block {
    _header: BlockHeader,
    _ommers: Vec<Block>,
    _transactions: Vec<Transaction>,
}
