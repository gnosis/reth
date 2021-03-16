// Copyright 2021 Gnosis Ltd.
// SPDX-License-Identifier: Apache-2.0


use crate::{Address, BlockNumber, Bytes, U256};
use ethereum_types::{H160, H256};
use keccak_hash::keccak;
use rlp::{self, DecoderError, Rlp, RlpStream};
use super::{ChainId,Signature,AccessListData,TransactionBase,Transaction};
use crypto::publickey::{self, public_to_address, recover, Public, Secret};

pub trait TransactionTypeTrait {
    fn encode(&self, chain_id: Option<ChainId>, signature: Option<&Signature>) -> Vec<u8>;
    fn decode(rlp: &Rlp) -> Result<Transaction, DecoderError>;
}

pub enum Data {
    Legacy(TransactionBase),
    AccessList(AccessListData),
}

