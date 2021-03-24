// Copyright 2020-2021 Gnosis Ltd.
// SPDX-License-Identifier: Apache-2.0

use crate::snapshot_manager;
use interfaces::{
    devp2p::{PeerPenal, ProtocolId},
    importer::ImporterStatus,
    snapshot::{Manifest, Snapshot},
};

use super::{
    peer_organizer::{ErrorAct, PeerCapability, PeerId, Task, TaskId},
    protocol::{EthProtocolVersion, ParityProtocolVersion},
};
use rlp::{DecoderError, Rlp, RlpStream};
use std::{collections::HashMap, str::FromStr};

use core::{H256, U256};
use ethereum_forkid::{ForkFilter, ForkId};

#[derive(Debug, Clone)]
pub struct Handshake {
    pub peers: HashMap<PeerId, (TaskId, PeerCapability)>,
    // field bellow are needed for creating and verifying status msg
    pub network_id: u64, //it is hard coded in start
    pub genesis_hash: H256,
    //pub fork_filter: ForkFilter,
}

#[derive(Debug, Clone, Copy)]
pub struct HandshakeInfo {
    pub peer_id: PeerId,
    pub eth_protocol_version: u8,
    pub genesis_hash: H256,
    pub network_id: u64,
    pub latest_hash: H256,
    pub total_difficulty: Option<U256>,
    pub fork_id: Option<ForkId>,
    pub snapshot: Option<(H256, U256)>, //latest snapshot (hash,number)
}

impl Handshake {
    pub fn new() -> Handshake {
        Handshake {
            peers: HashMap::new(),
            network_id: 1,
            genesis_hash: H256::from_str(
                "d4e56740f876aef8c010b86a40d5f56745a118d0906a34e69aec8c0db1cb8fa3",
            )
            .unwrap(),
            //ForkFilter:: ForkFilter::fork_filter()
        }
    }

    fn encode_rlp_status_msg(
        status: &ImporterStatus,
        protocol_version: u32,
        fork_ids: Option<ForkId>,
        snapshot_manifest: Option<Manifest>,
    ) -> Vec<u8> {
        let mut rlp = RlpStream::new();
        rlp.begin_unbounded_list();
        rlp.append(&protocol_version); //send protocol
        rlp.append(&status.network_id); //network ID
        rlp.append(&status.total_difficulty);
        rlp.append(&status.highest_block.1);
        rlp.append(&status.genesis_block_hash);

        if let Some(fork_id) = fork_ids {
            //protocol
            rlp.append(&fork_id);
        }

        if let Some(_snapshot_ms) = snapshot_manifest {
            //TODO for now send empty
            //rlp.append(&H256::zero());//&snapshot_ms.hash);
            //rlp.append(&(0 as u64));//&snapshot_ms.block_number);
        }
        rlp.finalize_unbounded_list();
        rlp.out().to_vec()
    }

    fn decode_rlp_status_msg(
        data: &[u8],
        has_parity_protocol: bool,
    ) -> Result<HandshakeInfo, DecoderError> {
        let iter = Rlp::new(data);
        let mut iter = iter.iter();

        let eth_protocol_version = iter.next().ok_or(DecoderError::RlpIsTooShort)?.as_val()?;
        let network_id = iter.next().ok_or(DecoderError::RlpIsTooShort)?.as_val()?;
        let total_difficulty = Some(iter.next().ok_or(DecoderError::RlpIsTooShort)?.as_val()?);
        let latest_hash: H256 = iter.next().ok_or(DecoderError::RlpIsTooShort)?.as_val()?;
        let genesis_hash: H256 = iter.next().ok_or(DecoderError::RlpIsTooShort)?.as_val()?;

        let fork_id = if eth_protocol_version >= EthProtocolVersion::VERSION_64.to_version_byte() {
            Some(iter.next().ok_or(DecoderError::RlpIsTooShort)?.as_val()?)
        } else {
            None
        };
        let mut snapshot = None;
        if has_parity_protocol {
            snapshot = Some((
                iter.next().ok_or(DecoderError::RlpIsTooShort)?.as_val()?,
                iter.next().ok_or(DecoderError::RlpIsTooShort)?.as_val()?,
            ));
        }
        info!("STATUS MSG IS OKAY");

        Ok(HandshakeInfo {
            peer_id: 0,
            eth_protocol_version,
            network_id,
            total_difficulty,
            latest_hash,
            genesis_hash,
            fork_id,
            snapshot,
        })
    }

    pub fn connect_and_create_status_message(
        &mut self,
        peer: &PeerId,
        id: TaskId,
        capability: &PeerCapability,
        status: &ImporterStatus,
        snapshot_manifest: Manifest,
    ) -> Vec<u8> {
        self.peers.insert(*peer, (id, capability.clone()));
        let mut fork_id = None;
        if let Some(eth_ver) = capability.get(&ProtocolId::Eth) {
            if eth_ver
                .iter()
                .find(|&ver| *ver >= EthProtocolVersion::VERSION_64.to_number())
                .is_some()
            {
                fork_id = Some(status.fork);
            }
        };
        let mut snap_manifest = None;

        if let Some(par_ver) = capability.get(&ProtocolId::Parity) {
            if par_ver.contains(&ParityProtocolVersion::VERSION_2.to_number()) {
                snap_manifest = Some(snapshot_manifest);
            }
        }

        Self::encode_rlp_status_msg(
            status,
            EthProtocolVersion::VERSION_64.to_number() as u32,
            fork_id,
            snap_manifest,
        )
    }

    pub fn verify_status(&self, hi: &HandshakeInfo) -> Result<(), ErrorAct> {
        if hi.eth_protocol_version < EthProtocolVersion::VERSION_64.to_version_byte() {
            ErrorAct::new_kick("Unsupported Eth version".into())?
        }
        if hi.genesis_hash != self.genesis_hash {
            ErrorAct::new_kick("Genesis hash is different".into())?
        }
        if hi.network_id != self.network_id {
            ErrorAct::new_kick("Network id is different".into())?
        }
        //TODO self.fork_filter.is_compatible(hi.fork_id)?

        Ok(())
    }

    pub fn handle_status_message(&mut self, peer: &PeerId, data: &[u8]) -> Result<Task, ErrorAct> {
        if let Some((_, capability)) = self.peers.remove(peer) {
            match Self::decode_rlp_status_msg(data, capability.contains_key(&ProtocolId::Parity)) {
                Ok(mut hi) => {
                    hi.peer_id = *peer;
                    self.verify_status(&hi)?;
                    return Ok(Task::InsertPeer(hi));
                }
                Err(err) => ErrorAct::new_kick(format!("Handshake error:{:?}", err))?,
            };
        }
        Err(ErrorAct::new_kick("Unknown peer in handshake".into()).expect_err(""))
    }

    pub fn disconnect(&mut self, peer: &PeerId) -> Option<TaskId> {
        self.peers
            .remove(peer)
            .and_then(|(task_id, _)| Some(task_id))
    }
}
