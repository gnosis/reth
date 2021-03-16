// Copyright 2020 Gnosis Ltd.
// SPDX-License-Identifier: Apache-2.0

use std::sync::{Arc, Mutex};
use super::rlp_en_de::{
    decode_block_headers,
    decode_new_block,
    decode_new_block_hashes,
    encode_block_headers,
    encode_block_bodies,
    encode_get_block_bodies,
    encode_get_block_headers
};
use crate::{
    client_adapter::Blockchain,
    common_types::{BlockId, BlockBody, GetBlockHeaders},
    devp2p_adapter::PeerPenal,
    scheduler::PeerOrganizer,
    scheduler::peer_organizer::{ErrorAct, InitialRequest, PeerId, Task},
    scheduler::protocol::{ProtocolId, MessageId, EthMessageId}
};
use primitive_types::H256;
use crate::block_manager::rlp_en_de::{decode_block_bodies, decode_get_block_headers, decode_get_block_bodies};

pub struct BlockManager {
    chain: Arc<Mutex<dyn Blockchain + Send + Sync>>,
}

//ALL APIs
impl BlockManager {
    pub fn new(chain: Arc<Mutex<dyn Blockchain + Send + Sync>>) -> Arc<Mutex<Self>> {
        Arc::new(Mutex::new(BlockManager { chain }))
    }

    fn request_block_headers(&self) -> InitialRequest {
        let request = GetBlockHeaders::new(BlockId::Number(10_000_000), 100, 0, false);
        let data = encode_get_block_headers(&request);
        InitialRequest::new(EthMessageId::GetBlockHeaders, data)
    }

    fn request_block_bodies(&self) -> InitialRequest {
        let hash: Vec<u8> = vec![
            254, 133, 237, 238, 76, 75, 76, 219, 252, 14, 247, 181, 240, 164, 1, 45,
            207, 31, 229, 94, 39, 154, 120, 247, 42, 246, 24, 88, 2, 167, 254, 215
        ];
        let hashes = vec![H256::from_slice(&hash)];
        let data = encode_get_block_bodies(&hashes);
        InitialRequest::new(EthMessageId::GetBlockBodies, data)
    }

    pub fn is_syncing(&self) -> bool {
        // TODO implement sync instead of this test request
        self.chain.lock().unwrap().best_block_header().is_none()
    }

    pub fn next_sync_task(&self) -> Option<InitialRequest> {
        // TODO implement sync instead of this test request
        if self.is_syncing() {
            Some(self.request_block_headers())
        } else {
            None
        }
    }

    pub fn api_new_block_hashes(&self, peer: &PeerId, data: &[u8]) -> Result<Task,ErrorAct> {
        match decode_new_block_hashes(data) {
            Ok(hashes) => {
                info!("Blockhashes: {:?}", hashes);
                Ok(Task::None)  // Task::InsertPeer()
            },
            Err(err) => ErrorAct::new_kick_generic(
                format!("Invalid NewBlockHashes request: {}", err)
            )
        }
    }

    pub fn api_get_block_headers(&self, peer: &PeerId, data: &[u8]) -> Result<Task, ErrorAct> {
        match decode_get_block_headers(data) {
            Ok(request) => {
                Ok(Task::Responde(
                    *peer,
                    ProtocolId::Eth,
                    MessageId::Eth(EthMessageId::BlockHeaders),
                    encode_block_headers(&self.chain.lock().unwrap().block_headers(request)),
                ))
            },
            Err(err) => {
                ErrorAct::new_kick_generic::<Task>(
                    format!("Invalid GetBlockHeaders request: {}", err)
                )
            }
        }

    }

    fn retrieve_block_bodies(&self, hashes: &[H256]) -> Vec<BlockBody> {
        let mut bodies = vec![];
        for ref hash in hashes {
            if let Some(body) = self.chain.lock().unwrap().block_body(hash) {
                bodies.push(body);
            }
        }
        bodies
    }

    pub fn api_get_block_bodies(&self, peer: &PeerId, data: &[u8]) -> Result<Task, ErrorAct> {
        match decode_get_block_bodies(data) {
            Ok(ref hashes) => {
                Ok(Task::Responde(
                    *peer,
                    ProtocolId::Eth,
                    MessageId::Eth(EthMessageId::BlockBodies),
                    encode_block_bodies(&self.retrieve_block_bodies(hashes)),
                ))
            },
            Err(err) => {
                ErrorAct::new_kick_generic::<Task>(
                    format!("Invalid GetBlockBodies request: {}", err)
                )
            }
        }
    }

    pub fn process_block_headers(&self, data: &[u8]) {
        let decoded = decode_block_headers(&data);
        match decoded {
            Ok(headers) => {
                info!("Decoded block headers: {:?}", headers);
                for ref header in headers {
                    self.chain.lock().unwrap().import_block_header(header);
                }
            },
            Err(err) => error!("Could not decode block header: {}", err),
        }
    }

    pub fn process_block_bodies(&self, data: &[u8]) {
        match decode_block_bodies(&data) {
            Ok(bodies) => {
                for ref body in bodies {
                    self.chain.lock().unwrap().import_block_body(body);
                }
            },
            Err(err) => error!("Could not decode block bodies: {}", err),
        }
    }

    pub fn api_new_block(&self, data: &[u8]) -> Result<Task, ErrorAct> {
        match decode_new_block(data) {
            Ok(new_block) => {
                info!("NewBlock: {:?}", new_block);
                Ok(Task::None)
            },
            Err(err) => {
                ErrorAct::new_kick_generic(
                format!("Invalid NewBlockHashes request: {}", err)
                )
            }
        }
    }

    pub fn api_get_receipts(&self) {}
}
