// Copyright 2020 Gnosis Ltd.
// SPDX-License-Identifier: Apache-2.0

pub type ProtocolIdType = [u8; 3];

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

/// ETH protocol version related protocol
#[derive(PartialEq)]
pub enum EthProtocolVersion {
    VERSION_63,
    VERSION_64,
    HIGHER_VERSION(u8),
}

impl EthProtocolVersion {
    pub fn to_version_byte(self) -> u8 {
        match self {
            Self::VERSION_63 => 0x11,
            Self::VERSION_64 => 0x11,
            Self::HIGHER_VERSION(ver) => ver,
        }
    }

    pub fn to_number(self) -> u8 {
        match self {
            Self::VERSION_63 => 63,
            Self::VERSION_64 => 64,
            Self::HIGHER_VERSION(ver) => ver,
        }
    }

    pub fn from_version_byte(byte: u8) -> Option<EthProtocolVersion> {
        match byte {
            0x11 => Some(Self::VERSION_64),
            byte if byte > 0x11 => Some(Self::HIGHER_VERSION(byte)),
            _ => None,
        }
    }
}

/// Parity protocol version related protocol
pub enum ParityProtocolVersion {
    VERSION_1 = 1,
    VERSION_2 = 2,
}

impl ParityProtocolVersion {
    pub fn to_number(self) -> u8 {
        match self {
            Self::VERSION_1 => 1,
            Self::VERSION_2 => 2,
        }
    }

    pub fn to_version_byte(self) -> u8 {
        match self {
            Self::VERSION_1 => 0x15,
            Self::VERSION_2 => 0x16,
        }
    }

    pub fn from_version_byte(byte: u8) -> Option<ParityProtocolVersion> {
        match byte {
            0x15 => Some(Self::VERSION_1),
            0x16 => Some(Self::VERSION_2),
            _ => None,
        }
    }
}

#[derive(FromPrimitive,Debug,Copy,Clone)]
pub enum EthMessageId {
    Status = 0x00,
    NewBlockHashes = 0x01,
    Transactions = 0x02,
    GetBlockHeaders = 0x03,
    BlockHeaders = 0x04,
    GetBlockBodies = 0x05,
    BlockBodies = 0x06,
    NewBlock = 0x07,
    // NewPooledTransactionHashes = 0x08, // eth/65 protocol
    // GetPooledTransactions = 0x09, // eth/65 protocol
    // PooledTransactions  = 0x0a, // eth/65 protocol
    //GetNodeData = 0x0d, // ommited it can overburder client.
    //NodeData = 0x0e,    // ommited it can overburder client
    GetReceipts = 0x0f,
    Receipts = 0x10,
}

impl EthMessageId {
    pub fn is_response(&self) -> bool {
        match self {
            Self::BlockHeaders | Self::BlockBodies | Self::Receipts => true,
            _ => false
        }
    }
}

#[derive(FromPrimitive,Debug, Copy,Clone)]
pub enum ParityMessageId {
    // Snapshot related id/s
    GetSnapshotManifest = 0x11,
    SnapshotManifest = 0x12,
    GetSnapshotData = 0x13,
    SnapshotData = 0x14,
    ConsensusData = 0x15,
}


impl ParityMessageId {
    pub fn is_response(&self) -> bool {
        match self {
            Self::SnapshotManifest | Self::SnapshotData => true,
            _ => false
        }
    }
}

#[derive(Debug,Copy,Clone)]
pub enum MessageId {
    Eth(EthMessageId),
    Parity(ParityMessageId),
}

impl MessageId {
    pub fn to_u8(&self) -> u8 {
        match self {
            Self::Eth(msg_id) => *msg_id as u8,
            Self::Parity(msg_id) => *msg_id as u8,
        }
    }
}