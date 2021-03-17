// Copyright 2021 Gnosis Ltd.
// SPDX-License-Identifier: Apache-2.0

use super::{AccessListData, ChainId, LegacyData, Signature, Transaction, TxType};
use crate::Address;
use rlp::{self, DecoderError, Rlp, RlpStream};

pub trait DataTrait {
    fn encode(&self, chain_id: Option<ChainId>, signature: Option<&Signature>) -> Vec<u8>;
    fn decode(input: &[u8]) -> Result<Transaction, DecoderError>;
}

pub enum Data {
    Legacy(LegacyData),
    AccessList(AccessListData),
}

impl DataTrait for Data {
    fn encode(&self, chain_id: Option<ChainId>, signature: Option<&Signature>) -> Vec<u8> {
        match self {
            Self::Legacy(data) => data.encode(chain_id, signature),
            Self::AccessList(data) => data.encode(chain_id, signature),
        }
    }

    fn decode(input: &[u8]) -> Result<Transaction, DecoderError> {
        if input.is_empty() {
            // at least one byte needs to be present
            return Err(DecoderError::RlpIncorrectListLen);
        }
        let header = input[0];
        // type of transaction can be obtained from first byte. If first bit is 1 it means we are dealing with RLP list.
        // if it is 0 it means that we are dealing with custom transaction defined in EIP-2718.
        if (header & 0x80) != 0x00 {
            LegacyData::decode(input)
        } else {
            let id = TxType::try_from_wire_byte(header)
                .map_err(|_| DecoderError::Custom("Unknown transaction"))?;
            // other transaction types
            match id {
                TxType::AccessList => AccessListData::decode(&input[1..]),
                TxType::Legacy => return Err(DecoderError::Custom("Unknown transaction legacy")),
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CallType {
    CreateContract(),
    CallMessage(Address),
}

impl rlp::Decodable for CallType {
    fn decode(rlp: &Rlp) -> Result<Self, DecoderError> {
        if rlp.is_empty() {
            if rlp.is_data() {
                Ok(CallType::CreateContract())
            } else {
                Err(DecoderError::RlpExpectedToBeData)
            }
        } else {
            Ok(CallType::CallMessage(rlp.as_val()?))
        }
    }
}

impl rlp::Encodable for CallType {
    fn rlp_append(&self, s: &mut RlpStream) {
        match *self {
            CallType::CreateContract() => s.append_internal(&""),
            CallType::CallMessage(ref addr) => s.append_internal(addr),
        };
    }
}
