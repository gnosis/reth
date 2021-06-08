// Copyright 2021 Gnosis Ltd.
// SPDX-License-Identifier: Apache-2.0

use super::{
    eip1559_payload::Eip1559Payload, AccessListPayload, LegacyPayload, Transaction, TxType,
};
use crate::Address;
use rlp::{self, DecoderError, Rlp, RlpStream};

pub trait PayloadTrait {
    fn encode(tx: &Transaction, for_signature: bool) -> Vec<u8>;
    fn decode(input: &[u8]) -> Result<Transaction, DecoderError>;
}

/// transaction type specific data and encode decode schemes for every type.
#[derive(Debug, Clone)]
pub enum TypePayload {
    Legacy(LegacyPayload),
    AccessList(AccessListPayload),
    Eip1559(Eip1559Payload),
}

impl TypePayload {
    pub fn txtype(&self) -> TxType {
        match self {
            Self::Legacy(_) => TxType::Legacy,
            Self::AccessList(_) => TxType::AccessList,
            Self::Eip1559(_) => TxType::Eip1559,
        }
    }
}

impl Default for TypePayload {
    fn default() -> TypePayload {
        TypePayload::Legacy(LegacyPayload::default())
    }
}

impl PayloadTrait for TypePayload {
    fn encode(tx: &Transaction, for_signature: bool) -> Vec<u8> {
        match tx.txtype() {
            TxType::Legacy => LegacyPayload::encode(tx, for_signature),
            TxType::AccessList => AccessListPayload::encode(tx, for_signature),
            TxType::Eip1559 => Eip1559Payload::encode(tx, for_signature),
        }
    }

    fn decode(input: &[u8]) -> Result<Transaction, DecoderError> {
        if input.is_empty() {
            return Err(DecoderError::RlpIncorrectListLen);
        }
        let type_byte = input[0];
        //if first bit is `1` it means that we are dealing with rlp list and old legacy transaction
        if (type_byte & 0x80) != 0x00 {
            LegacyPayload::decode(input)
        } else {
            let id = TxType::try_from_wire_byte(type_byte)
                .map_err(|_| DecoderError::Custom("Unknown transaction"))?;
            // other transaction types
            match id {
                TxType::Eip1559 => Eip1559Payload::decode(input),
                TxType::AccessList => AccessListPayload::decode(input),
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

impl Default for CallType {
    fn default() -> Self {
        Self::CreateContract()
    }
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
