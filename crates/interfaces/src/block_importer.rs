// Copyright 2021 Gnosis Ltd.
// SPDX-License-Identifier: Apache-2.0

/// Block importer has few phases:
/// 1. solo block verification
/// 2. family block verification (child->parent verf);
/// 3. execution of transactions
/// 4. execution output verification
/// 5. final verification
/// 6. commit state to statedb
/// 7. commit block to blockchain
pub trait BlockImporter {
    //
    fn import_block(&mut self, block: &WireBlock);
    fn import_ancient_block(&self);
    fn verificator_info(&self) -> &VerificatorInfo;

    fn block_status(&self) -> Option<u32>;
}


pub struct BlockImporterInfo {
    queue_limit: u32,
    queue_len: u32,
    ancient_queue_limit: u32,
    ancient_queue_len: u32,
}