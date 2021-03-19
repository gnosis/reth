// Copyright 2021 Gnosis Ltd.
// SPDX-License-Identifier: Apache-2.0

mod account;
mod block;
pub mod transaction;

// large integers
pub use ethereum_types::{U256, U64};

// special purpose hashes
pub use ethereum_types::{Address, Bloom, H160, H256};

pub type Keccak = H256;

pub type Bytes = Vec<u8>;

pub type BlockNumber = u64;

// domain types
pub use account::Account;
pub use block::{Block, BlockBody, BlockHeader, BlockId, BlockReceipt};
pub use transaction::Transaction;
