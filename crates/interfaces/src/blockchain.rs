use core::{BlockBody, BlockHeader, BlockId, BlockNumber, BlockReceipt, H256};

// Copyright 2021 Gnosis Ltd.
// SPDX-License-Identifier: Apache-2.0

/// Trait that allows getting blocks data
pub trait BlockchainReadOnly: Send + Sync {
    fn header(&self, number: BlockNumber) -> Option<BlockHeader>;
    fn header_list(&self, request: Vec<BlockId>) -> Vec<BlockHeader>;
    fn header_request(
        &self,
        block_id: BlockId,
        max_header: u64,
        skip: u64,
        reverse: bool,
    ) -> Vec<BlockHeader>;
    fn body(&self, hash: &H256) -> Option<BlockBody>;
    fn receipt(&self) -> Option<BlockReceipt>;
    fn best_header(&self) -> Option<BlockNumber>;
    fn tx(&self);

    //TODO just temporarily added this. This should not be part of blockchain for current flow.
    fn import_header(&self, header: &BlockHeader);
}

pub trait BlockchainCommit {
    fn commit_block(&self);
}
