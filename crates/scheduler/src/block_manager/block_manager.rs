// Copyright 2020-2021 Gnosis Ltd.
// SPDX-License-Identifier: Apache-2.0

use super::rlp_en_de::{
    decode_block_headers_with_hash, decode_new_block, decode_new_block_hashes, encode_block_bodies,
    encode_block_headers, encode_get_block_bodies, encode_get_block_headers,
};
use crate::{
    block_manager::{
        rlp_en_de::{decode_block_bodies, decode_get_block_bodies, decode_get_block_headers},
        sync_buffer::{SyncBuffer, SyncWatcher},
    },
    common_types::GetBlockHeaders,
    scheduler::{
        peer_organizer::{ErrorAct, InitialRequest, PeerId, Task},
        protocol::{EthMessageId, MessageId},
        PeerOrganizer,
    },
};
use core::{BlockBody, BlockId, H256};
use interfaces::{
    blockchain::BlockchainReadOnly,
    devp2p::{PeerPenal, ProtocolId},
    importer::Importer,
};
use std::sync::{Arc, Mutex, MutexGuard};

pub struct Devp2pHandler {
    chain: Arc<Mutex<BlockchainReadOnly>>,
}

impl Devp2pHandler {
    pub fn new(chain: Arc<Mutex<BlockchainReadOnly>>) -> Self {
        Devp2pHandler { chain }
    }

    pub fn new_block_hashes(&self, peer: &PeerId, data: &[u8]) -> Result<Task, ErrorAct> {
        match decode_new_block_hashes(data) {
            Ok(hashes) => {
                info!("Blockhashes: {:?}", hashes);
                Ok(Task::None) // Task::InsertPeer()
            }
            Err(err) => {
                ErrorAct::new_kick_generic(format!("Invalid NewBlockHashes request: {}", err))
            }
        }
    }

    pub fn get_block_headers(&self, peer: &PeerId, data: &[u8]) -> Result<Task, ErrorAct> {
        match decode_get_block_headers(data) {
            Ok(request) => Ok(Task::Responde(
                *peer,
                ProtocolId::Eth,
                MessageId::Eth(EthMessageId::BlockHeaders),
                encode_block_headers(&self.chain.lock().unwrap().header_request(
                    request.block_id,
                    request.max_headers,
                    request.skip,
                    request.reverse,
                )),
            )),
            Err(err) => ErrorAct::new_kick_generic::<Task>(format!(
                "Invalid GetBlockHeaders request: {}",
                err
            )),
        }
    }

    fn retrieve_block_bodies(&self, hashes: &[H256]) -> Vec<BlockBody> {
        let mut bodies = vec![];
        for ref hash in hashes {
            if let Some(body) = self.chain.lock().unwrap().body(hash) {
                bodies.push(body);
            }
        }
        bodies
    }

    pub fn get_block_bodies(&self, peer: &PeerId, data: &[u8]) -> Result<Task, ErrorAct> {
        match decode_get_block_bodies(data) {
            Ok(ref hashes) => Ok(Task::Responde(
                *peer,
                ProtocolId::Eth,
                MessageId::Eth(EthMessageId::BlockBodies),
                encode_block_bodies(&self.retrieve_block_bodies(hashes)),
            )),
            Err(err) => ErrorAct::new_kick_generic::<Task>(format!(
                "Invalid GetBlockBodies request: {}",
                err
            )),
        }
    }

    pub fn new_block(&self, peer: &PeerId, data: &[u8]) -> Result<Task, ErrorAct> {
        match decode_new_block(data) {
            Ok(new_block) => {
                info!("NewBlock: {:?}", new_block);
                Ok(Task::None)
            }
            Err(err) => {
                ErrorAct::new_kick_generic(format!("Invalid NewBlockHashes request: {}", err))
            }
        }
    }

    pub fn get_receipts(&self) {}
}

pub struct BlockchainSync {
    buffer: Arc<Mutex<SyncBuffer>>,
    watcher: Arc<Mutex<SyncWatcher>>,
    devp2p: Arc<Mutex<Devp2pHandler>>,
}

impl BlockchainSync {
    pub fn new(
        chain: Arc<Mutex<dyn BlockchainReadOnly>>,
        importer: Arc<Mutex<dyn Importer>>,
    ) -> Self {
        let buffer = Arc::new(Mutex::new(SyncBuffer::new(Arc::clone(&importer))));
        let watcher = Arc::new(Mutex::new(SyncWatcher::new(Arc::clone(&buffer))));
        let devp2p = Arc::new(Mutex::new(Devp2pHandler::new(Arc::clone(&chain))));
        BlockchainSync {
            buffer,
            watcher,
            devp2p,
        }
    }

    pub fn is_syncing(&self) -> bool {
        self.watcher.lock().unwrap().is_syncing()
    }

    pub fn next_sync_task(&self) -> Option<InitialRequest> {
        self.watcher.lock().unwrap().next_sync_task()
    }

    pub fn sync_task_started(&self) {
        self.watcher.lock().unwrap().sync_task_started();
    }

    pub fn process_block_headers(&self, data: &[u8]) {
        match decode_block_headers_with_hash(&data) {
            Ok(headers) => self.buffer.lock().unwrap().process_headers(&headers),
            Err(err) => error!("Sync: Could not decode block header: {}", err),
        }
    }

    pub fn process_block_bodies(&self, data: &[u8]) {
        match decode_block_bodies(&data) {
            Ok(bodies) => self.buffer.lock().unwrap().process_block_bodies(&bodies),
            Err(err) => error!("Sync: Could not decode block bodies: {}", err),
        }
    }

    pub fn api_new_block_hashes(&self, peer: &PeerId, data: &[u8]) -> Result<Task, ErrorAct> {
        self.devp2p.lock().unwrap().new_block_hashes(peer, data)
    }

    pub fn api_get_block_headers(&self, peer: &PeerId, data: &[u8]) -> Result<Task, ErrorAct> {
        self.devp2p.lock().unwrap().get_block_headers(peer, data)
    }

    pub fn api_get_block_bodies(&self, peer: &PeerId, data: &[u8]) -> Result<Task, ErrorAct> {
        self.devp2p.lock().unwrap().get_block_bodies(peer, data)
    }

    pub fn api_new_block(&self, peer: &PeerId, data: &[u8]) -> Result<Task, ErrorAct> {
        self.devp2p.lock().unwrap().new_block(peer, data)
    }

    pub fn api_get_receipts(&self) {
        self.devp2p.lock().unwrap().get_receipts()
    }
}
