// Copyright 2021 Gnosis Ltd.
// SPDX-License-Identifier: Apache-2.0
use super::{
    signature::replay_protection, type_payload::PayloadTrait, Author, CallType, Signature, TxType,
    TypePayload,
};
use crate::{Bytes, H256, U256, U64};
use crypto::publickey::{self, Secret};
use keccak_hash::keccak;
use rlp::DecoderError;

pub type ChainId = u64;

/// A transaction (formally, T) is a
/// single cryptographically-signed instruction constructed by
/// an actor externally to the scope of Ethereum
#[derive(Debug, Clone, Default)]
pub struct Transaction {
    /// specific data related to type. In future if some of field from standard transaction are removed
    /// it needs to be moved to TypePayload for support for older tx.
    pub type_payload: TypePayload,
    /// signature of transaction
    signature: Signature,
    /// replay protected chain_id
    chain_id: Option<ChainId>,
    /// hash of transaction
    hash: H256,
    /// extracted public key from signature.
    author: Option<Author>,
    /// The number of transactions sent by the sender.
    pub nonce: U64,
    /// The maximum amount of gas to be used in while executing a transaction
    pub gas_limit: U256,
    /// The 20-character recipient of a message call. In the case of a contract creation this is 0x000000000000000000
    pub to: CallType,
    /// The number of Wei to be transferred to the recipient of a message call.
    pub value: U256,
    /// Byte array specifying the input data of the message call or
    /// for contract creation:  EVM-code for the account initialisation procedure
    pub data: Bytes,
}

impl Transaction {
    pub fn new(
        type_payload: TypePayload,
        signature: Signature,
        chain_id: Option<ChainId>,
        hash: H256,
        nonce: U64,
        gas_limit: U256,
        to: CallType,
        value: U256,
        data: Bytes,
    ) -> Transaction {
        Transaction {
            type_payload,
            signature,
            chain_id,
            hash,
            nonce,
            author: None,
            gas_limit,
            to,
            value,
            data,
        }
    }

    pub fn sign(&mut self, secret: &Secret) {
        let signature_hash = keccak(TypePayload::encode(self, true));
        let sig: Signature = publickey::sign(secret, &signature_hash)
            .expect("Expecting valid data and signing of transaction to pass")
            .into();
        let author = sig
            .recover_author(&signature_hash)
            .expect("Expect to recover author successfully");

        self.signature = sig;
        self.author = Some(author);
        self.recompute_hash();
    }

    /// If we want to delay calculating of hash we can send invalid hash
    /// in constructor and use this function to calculate it when we see fit.
    fn recompute_hash(&mut self) {
        self.hash = keccak(&*TypePayload::encode(self, true));
    }

    pub fn author(&self) -> Option<Author> {
        self.author
    }

    pub fn type_payload(&self) -> &TypePayload {
        &self.type_payload
    }

    pub fn chain_id(&self) -> Option<ChainId> {
        self.chain_id
    }
    pub fn signature(&self) -> &Signature {
        &self.signature
    }
    pub fn hash(&self) -> H256 {
        self.hash
    }

    pub fn txtype(&self) -> TxType {
        self.type_payload.txtype()
    }

    /// V from signature that is received from wire in RLP.
    /// For legacy it contains V with replay protected chain_id.
    /// For new transaction types it is ordinary V field from signature.
    pub fn v(&self) -> u64 {
        match self.type_payload.txtype() {
            TxType::Legacy => replay_protection::encode(self.signature.v, self.chain_id),
            _ => self.signature.v as u64,
        }
    }

    pub fn recover_author(&mut self) -> Result<(), publickey::Error> {
        let signature_hash = keccak(TypePayload::encode(self, true));
        self.author = Some(self.signature.recover_author(&signature_hash)?);
        Ok(())
    }

    pub fn has_author(&self) -> bool {
        self.author.is_some()
    }

    // Encode decode functions
    pub fn encode(&self) -> Vec<u8> {
        TypePayload::encode(self, false)
    }

    pub fn decode(input: &[u8]) -> Result<Transaction, DecoderError> {
        TypePayload::decode(input)
    }

    pub fn rlp_append_list(rlp: &mut rlp::RlpStream, txs: &[Transaction]) {
        rlp.begin_list(txs.len());
        for tx in txs {
            let data = tx.encode();
            match tx.txtype() {
                TxType::Legacy => rlp.append_raw(&data, 1),
                TxType::AccessList => rlp.append(&data),
            };
        }
    }

    pub fn rlp_decode_list(rlp: &rlp::Rlp) -> Result<Vec<Transaction>, DecoderError> {
        if !rlp.is_list() {
            return Err(DecoderError::RlpIncorrectListLen);
        }
        let mut decoded = Vec::with_capacity(rlp.item_count()?);
        for tx in rlp.iter() {
            let tx = if tx.is_list() {
                TypePayload::decode(tx.as_raw())?
            } else {
                //this means it is wrapped bytes and we are extracting data that ignores rlp header.
                TypePayload::decode(tx.data()?)?
            };
            decoded.push(tx)
        }
        Ok(decoded)
    }
}

/// After transaction is inserter it contains index and block information.
/* Leave it her for now but this type
#[warn(dead_code)]
pub struct InsertedTransaction {
    tx: Transaction,
    block_number: BlockNumber,
    block_hash: H256,
    tx_index: usize,
}*/

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::super::{access_list_payload::*, legacy_payload::*};
    use super::*;
    use crypto::publickey::{Generator, Public};
    use ethereum_types::{Address, H160, H512, U256, U64};
    use rlp::{Rlp, RlpStream};
    use rustc_hex::{FromHex, ToHex};

    /// Dummy address defined in EIP-86.
    pub const DUMMY_AUTHOR: (Address, Public) = (H160([0xff; 20]), H512([0xff; 64]));

    /// Legacy EIP-86 compatible empty signature.
    /// It is used in json tests
    pub fn null_sign(mut tx: Transaction) -> Transaction {
        tx.signature = Signature::new(0, U256::zero(), U256::zero());
        tx.author = Some(DUMMY_AUTHOR);
        tx.recompute_hash();
        tx
    }

    pub fn fake_sign(mut tx: Transaction, author: Address) -> Transaction {
        tx.signature = Signature::new(4, U256::one(), U256::one());
        tx.author = Some((author, H512::zero()));
        tx.recompute_hash();
        tx
    }

    #[test]
    fn default_legacy_en_de() {
        let tx = Transaction::default();
        let tx = null_sign(tx);

        let hash_original = tx.hash();
        let tx_bytes = tx.encode();
        let mut tx_revived = Transaction::decode(&tx_bytes).expect("Expect decode to pass");
        tx_revived.recompute_hash();
        let new_hash = tx_revived.hash();
        assert_eq!(hash_original, new_hash);
    }

    #[test]
    fn default_access_list_en_de() {
        let mut tx = Transaction::default();
        tx.chain_id = Some(100);
        tx.type_payload = TypePayload::AccessList(AccessListPayload::default());

        let mut tx = null_sign(tx);
        tx.recompute_hash();
        let hash_original = tx.hash();
        let tx_bytes = tx.encode();
        let mut tx_revived = Transaction::decode(&tx_bytes).expect("Expect decode to pass");
        tx_revived.recompute_hash();
        let new_hash = tx_revived.hash();
        assert_eq!(hash_original, new_hash);
    }

    fn null_signed_dummy_legacy_tx() -> Transaction {
        let tx = Transaction {
            type_payload: TypePayload::Legacy(LegacyPayload {
                gas_price: 15.into(),
            }),
            nonce: 10.into(),
            gas_limit: 20.into(),
            to: CallType::CallMessage(Address::from_low_u64_be(30)),
            value: 50.into(),
            data: vec![0x11, 0x22, 0x33],
            signature: Signature::default(),
            chain_id: Some(10),
            hash: H256::zero(),
            author: None,
        };

        null_sign(tx)
    }

    fn null_signed_dummy_access_list_tx() -> Transaction {
        let type_payload = TypePayload::AccessList(AccessListPayload {
            legacy_data: LegacyPayload {
                gas_price: U256::from(10),
            },
            access_list: vec![AccessListItem::new(
                Address::from_low_u64_be(10),
                vec![H256::from_low_u64_be(30), H256::from_low_u64_be(500)],
            )],
        });

        let tx = Transaction {
            type_payload,
            nonce: 100.into(),
            gas_limit: 200.into(),
            to: CallType::CallMessage(Address::from_low_u64_be(300)),
            value: 500.into(),
            data: vec![0x11, 0x22, 0x33, 0x44, 0x55],
            signature: Signature::default(),
            chain_id: Some(10),
            hash: H256::zero(),
            author: None,
        };

        null_sign(tx)
    }

    #[test]
    fn should_encode_decode_vec_tx() {
        let dummy = null_signed_dummy_legacy_tx();
        let dumm_ac = null_signed_dummy_access_list_tx();
        let vecd = vec![dummy.clone(), dumm_ac.clone(), dummy.clone()];

        let mut rlp = RlpStream::new();
        Transaction::rlp_append_list(&mut rlp, &vecd);
        let output : Vec<u8> = "f8cae00a0f1494000000000000000000000000000000000000001e3283112233378080b88601f8830a640a81c894000000000000000000000000000000000000012c8201f4851122334455f85bf85994000000000000000000000000000000000000000af842a0000000000000000000000000000000000000000000000000000000000000001ea000000000000000000000000000000000000000000000000000000000000001f4808080e00a0f1494000000000000000000000000000000000000001e3283112233378080".from_hex().unwrap();
        assert_eq!(rlp.as_raw(), output);
        let out_vecd = Transaction::rlp_decode_list(&Rlp::new(&output)).unwrap();
        assert_eq!(out_vecd.len(), 3);
    }

    #[test]
    fn should_sign() {
        let keypair = crypto::publickey::Random.generate();

        let mut tx = Transaction {
            type_payload: TypePayload::Legacy(LegacyPayload {
                gas_price: U256::from(4000),
            }),
            to: CallType::CreateContract(),
            nonce: U64::from(42),
            gas_limit: U256::from(60_000),
            value: U256::from(10),
            data: b"Hello World!".to_vec(),
            signature: Signature::default(),
            chain_id: None,
            hash: H256::zero(),
            author: None,
        };
        tx.sign(keypair.secret());

        assert_eq!(
            Address::from(keccak(keypair.public())),
            tx.author().unwrap().0
        );
        assert_eq!(*keypair.public(), tx.author().unwrap().1);
        assert_eq!(tx.chain_id(), None);
    }

    #[test]
    fn should_sign_chain_id() {
        let keypair = crypto::publickey::Random.generate();

        let mut tx = Transaction {
            type_payload: TypePayload::Legacy(LegacyPayload {
                gas_price: U256::from(4000),
            }),
            to: CallType::CreateContract(),
            nonce: U64::from(42),
            gas_limit: U256::from(60_000),
            value: U256::from(10),
            data: b"Hello World!".to_vec(),
            signature: Signature::default(),
            chain_id: None,
            hash: H256::zero(),
            author: None,
        };
        tx.sign(keypair.secret());
        assert_eq!(
            Address::from(keccak(keypair.public())),
            tx.author().unwrap().0
        );
        assert_eq!(*keypair.public(), tx.author().unwrap().1);
        assert_eq!(tx.chain_id(), None);
    }

    #[test]
    fn should_sign_access_list() {
        let keypair = crypto::publickey::Random.generate();

        let type_payload = TypePayload::AccessList(AccessListPayload {
            legacy_data: LegacyPayload {
                gas_price: U256::from(10),
            },
            access_list: vec![AccessListItem::new(
                Address::from_low_u64_be(10),
                vec![H256::from_low_u64_be(30), H256::from_low_u64_be(500)],
            )],
        });
        let mut tx = Transaction {
            type_payload,
            to: CallType::CreateContract(),
            nonce: U64::from(42),
            gas_limit: U256::from(60_000),
            value: U256::from(10),
            data: b"Hello World!".to_vec(),
            signature: Signature::default(),
            chain_id: Some(10),
            hash: H256::zero(),
            author: None,
        };
        tx.sign(keypair.secret());

        assert_eq!(
            Address::from(keccak(keypair.public())),
            tx.author().unwrap().0
        );
        assert_eq!(*keypair.public(), tx.author().unwrap().1);
        assert_eq!(tx.chain_id(), Some(10));
    }

    /*#[test]
    fn decode_tx() {
        let raw: Vec<u8> = "f88b8212b085028fa6ae00830f424094aad593da0c8116ef7d2d594dd6a63241bccfc26c80a48318b64b000000000000000000000000641c5d790f862a58ec7abcfd644c0442e9c201b32aa0a6ef9e170bca5ffb7ac05433b13b7043de667fbb0b4a5e45d3b54fb2d6efcc63a0037ec2c05c3d60c5f5f78244ce0a3859e3a18a36c61efb061b383507d3ce19d2".from_hex().unwrap();
        let hash =
            H256::from_str("559fb34c4a7f115db26cbf8505389475caaab3df45f5c7a0faa4abfa3835306c")
                .unwrap();
        let author_add = H160::from_str("641c5d790f862a58ec7abcfd644c0442e9c201b3").unwrap();;
        let mut tx = Transaction::decode(&raw).unwrap();
        tx.recompute_hash();
        tx.recover_author().unwrap();
        assert_eq!(tx.hash(), hash);
        assert_eq!(tx.author().unwrap().0, author_add);
        assert_eq!(tx.chain_id(), Some(3));
        assert_eq!(tx.v(), U64::from_str("2a").unwrap().as_u64());
        assert_eq!(tx.gas_limit, U256::from_str("f4240").unwrap());
        assert_eq!(tx.nonce)

    }*/
}
