// Copyright 2020 Gnosis Ltd.
// SPDX-License-Identifier: Apache-2.0

use crate::common_types::{BlockBody, BlockHeader, BlockNumber, GetBlockHeaders};
use primitive_types::H256;

pub trait Blockchain {
    fn block_header(&self, number: BlockNumber) -> Option<BlockHeader>;
    fn block_headers(&self, request: GetBlockHeaders) -> Vec<BlockHeader>;
    fn block_body(&self, hash: &H256) -> Option<BlockBody>;
    fn block_receipt(&self);
    fn best_block_header(&self) -> Option<&BlockNumber>;

    fn import_block_header(&mut self, header: &BlockHeader);
    fn import_block_body(&mut self, body: &BlockBody);
    fn import_old_block(&self);
}
