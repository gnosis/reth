// Copyright 2021 Gnosis Ltd.
// SPDX-License-Identifier: Apache-2.0

mod account;
mod block;
mod grpc;
pub mod transaction;

// large integers
pub use ethereum_types::{U128, U256, U512, U64};

// special purpose hashes
pub use ethereum_types::{Address, Bloom, H128, H160, H256, H512};

// grp conversion
pub use grpc::*;

pub type Keccak = H256;

pub type Bytes = Vec<u8>;

pub type BlockNumber = u64;

// domain types
pub use account::Account;
pub use block::{Block, BlockBody, BlockHeader, BlockId, BlockReceipt, WireBlock};
pub use transaction::Transaction;
