// Copyright 2020 Gnosis Ltd.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    block_manager::rlp_en_de::{encode_get_block_headers, encode_get_block_bodies},
    common_types::{BlockHeaderAndHash, GetBlockHeaders},
    scheduler::{peer_organizer::InitialRequest, protocol::EthMessageId},
};
use core::{BlockBody, BlockHeader, BlockId, H256, WireBlock};
use interfaces::{blockchain::BlockchainReadOnly, importer::Importer};

use std::{
    collections::HashMap,
    iter::Iterator,
    sync::{Arc, Mutex},
};
use std::cmp::min;

#[derive(Clone, Copy, Debug)]
enum SyncSliceStatus {
    Idle,
    AwaitingHeaders,
    ReceivedHeaders,
    AwaitingBodies,
    ReceivedBodies(usize),
}

pub struct SyncBuffer {
    headers: HashMap<H256, BlockHeader>,
    block_hashes: Vec<H256>,
    importer: Arc<Mutex<dyn Importer>>,
    status: SyncSliceStatus,
}

impl SyncBuffer {
    pub fn new(importer: Arc<Mutex<dyn Importer>>) -> Self {
        SyncBuffer {
            headers: HashMap::new(),
            block_hashes: vec![],
            importer,
            status: SyncSliceStatus::Idle,
        }
    }

    pub fn process_headers(&mut self, headers: &[BlockHeaderAndHash]) {
        info!("Sync: processing {} headers", headers.len());
        if headers.is_empty() {
            self.status = SyncSliceStatus::Idle;
            return;
        }
        let mut new_block_hashes = vec![];
        for header in headers {
            info!("HEADER {:?}", header);
            self.headers.insert(header.hash, header.header.clone());
            new_block_hashes.push(header.hash);
        }
        self.block_hashes = new_block_hashes;
        self.status = SyncSliceStatus::ReceivedHeaders;
    }

    pub fn unmatched_headers(&self) -> Vec<H256> { self.block_hashes.clone() }

    pub fn get_status(&self) -> SyncSliceStatus { self.status }

    pub fn set_status(&mut self, new_status: SyncSliceStatus) { self.status = new_status; }

    pub fn process_block_bodies(&mut self, bodies: &[BlockBody]) {
        let n_hashes = self.block_hashes.len();
        let n_bodies = bodies.len();
        info!("Got {} block bodies for {} headers", n_bodies, n_hashes);
        let n_blocks = min(n_hashes, n_bodies);
        for i in 0..n_blocks {
            let hash = &self.block_hashes[i];
            if let Some(header) = self.headers.get(hash) {
                let block = WireBlock { header: header.clone(), body: bodies[i].clone() };
                self.importer.lock().unwrap().import_block(&block);
            } else {
                error!("No matching header found for {}", hash);
            }
        }
        self.block_hashes.clear();
        self.status = SyncSliceStatus::ReceivedBodies(n_blocks);
    }
}

pub struct SyncWatcher {
    block_height: u64,
    buffer: Arc<Mutex<SyncBuffer>>,
    status: SyncSliceStatus,
}

fn request_block_headers(block_height: u64) -> InitialRequest {
    let request = GetBlockHeaders::new(BlockId::Number(block_height), 100, 0, false);
    let data = encode_get_block_headers(&request);
    info!("Sync: Requesting headers from {}", block_height);
    InitialRequest::new(EthMessageId::GetBlockHeaders, data)
}

fn request_block_bodies(hashes: &[H256]) -> InitialRequest {
    let data = encode_get_block_bodies(hashes);
    info!("Sync: Requesting {} block bodies", hashes.len());
    InitialRequest::new(EthMessageId::GetBlockBodies, data)
}

impl SyncWatcher {
    pub fn new(buffer: Arc<Mutex<SyncBuffer>>) -> Self {
        SyncWatcher { block_height: 10_000_000, buffer, status: SyncSliceStatus::Idle }
    }

    pub fn is_syncing(&self) -> bool {
        self.block_height < 10_001_000
    }

    pub fn next_sync_task(&mut self) -> Option<InitialRequest> {
        if self.is_syncing() {
            let status = self.buffer.lock().unwrap().get_status();
            info!("Sync status: {:?}", status);
            match status {
                SyncSliceStatus::Idle => {
                    Some(request_block_headers(self.block_height))
                },
                SyncSliceStatus::ReceivedHeaders => {
                    Some(request_block_bodies(&self.buffer.lock().unwrap().unmatched_headers()))
                }
                SyncSliceStatus::ReceivedBodies(n) => {
                    self.block_height += n as u64;
                    Some(request_block_headers(self.block_height))
                }
                _ => None,
            }
        } else {
            None
        }
    }

    pub fn sync_task_started(&mut self) {
        let mut buffer = self.buffer.lock().unwrap();
        match buffer.get_status() {
            SyncSliceStatus::Idle | SyncSliceStatus::ReceivedBodies(_) => {
                buffer.set_status(SyncSliceStatus::AwaitingHeaders);
            }
            SyncSliceStatus::ReceivedHeaders => {
                buffer.set_status(SyncSliceStatus::AwaitingBodies);
            },
            _ => {}
        }
    }
}