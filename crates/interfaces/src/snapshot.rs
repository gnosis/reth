// Copyright 2021 Gnosis Ltd.
// SPDX-License-Identifier: Apache-2.0

pub trait Snapshot {

    pub fn create_snapshot(&self);

    pub fn manifest(&self);
    pub fn status(&self);


    pub fn chunk(&self);
    // warping
    pub fn begin_restoration(manifest: &Manifest);
    pub fn abort_restoration(&self);
    pub fn restore_chunk(&self, chunk: Vec<u8>, chunk_type: ChunkType);
}