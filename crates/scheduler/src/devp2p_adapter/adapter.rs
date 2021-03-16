// Copyright 2020 Gnosis Ltd.
// SPDX-License-Identifier: Apache-2.0

use std::sync::Arc;

use crate::scheduler::{
    peer_organizer::{PeerCapability, PeerId},
    protocol::ProtocolId,
};

pub trait Devp2pAdapter: Send + Sync {
    fn start(&self);
    fn stop(&self);
    fn register_handler(&self, handle: Arc<dyn Devp2pInbound>);
    //unregister handler?
    fn send_mesage(&self, protocol: ProtocolId, peer: &PeerId, mesage_id: u8, data: &[u8]);
    fn penalize_peer(&self, peer: &PeerId, penal: PeerPenal);
}
#[derive(Debug, Copy, Clone)]
pub enum PeerPenal {
    Kick,
    Ban,
}

pub trait Devp2pInbound: Send + Sync {
    /// Called when new network packet received.
    fn receive_message(&self, peer: &PeerId, protocol: ProtocolId, message_id: u8, data: &[u8]);
    /// Called when new peer is connected. Only called when peer supports the same protocol.
    fn connected(&self, peer: &PeerId, capability: &PeerCapability);
    /// Called when a previously connected peer disconnects.
    fn disconnected(&self, peer: &PeerId);
}
