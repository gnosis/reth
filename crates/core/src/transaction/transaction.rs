// Copyright 2021 Gnosis Ltd.
// SPDX-License-Identifier: Apache-2.0


use crate::{Address, BlockNumber, Bytes, U256};
use ethereum_types::{H160, H256};
use keccak_hash::keccak;
use rlp::{self, DecoderError, Rlp, RlpStream};
use super::{AccessListData, ChainId, Data, Signature, TransactionBase};
use crypto::publickey::{self, public_to_address, recover, Public, Secret};



pub struct Transaction {
    data: Data,
    signature: Signature,
    chain_id: Option<ChainId>,
    hash: H256,
    signer: Option<(Address,Public)>,
}

impl Transaction {

    pub fn new(data: Data, signature: Signature, chain_id: Option<ChainId>, hash: H256) -> Transaction {
        Transaction {
            data,
            signature,
            chain_id,
            hash,
            signer: None,
        }
    }
}


pub struct InsertedTransaction {
    tx: Transaction,
    block_number: BlockNumber,
    block_hash: H256,
    tx_index: usize,
}


#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CallType {
  CreateContract(),
  CallMessage(Address)
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
