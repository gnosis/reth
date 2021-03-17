// Copyright 2021 Gnosis Ltd.
// SPDX-License-Identifier: Apache-2.0
use self::data::DataTrait;

pub use super::{signature::replay_protection, *};
use crate::{Bytes, U256};
use keccak_hash::keccak;
use rlp::{self, DecoderError, Rlp, RlpStream};

/// A transaction (formally, T) is a
/// single cryptographically-signed instruction constructed by
/// an actor externally to the scope of Ethereum
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LegacyData {
    /// The number of transactions sent by the sender.
    pub nonce: U256,
    /// The number of Wei to pay the network for unit of gas.
    pub gas_price: U256,
    /// The maximum amount of gas to be used in while executing a transaction
    pub gas_limit: U256,
    /// The 20-character recipient of a message call. In the case of a contract creation this is 0x000000000000000000
    pub call_type: CallType,
    /// The number of Wei to be transferred to the recipient of a message call.
    pub value: U256,
    /// Byte array specifying the input data of the message call or
    /// for contract creation:  EVM-code for the account initialisation procedure
    pub data: Bytes,
}

impl DataTrait for LegacyData {
    fn encode(&self, chain_id: Option<ChainId>, signature: Option<&Signature>) -> Vec<u8> {
        let mut stream = RlpStream::new();
        self.rlp(&mut stream, chain_id, signature);
        stream.out().to_vec()
    }

    fn decode(input: &[u8]) -> Result<Transaction, DecoderError> {
        let rlp = Rlp::new(input);
        if rlp.item_count()? != 9 {
            return Err(DecoderError::RlpIncorrectListLen);
        }
        let hash = keccak(rlp.as_raw());
        let data = Data::Legacy(Self::decode_data(&rlp, 0)?);
        let mixed_v: SigVLegacy = rlp.val_at(6)?;
        let r = rlp.val_at(7)?;
        let s = rlp.val_at(8)?;
        let v = replay_protection::decode_v(mixed_v);
        let chain_id = replay_protection::decode_chain_id(mixed_v);

        Ok(Transaction::new(
            data,
            Signature { v, r, s },
            chain_id,
            hash,
        ))
    }
}

impl LegacyData {
    pub fn rlp_append(&self, rlp: &mut RlpStream, chain_id: Option<u64>, signature: &Signature) {
        self.rlp(rlp, chain_id, Some(signature));
    }

    fn rlp(&self, rlp: &mut RlpStream, chain_id: Option<u64>, signature: Option<&Signature>) {
        rlp.begin_unbounded_list();
        self.rlp_append_fields(rlp);
        if let Some(sig) = signature {
            rlp.append(&replay_protection::encode(sig.v, chain_id));
            rlp.append(&sig.r);
            rlp.append(&sig.s);
        } else {
            if let Some(n) = chain_id {
                rlp.append(&n);
                rlp.append(&0u8);
                rlp.append(&0u8);
            }
        }
        rlp.finalize_unbounded_list();
    }

    fn rlp_append_fields(&self, s: &mut RlpStream) {
        s.append(&self.nonce);
        s.append(&self.gas_price);
        s.append(&self.gas_limit);
        s.append(&self.call_type);
        s.append(&self.value);
        s.append(&self.data);
    }

    fn decode_data(d: &Rlp, offset: usize) -> Result<LegacyData, DecoderError> {
        Ok(LegacyData {
            nonce: d.val_at(offset)?,
            gas_price: d.val_at(offset + 1)?,
            gas_limit: d.val_at(offset + 2)?,
            call_type: d.val_at(offset + 3)?,
            value: d.val_at(offset + 4)?,
            data: d.val_at(offset + 5)?,
        })
    }
}
