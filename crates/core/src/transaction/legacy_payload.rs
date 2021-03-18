// Copyright 2021 Gnosis Ltd.
// SPDX-License-Identifier: Apache-2.0

use super::{
    signature::replay_protection,
    signature::{SigVLegacy, Signature},
    type_payload::{PayloadTrait, TypePayload},
    Transaction,
};
use crate::U256;
use keccak_hash::keccak;
use rlp::{self, DecoderError, Rlp, RlpStream};

/// A transaction (formally, T) is a
/// single cryptographically-signed instruction constructed by
/// an actor externally to the scope of Ethereum
#[derive(Debug, Clone, Default)]
pub struct LegacyPayload {
    /// The number of Wei to pay the network for unit of gas.
    pub gas_price: U256,
}

impl PayloadTrait for LegacyPayload {
    fn encode(tx: &Transaction, for_signature: bool) -> Vec<u8> {
        let data = match tx.type_payload {
            TypePayload::Legacy(ref data) => data,
            _ => panic!("Wrong type send to LegacyPayload encoding"),
        };
        let mut rlp = RlpStream::new();
        rlp.begin_unbounded_list();

        rlp.append(&tx.nonce);
        rlp.append(&data.gas_price);
        rlp.append(&tx.gas_limit);
        rlp.append(&tx.to);
        rlp.append(&tx.value);
        rlp.append(&tx.data);
        if for_signature {
            if let Some(n) = tx.chain_id() {
                rlp.append(&n);
                rlp.append(&0u8);
                rlp.append(&0u8);
            }
        } else {
            rlp.append(&tx.v());
            rlp.append(&tx.signature().r);
            rlp.append(&tx.signature().s);
        }
        rlp.finalize_unbounded_list();
        rlp.out().to_vec()
    }

    fn decode(input: &[u8]) -> Result<Transaction, DecoderError> {
        let rlp = Rlp::new(&input);
        if rlp.item_count()? != 9 {
            return Err(DecoderError::RlpIncorrectListLen);
        }
        let nonce = rlp.val_at(0)?;
        let gas_price = rlp.val_at(1)?;
        let gas_limit = rlp.val_at(2)?;
        let to = rlp.val_at(3)?;
        let value = rlp.val_at(4)?;
        let data = rlp.val_at(5)?;
        let mixed_v: SigVLegacy = rlp.val_at(6)?;
        let r = rlp.val_at(7)?;
        let s = rlp.val_at(8)?;
        let v = replay_protection::decode_v(mixed_v);
        let signature = Signature::new(v, r, s);
        let chain_id = replay_protection::decode_chain_id(mixed_v);

        Ok(Transaction::new(
            TypePayload::Legacy(LegacyPayload { gas_price }),
            signature,
            chain_id,
            keccak(input),
            nonce,
            gas_limit,
            to,
            value,
            data,
        ))
    }
}
