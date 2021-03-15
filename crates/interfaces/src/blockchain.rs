// Copyright 2021 Gnosis Ltd.
// SPDX-License-Identifier: Apache-2.0

/// Trait that allows getting blocks data
pub trait BlockchainReadOnly {
    fn header(&self, number: BlockNumber) -> Option<BlockHeader>;
    fn header_list(&self, request: Vec<BlockNumber>) -> Vec<BlockHeader>;
    fn body(&self, hash: &H256) -> Option<BlockBody>;
    fn receipt(&self) -> Option<BlockReceipt>;
    fn best_header(&self) -> Option<&BlockNumber>;
    fn tx(&self);
}


pub trait BlockchainCommit {
    fn commit_block(&self);
}
