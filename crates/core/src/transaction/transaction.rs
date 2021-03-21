// Copyright 2021 Gnosis Ltd.
// SPDX-License-Identifier: Apache-2.0

//! This module defines data (as opposed to information) defined in
//! the Ethereum Yellow Paper:
//!
//! Wood, Gavin. "Ethereum: A secure decentralised generalised transaction ledger."
//! Ethereum project yellow paper PETERSBURG VERSION 41c1837 – 2021-02-14
//!
//! Types in this module reconstruct raw definitions of the ethereum protocol
//! and aim to be wire-protocol compatible. All interpretations of those data
//! in form of information happen at higher levels of abstraction.
//!~
//! An example of the difference between data and information is the "to" field
//! in a transaction. The address where the transactions is sent is data, whereas
//! the interpretation whether the address is creation/call is information.
//!
//! Entities in this module are serializable to various formats as is without
//! any further changes.

use std::error::Error;

use crate::{verifiable::Verifiable, Bytes, H256, U256, U64};
use crypto::publickey::{self, Secret};
use ethereum_types::Address;
use keccak_hash::keccak;

use serde::{Deserialize, Serialize};

use super::{Author, Signature};

/// Ethereum Yellow Paper 4.2.
///
/// A transaction (formally, T) is a single cryptographically-signed instruction
/// constructed by an actor externally to the scope of Ethereum.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Transaction {
    /// The number of transactions sent by the sender.
    pub nonce: U64,

    /// A scalar value equal to the number of Wei to be paid per unit of gas for
    /// all computation costs incurred as a result of the execution of this transaction;
    /// formally Tp.
    pub gas_price: U256,

    /// A scalar value equal to the maximum amount of gas that should be used
    /// in executing this transaction. This is paid up-front, before any computation
    /// is done and may not be increased later; formally Tg.
    pub gas_limit: U256,

    /// The 160-bit (20 character) address of the message call’s recipient or,
    /// for a contract creation transaction, 0; formally Tt.
    pub to: Address,

    /// A scalar value equal to the number of Wei to be transferred to the
    /// message call’s recipient or, in the case of contract creation,
    /// as an endowment to the newly created account; formally Tv.
    pub value: U256,

    /// An unlimited size byte array specifying the input data of the message call, formally Td.
    /// In case of contract creation: data is an EVM-code fragment; it returns the body,
    /// a second fragment of code that executes each time the account receives a message
    /// call (either through a trans- action or due to the internal execution of code).
    /// init is executed only once at account creation and gets discarded immediately thereafter.
    pub data: Bytes,

    /// v, r, s: Values corresponding to the signature of the transaction and
    /// used to determine the sender of the transaction; formally Tw, Tr and Ts.
    pub signature: Signature,
}

type Result<T> = std::result::Result<T, Box<dyn Error>>;

impl Transaction {
    pub fn compute_signature(&self, secret: &Secret) -> Result<Signature> {
        Ok(publickey::sign(secret, &self.hash()?)
            .expect("Expecting valid data and signing of transaction to pass")
            .into())
    }

    pub fn hash(&self) -> Result<H256> {
        Ok(keccak(serde_rlp::serialize(&self)?))
    }

    pub fn chain_id(&self) -> u64 {
        todo!() // extract from V in VSR signature
    }
}

impl Verifiable<Secret> for Transaction {
    fn valid(&self, identity: &Secret) -> Result<bool> {
        Ok(Signature::for_transaction(&self, identity)? == self.signature)
    }
}

impl Transaction {
    pub fn author(&self) -> Result<Author> {
        Ok(self.signature.recover_author(&self.hash()?)?)
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;
    // use super::{
    //     super::{access_list_payload::*, legacy_payload::*},
    //     *,
    // };
    // use crypto::publickey::{Generator, Public};
    // use ethereum_types::{Address, H160, H512, U256, U64};
    // use rlp::{Rlp, RlpStream};
    use crypto::publickey::Public;
    use rustc_hex::{FromHex, ToHex};
    // use std::{error::Error, str::FromStr};

    // /// Dummy address defined in EIP-86.
    // pub const DUMMY_AUTHOR: (Address, Public) = (H160([0xff; 20]), H512([0xff; 64]));

    // /// Legacy EIP-86 compatible empty signature.
    // /// It is used in json tests
    // pub fn null_sign(mut tx: Transaction) -> Transaction {
    //     tx.signature = Signature::new(0, U256::zero(), U256::zero());
    //     tx.author = Some(DUMMY_AUTHOR);
    //     tx.recompute_hash();
    //     tx
    // }

    // pub fn fake_sign(mut tx: Transaction, author: Address) -> Transaction {
    //     tx.signature = Signature::new(4, U256::one(), U256::one());
    //     tx.author = Some((author, H512::zero()));
    //     tx.recompute_hash();
    //     tx
    // }

    #[test]
    fn sanity_ser_de() -> Result<()> {
        let tx = Transaction::default();
        let hash_original = tx.hash()?;
        let tx_bytes = serde_rlp::serialize(&tx)?;
        println!("txbytes: {:?}", tx_bytes[..].to_hex::<String>());
        let tx_revived: Transaction = serde_rlp::deserialize(&tx_bytes)?;
        assert_eq!(hash_original, tx_revived.hash()?);
        Ok(())
    }

    #[test]
    fn decode_real_rx() -> Result<()> {
        let raw: Vec<u8> = "f88b8212b085028fa6ae00830f424094aad593da0c8116ef7d2d594dd6a63241bccfc26c80a48318b64b000000000000000000000000641c5d790f862a58ec7abcfd644c0442e9c201b32aa0a6ef9e170bca5ffb7ac05433b13b7043de667fbb0b4a5e45d3b54fb2d6efcc63a0037ec2c05c3d60c5f5f78244ce0a3859e3a18a36c61efb061b383507d3ce19d2".from_hex().unwrap();
        let tx: Transaction = serde_rlp::deserialize(&raw)?;
        assert_eq!(
            tx.hash()?,
            H256::from_str("559fb34c4a7f115db26cbf8505389475caaab3df45f5c7a0faa4abfa3835306c")?
        );

        assert_eq!(tx.value, U256::from_str("0")?);
        assert_eq!(tx.nonce, U64::from_str("12b0").unwrap());
        assert_eq!(tx.gas_limit, U256::from_str("f4240").unwrap());
        assert_eq!(tx.to, Address::from_str("aad593da0c8116ef7d2d594dd6a63241bccfc26c")?);
        assert_eq!(tx.signature.v as u64, U64::from_str("2a")?.as_u64());
        assert_eq!(
            tx.signature.s,
            U256::from_str("37ec2c05c3d60c5f5f78244ce0a3859e3a18a36c61efb061b383507d3ce19d2")?
        );
        assert_eq!(
            tx.signature.r,
            U256::from_str("a6ef9e170bca5ffb7ac05433b13b7043de667fbb0b4a5e45d3b54fb2d6efcc63")?
        );

        assert_eq!(
            tx.author()?.1, 
            Public::from_str("695ee214d90789c0ff826ff97a7139b8f309a84336a5e6b136c8a0702a86e624b98aa17f3611e5c22cc0c792c578be40854a7ce60d5bdfe1b1fc175a4c74c5ea")?);

        assert_eq!(
            tx.data,
            "8318b64b000000000000000000000000641c5d790f862a58ec7abcfd644c0442e9c201b3"
                .from_hex::<Vec<u8>>()?
        );


        Ok(())
    }

    // #[test]
    // fn default_access_list_en_de() {
    //     let mut tx = Transaction::default();
    //     tx.chain_id = Some(100);
    //     tx.type_payload = TypePayload::AccessList(AccessListPayload::default());

    //     let mut tx = null_sign(tx);
    //     tx.recompute_hash();
    //     let hash_original = tx.hash();
    //     let tx_bytes = tx.encode();
    //     let mut tx_revived = Transaction::decode(&tx_bytes).expect("Expect decode to pass");
    //     tx_revived.recompute_hash();
    //     let new_hash = tx_revived.hash();
    //     assert_eq!(hash_original, new_hash);
    // }

    // fn null_signed_dummy_legacy_tx() -> Transaction {
    //     let tx = Transaction {
    //         type_payload: TypePayload::Legacy(LegacyPayload {
    //             gas_price: 15.into(),
    //         }),
    //         nonce: 10.into(),
    //         gas_limit: 20.into(),
    //         to: CallType::CallMessage(Address::from_low_u64_be(30)),
    //         value: 50.into(),
    //         data: vec![0x11, 0x22, 0x33],
    //         signature: Signature::default(),
    //         chain_id: Some(10),
    //         hash: H256::zero(),
    //         author: None,
    //     };

    //     null_sign(tx)
    // }

    // fn null_signed_dummy_access_list_tx() -> Transaction {
    //     let type_payload = TypePayload::AccessList(AccessListPayload {
    //         legacy_payload: LegacyPayload {
    //             gas_price: U256::from(10),
    //         },
    //         access_list: vec![AccessListItem::new(
    //             Address::from_low_u64_be(10),
    //             vec![H256::from_low_u64_be(30), H256::from_low_u64_be(500)],
    //         )],
    //     });

    //     let tx = Transaction {
    //         type_payload,
    //         nonce: 100.into(),
    //         gas_limit: 200.into(),
    //         to: CallType::CallMessage(Address::from_low_u64_be(300)),
    //         value: 500.into(),
    //         data: vec![0x11, 0x22, 0x33, 0x44, 0x55],
    //         signature: Signature::default(),
    //         chain_id: Some(10),
    //         hash: H256::zero(),
    //         author: None,
    //     };

    //     null_sign(tx)
    // }

    // #[test]
    // fn should_encode_decode_vec_tx() {
    //     let dummy = null_signed_dummy_legacy_tx();
    //     let dumm_ac = null_signed_dummy_access_list_tx();
    //     let vecd = vec![dummy.clone(), dumm_ac.clone(), dummy.clone()];

    //     let mut rlp = RlpStream::new();
    //     Transaction::rlp_append_list(&mut rlp, &vecd);
    //     let output : Vec<u8> = "f8cae00a0f1494000000000000000000000000000000000000001e3283112233378080b88601f8830a640a81c894000000000000000000000000000000000000012c8201f4851122334455f85bf85994000000000000000000000000000000000000000af842a0000000000000000000000000000000000000000000000000000000000000001ea000000000000000000000000000000000000000000000000000000000000001f4808080e00a0f1494000000000000000000000000000000000000001e3283112233378080".from_hex().unwrap();
    //     assert_eq!(rlp.as_raw(), output);
    //     let out_vecd = Transaction::rlp_decode_list(&Rlp::new(&output)).unwrap();
    //     assert_eq!(out_vecd.len(), 3);
    // }

    // #[test]
    // fn should_sign() {
    //     let keypair = crypto::publickey::Random.generate();

    //     let mut tx = Transaction {
    //         type_payload: TypePayload::Legacy(LegacyPayload {
    //             gas_price: U256::from(4000),
    //         }),
    //         to: CallType::CreateContract(),
    //         nonce: U64::from(42),
    //         gas_limit: U256::from(60_000),
    //         value: U256::from(10),
    //         data: b"Hello World!".to_vec(),
    //         signature: Signature::default(),
    //         chain_id: None,
    //         hash: H256::zero(),
    //         author: None,
    //     };
    //     tx.sign(keypair.secret());

    //     assert_eq!(
    //         Address::from(keccak(keypair.public())),
    //         tx.author().unwrap().0
    //     );
    //     assert_eq!(*keypair.public(), tx.author().unwrap().1);
    //     assert_eq!(tx.chain_id, None);
    // }

    // #[test]
    // fn should_sign_chain_id() {
    //     let keypair = crypto::publickey::Random.generate();

    //     let mut tx = Transaction {
    //         type_payload: TypePayload::Legacy(LegacyPayload {
    //             gas_price: U256::from(4000),
    //         }),
    //         to: CallType::CreateContract(),
    //         nonce: U64::from(42),
    //         gas_limit: U256::from(60_000),
    //         value: U256::from(10),
    //         data: b"Hello World!".to_vec(),
    //         signature: Signature::default(),
    //         chain_id: None,
    //         hash: H256::zero(),
    //         author: None,
    //     };
    //     tx.sign(keypair.secret());
    //     assert_eq!(
    //         Address::from(keccak(keypair.public())),
    //         tx.author().unwrap().0
    //     );
    //     assert_eq!(*keypair.public(), tx.author().unwrap().1);
    //     assert_eq!(tx.chain_id, None);
    // }

    // #[test]
    // fn should_sign_access_list() {
    //     let keypair = crypto::publickey::Random.generate();

    //     let type_payload = TypePayload::AccessList(AccessListPayload {
    //         legacy_payload: LegacyPayload {
    //             gas_price: U256::from(10),
    //         },
    //         access_list: vec![AccessListItem::new(
    //             Address::from_low_u64_be(10),
    //             vec![H256::from_low_u64_be(30), H256::from_low_u64_be(500)],
    //         )],
    //     });
    //     let mut tx = Transaction {
    //         type_payload,
    //         to: CallType::CreateContract(),
    //         nonce: U64::from(42),
    //         gas_limit: U256::from(60_000),
    //         value: U256::from(10),
    //         data: b"Hello World!".to_vec(),
    //         signature: Signature::default(),
    //         chain_id: Some(10),
    //         hash: H256::zero(),
    //         author: None,
    //     };
    //     tx.sign(keypair.secret());

    //     assert_eq!(
    //         Address::from(keccak(keypair.public())),
    //         tx.author().unwrap().0
    //     );
    //     assert_eq!(*keypair.public(), tx.author().unwrap().1);
    //     assert_eq!(tx.chain_id, Some(10));
    // }

    // #[test]
    // fn decode_real_legacy_tx_and_check_hash() {
    //     // transaction is from ropsten
    //     let raw: Vec<u8> = "f88b8212b085028fa6ae00830f424094aad593da0c8116ef7d2d594dd6a63241bccfc26c80a48318b64b000000000000000000000000641c5d790f862a58ec7abcfd644c0442e9c201b32aa0a6ef9e170bca5ffb7ac05433b13b7043de667fbb0b4a5e45d3b54fb2d6efcc63a0037ec2c05c3d60c5f5f78244ce0a3859e3a18a36c61efb061b383507d3ce19d2".from_hex().unwrap();
    //     let mut tx = Transaction::decode(&raw).unwrap();
    //     tx.recompute_hash();
    //     tx.recover_author().unwrap();
    // assert_eq!(
    //     tx.hash(),
    //     H256::from_str("559fb34c4a7f115db26cbf8505389475caaab3df45f5c7a0faa4abfa3835306c")
    //         .unwrap()
    // );
    //     assert_eq!(
    //         tx.author().unwrap().0,
    //         H160::from_str("641c5d790f862a58ec7abcfd644c0442e9c201b3").unwrap()
    //     );
    //     assert_eq!(tx.chain_id, Some(3));
    //     assert_eq!(tx.v(), U64::from_str("2a").unwrap().as_u64());
    //     assert_eq!(
    //         tx.signature.s,
    //         U256::from_str("37ec2c05c3d60c5f5f78244ce0a3859e3a18a36c61efb061b383507d3ce19d2")
    //             .unwrap()
    //     );
    //     assert_eq!(
    //         tx.signature.r,
    //         U256::from_str("a6ef9e170bca5ffb7ac05433b13b7043de667fbb0b4a5e45d3b54fb2d6efcc63")
    //             .unwrap()
    //     );
    //     assert_eq!(tx.gas_limit, U256::from_str("f4240").unwrap());
    //     assert_eq!(tx.nonce, U64::from_str("12b0").unwrap());
    //     assert_eq!(tx.author().unwrap().1, Public::from_str("695ee214d90789c0ff826ff97a7139b8f309a84336a5e6b136c8a0702a86e624b98aa17f3611e5c22cc0c792c578be40854a7ce60d5bdfe1b1fc175a4c74c5ea").unwrap());
    //     assert_eq!(
    //         tx.data,
    //         "8318b64b000000000000000000000000641c5d790f862a58ec7abcfd644c0442e9c201b3"
    //             .from_hex::<Vec<u8>>()
    //             .unwrap()
    //     );
    //     if let TypePayload::Legacy(LegacyPayload { gas_price }) = tx.type_payload {
    //         assert_eq!(gas_price, U256::from_str("28fa6ae00").unwrap())
    //     } else {
    //         panic!("it is not legacy");
    //     }
    //     let t = CallType::CallMessage(
    //         H160::from_str("aad593da0c8116ef7d2d594dd6a63241bccfc26c").unwrap(),
    //     );
    //     assert_eq!(tx.to, t);
    //     assert_eq!(tx.value, U256::from_str("0").unwrap());
    // }

    // #[test]
    // fn decode_real_access_list_tx_and_check_hash() {
    //     //transaction is from goerli
    //     let raw: Vec<u8> = "01f8c20502843b9aca00830186a094e13ece23b514caa5b53395c01d0d53d1843258ad6480f85bf859940000000000000000000000000000000000000101f842a00000000000000000000000000000000000000000000000000000000000000000a000000000000000000000000000000000000000000000000000000000000060a780a0c478405180948befd7d603eba34dfe689c52c57debb870cd7753431b28e27d73a01ab5b2b744cfce239169a0dd8fe2b77a0c49cff267368bdccc47a3745c1ecdb1".from_hex().unwrap();
    //     let mut tx = Transaction::decode(&raw).unwrap();
    //     tx.recompute_hash();
    //     tx.recover_author().unwrap();
    //     assert_eq!(
    //         tx.hash(),
    //         H256::from_str("e887e4fc88b76d635ea7b2b4db6d960b8a7cabb7a2dba6872d6bfebf3486bb51")
    //             .unwrap()
    //     );
    //     assert_eq!(
    //         tx.author().unwrap().0,
    //         H160::from_str("aaec86394441f915bce3e6ab399977e9906f3b69").unwrap()
    //     );
    //     assert_eq!(tx.author().unwrap().1, Public::from_str("91201f5b4d7739ce3030e17779e7b2ad5190cb3d61639bcd40ba24e098df7567e834b483858ccda7ff9a760b6da7adbe126d2e04b9b3ca7e071b43d846cd2378").unwrap());
    //     assert_eq!(tx.chain_id, Some(5));
    //     assert_eq!(tx.v(), U64::from_str("0").unwrap().as_u64());
    //     assert_eq!(
    //         tx.signature.s,
    //         U256::from_str("1ab5b2b744cfce239169a0dd8fe2b77a0c49cff267368bdccc47a3745c1ecdb1")
    //             .unwrap()
    //     );
    //     assert_eq!(
    //         tx.signature.r,
    //         U256::from_str("c478405180948befd7d603eba34dfe689c52c57debb870cd7753431b28e27d73")
    //             .unwrap()
    //     );
    //     assert_eq!(tx.gas_limit, U256::from_str("186a0").unwrap());
    //     assert_eq!(tx.nonce, U64::from_str("2").unwrap());
    //     assert_eq!(tx.data, vec![]);
    //     if let TypePayload::AccessList(AccessListPayload {
    //         legacy_payload,
    //         access_list,
    //     }) = tx.type_payload
    //     {
    //         assert_eq!(
    //             legacy_payload.gas_price,
    //             U256::from_str("3b9aca00").unwrap()
    //         );
    //         assert_eq!(access_list.len(), 1);
    //         let ref item = access_list[0];
    //         assert_eq!(item.storage_keys().len(), 2);
    //         assert_eq!(
    //             *item.address(),
    //             Address::from_str("0000000000000000000000000000000000000101").unwrap()
    //         );
    //         let storage_keys = item.storage_keys();
    //         assert_eq!(
    //             storage_keys[0],
    //             H256::from_str("0000000000000000000000000000000000000000000000000000000000000000")
    //                 .unwrap()
    //         );
    //         assert_eq!(
    //             storage_keys[1],
    //             H256::from_str("00000000000000000000000000000000000000000000000000000000000060a7")
    //                 .unwrap()
    //         );
    //     } else {
    //         panic!("it is not access list type");
    //     }
    //     let t = CallType::CallMessage(
    //         H160::from_str("e13ece23b514caa5b53395c01d0d53d1843258ad").unwrap(),
    //     );
    //     assert_eq!(tx.to, t);
    //     assert_eq!(tx.value, U256::from_str("64").unwrap());
    // }
}
