use core::{BlockNumber, H256};

// Copyright 2021 Gnosis Ltd.
// SPDX-License-Identifier: Apache-2.0

pub trait Snapshot: Send + Sync {
    fn create_snapshot(&self);

    fn manifest(&self);
    fn status(&self);

    fn chunk(&self);
    // warping
    fn begin_restoration(&self, manifest: &Manifest);
    fn abort_restoration(&self);
    fn restore_chunk(&self, chunk: Vec<u8>, chunk_type: ChunkType);

    // TODO just for now
    fn manifest_status(&self) -> Manifest {
        Manifest {
            block_number: 0,
            hash: H256::zero(),
        }
    }
}

pub enum ChunkType {
    Block,
    State,
}

pub struct Manifest {
    pub block_number: BlockNumber,
    pub hash: H256,
}

impl Manifest {
    pub fn not_exist(&self) -> bool {
        self.block_number == 0
    }
}
