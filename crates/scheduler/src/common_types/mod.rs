// will be extracted to separate library. Maybe in util :)

use core::{BlockBody, BlockHeader, BlockId, BlockNumber, Transaction, H160, H256, U256};

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
        GetBlockHeaders {
            block_id,
            max_headers,
            skip,
            reverse,
        }
    }
}

#[derive(Debug)]
pub struct NewBlock {
    pub header: BlockHeader,
    pub transactions: Vec<Transaction>,
    pub ommers: Vec<BlockHeader>,
    pub score: U256,
}
