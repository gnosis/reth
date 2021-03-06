// Copyright 2021 Gnosis Ltd.
// SPDX-License-Identifier: Apache-2.0

use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

pub type ProtocolIdType = [u8; 3];
pub type PeerId = usize;
pub type PeerCapability = HashMap<ProtocolId, HashSet<u8>>;

#[derive(Debug, Eq, PartialEq, Hash, Copy, Clone)]
pub enum ProtocolId {
    Parity,
    Eth,
}

impl ProtocolId {
    pub fn to_protocol_type(self) -> ProtocolIdType {
        match self {
            Self::Parity => *b"par",
            Self::Eth => *b"eth",
        }
    }
}

/// Interface that devp2p need to implement to merge with rest of reth modules.
pub trait Adapter: Send + Sync {
    fn start(&self);
    fn stop(&self);
    fn register_handler(&self, handle: Arc<dyn Inbound>);
    //unregister handler?
    fn send_mesage(&self, protocol: ProtocolId, peer: &PeerId, mesage_id: u8, data: &[u8]);
    fn penalize_peer(&self, peer: &PeerId, penal: PeerPenal);
}

/// Types of penalty that scheduler can send to devp2p
#[derive(Debug, Copy, Clone)]
pub enum PeerPenal {
    Kick,
    Ban,
}

/// Inbound communication from devp2p to scheduler side
pub trait Inbound: Send + Sync {
    /// Called when new network packet received.
    fn receive_message(&self, peer: &PeerId, protocol: ProtocolId, message_id: u8, data: &[u8]);
    /// Called when new peer is connected. Only called when peer supports the same protocol.
    fn connected(&self, peer: &PeerId, capability: &PeerCapability);
    /// Called when a previously connected peer disconnects.
    fn disconnected(&self, peer: &PeerId);
}
