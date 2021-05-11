// Copyright 2021 Gnosis Ltd.
// SPDX-License-Identifier: Apache-2.0

use crate::{BlockNumber, Transaction, H160, H256};

#[derive(Clone, Debug, PartialEq, Hash, Eq)]
pub enum BlockId {
    Number(BlockNumber),
    Hash(H256),
    Latest,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BlockHeader {
    pub parent_hash: H256,
    pub ommers_hash: H256,
    pub beneficiary_address: H160,
    pub state_root: H256,
    pub transactions_root: H256,
    pub receipts_root: H256,
    pub logs_bloom: Vec<u8>,
    pub difficulty: u64,
    pub number: u64,
    pub gas_limit: u64,
    pub gas_used: u64,
    pub timestamp: u64,
    pub extra_data: Vec<u8>,
    pub mix_hash: H256,
    pub nonce: u64,
}

pub struct BlockReceipt {}

#[derive(Clone, Debug)]
pub struct BlockBody {
    pub transactions: Vec<Transaction>,
    pub ommers: Vec<BlockHeader>,
}

pub struct WireBlock {
    pub header: BlockHeader,
    pub body: BlockBody,
}

/// https://ethereum.stackexchange.com/questions/268/ethereum-block-architecture
//#[derive(Debug, Eq, Clone, PartialEq, Serialize, Deserialize)]
pub struct Block {
    _header: BlockHeader,
    _ommers: Vec<Block>,
    _transactions: Vec<Transaction>,
}
