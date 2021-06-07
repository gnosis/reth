// Copyright 2021 Gnosis Ltd.
// SPDX-License-Identifier: Apache-2.0

use super::{
    access_list_payload::{AccessList, AccessListItem, AccessListPayload},
    type_payload::PayloadTrait,
    Signature, Transaction, TxType, TypePayload,
};
use crate::{Address, Keccak};
use ethereum_types::U256;
use keccak_hash::keccak;
use rlp::{DecoderError, Rlp, RlpStream};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default)]

/// eip1559 transaction:
/// 0x02 || rlp([chainId, nonce, maxPriorityFeePerGas, maxFeePerGas, gasLimit, to, value, data, accessList, signatureYParity, signatureR, signatureS]).
pub struct Eip1559Payload {
    pub max_priority_fee_per_gas: U256,
    pub access_list: AccessList,
}

impl PayloadTrait for Eip1559Payload {
    fn encode(tx: &Transaction, for_signature: bool) -> Vec<u8> {
        let data = match tx.type_payload {
            TypePayload::Eip1559(ref data) => data,
            _ => panic!("Wrong type send to AccessList encoding"),
        };
        let mut rlp = RlpStream::new();
        rlp.begin_unbounded_list();
        rlp.append(
            &tx.chain_id
                .expect("ChainId should allways be present in new transaction types"),
        );
        rlp.append(&tx.nonce);
        rlp.append(&data.max_priority_fee_per_gas);
        rlp.append(&tx.gas_price);
        rlp.append(&tx.gas_limit);
        rlp.append(&tx.to);
        rlp.append(&tx.value);
        rlp.append(&tx.data);

        rlp.begin_list(data.access_list.len());
        for access in data.access_list.iter() {
            rlp.begin_list(2);
            rlp.append(access.address());
            rlp.begin_list(access.storage_keys().len());
            for storage_key in access.storage_keys().iter() {
                rlp.append(storage_key);
            }
        }

        if !for_signature {
            tx.signature().rlp_append(&mut rlp);
        }
        rlp.finalize_unbounded_list();
        [&[TxType::AccessList as u8], rlp.as_raw()].concat()
    }

    fn decode(input: &[u8]) -> Result<Transaction, rlp::DecoderError> {
        let rlp = &Rlp::new(&input[1..]);

        if rlp.item_count()? != 11 {
            return Err(DecoderError::RlpIncorrectListLen);
        }

        let chain_id = Some(rlp.val_at(0)?);
        let nonce = rlp.val_at(1)?;
        let max_priority_fee_per_gas = rlp.val_at(2)?;
        let gas_price = rlp.val_at(3)?;
        let gas_limit = rlp.val_at(4)?;
        let to = rlp.val_at(5)?;
        let value = rlp.val_at(6)?;
        let data = rlp.val_at(7)?;
        let access_list_rlp = rlp.at(8)?;

        // access_list pattern: [[{20 bytes}, [{32 bytes}...]]...]
        let mut access_list: AccessList = Vec::new();

        for account in access_list_rlp.iter() {
            // check if there is list of 2 items
            if account.item_count()? != 2 {
                return Err(DecoderError::Custom(
                    "Wrong rlp access list length. We expect two items.",
                ));
            }
            access_list.push(AccessListItem::new(account.val_at(0)?, account.list_at(1)?));
        }

        // we get signature part from here
        let signature = Signature {
            v: rlp.val_at(8)?,
            r: rlp.val_at(9)?,
            s: rlp.val_at(10)?,
        };

        // and here we create UnverifiedTransaction and calculate its hash
        Ok(Transaction::new(
            TypePayload::Eip1559(Eip1559Payload {
                max_priority_fee_per_gas,
                access_list,
            }),
            signature,
            chain_id,
            keccak(input),
            nonce,
            gas_limit,
            gas_price,
            to,
            value,
            data,
        ))
    }
}
