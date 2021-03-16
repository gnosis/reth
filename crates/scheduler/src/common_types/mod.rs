// will be extracted to separate library. Maybe in util :)

use primitive_types::{H160, H256, U256};

pub type BlockNumber = u64;

#[derive(Clone, Debug, PartialEq)]
pub enum BlockId {
    Number(BlockNumber),
    Hash(H256),
}

#[derive(Debug, PartialEq)]
pub struct NewBlockHash {
    pub hash: H256,
    pub number: BlockNumber,
}

impl NewBlockHash {
    pub fn new(hash: H256, number: BlockNumber) -> Self {
        NewBlockHash { hash, number }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct GetBlockHeaders {
    pub block_id: BlockId,
    pub max_headers: u64,
    pub skip: u64,
    pub reverse: bool,
}

impl GetBlockHeaders {
    pub fn new(block_id: BlockId, max_headers: u64, skip: u64, reverse: bool) -> GetBlockHeaders {
        GetBlockHeaders { block_id, max_headers, skip, reverse }
    }
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

#[derive(Clone, Debug, PartialEq)]
pub struct BlockTransaction {
    pub nonce: U256,
    pub gas_price: U256,
    pub gas_limit: U256,
    pub to: H160,
    pub value: U256,
    pub input_data: Vec<u8>,
    pub v: u8,
    pub r: U256,
    pub s: U256,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BlockBody {
    pub transactions: Vec<BlockTransaction>,
    pub ommers: Vec<BlockHeader>,
}

#[derive(Debug)]
pub struct NewBlock {
    pub header: BlockHeader,
    pub transactions: Vec<BlockTransaction>,
    pub ommers: Vec<BlockHeader>,
    pub score: U256,
}
