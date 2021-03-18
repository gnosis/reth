// Copyright 2021 Gnosis Ltd.
// SPDX-License-Identifier: Apache-2.0
use super::{type_payload::PayloadTrait, LegacyPayload, Signature, Transaction, TxType, TypePayload};
use crate::{Address, Keccak};
use keccak_hash::keccak;
use rlp::{DecoderError, Rlp, RlpStream};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default)]
pub struct AccessListPayload {
    pub legacy_data: LegacyPayload,
    pub access_list: AccessList,
}

pub type AccessList = Vec<AccessListItem>;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccessListItem {
    address: Address,
    storage_keys: Vec<Keccak>,
}

impl AccessListItem {
    pub fn new(address: Address, storage_keys: Vec<Keccak>) -> Self {
        Self {
            address,
            storage_keys,
        }
    }
}

impl PayloadTrait for AccessListPayload {
    fn encode(tx: &Transaction, for_signature: bool) -> Vec<u8> {
        let data = match tx.type_payload {
            TypePayload::AccessList(ref data) => data,
            _ => panic!("Wrong type send to AccessList encoding"),
        };
        let mut rlp = RlpStream::new();
        rlp.begin_unbounded_list();
        rlp.append(
            &tx.chain_id()
                .expect("ChainId should allways be present in new transaction types"),
        );
        rlp.append(&tx.nonce);
        rlp.append(&data.legacy_data.gas_price);
        rlp.append(&tx.gas_limit);
        rlp.append(&tx.to);
        rlp.append(&tx.value);
        rlp.append(&tx.data);

        rlp.begin_list(data.access_list.len());
        for access in data.access_list.iter() {
            rlp.begin_list(2);
            rlp.append(&access.address);
            rlp.begin_list(access.storage_keys.len());
            for storage_key in access.storage_keys.iter() {
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
        let gas_price = rlp.val_at(2)?;
        let gas_limit = rlp.val_at(3)?;
        let to = rlp.val_at(4)?;
        let value = rlp.val_at(5)?;
        let data = rlp.val_at(6)?;
        let access_list_rlp = rlp.at(7)?;

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
            TypePayload::AccessList(AccessListPayload {
                legacy_data: LegacyPayload { gas_price },
                access_list,
            }),
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
