// Copyright 2021 Gnosis Ltd.
// SPDX-License-Identifier: Apache-2.0
use super::{data::DataTrait, signature::replay_protection, Author, Data, Signature};
use crate::BlockNumber;
use crypto::publickey::{self, Secret};
use ethereum_types::H256;
use keccak_hash::keccak;

pub type ChainId = u64;

pub struct Transaction {
    data: Data,
    signature: Signature,
    chain_id: Option<ChainId>,
    hash: H256,
    author: Option<Author>,
}

impl Transaction {
    pub fn new(
        data: Data,
        signature: Signature,
        chain_id: Option<ChainId>,
        hash: H256,
    ) -> Transaction {
        Transaction {
            data,
            signature,
            chain_id,
            hash,
            author: None,
        }
    }

    pub fn new_sign(data: Data, secret: &Secret, chain_id: Option<ChainId>) -> Transaction {
        let signature_hash = keccak(data.encode(chain_id, None));
        let sig: Signature = publickey::sign(secret, &signature_hash)
            .expect("Expecting valid data and signing of transaction to pass")
            .into();
        let author = sig
            .recover_author(&signature_hash)
            .expect("Expect to recover author successfully");

        let mut tx = Transaction::new(data, sig, chain_id, H256::zero());
        tx.author = Some(author);
        tx.recompute_hash();
        tx
    }

    /// If we want to delay calculating of hash we can send invalid hash
    /// in constructor and use this function to calculate it when we see fit.
    fn recompute_hash(&mut self) {
        self.hash = keccak(&*self.data.encode(self.chain_id, Some(&self.signature)));
    }

    pub fn author(&self) -> Option<Author> {
        self.author
    }

    pub fn data(&self) -> &Data {
        &self.data
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

    /// V from signature that is received from wire in RLP.
    /// For legacy it contains V with replay protected chain_id.
    /// For new transaction types it is ordinary V field from signature.
    pub fn v(&self) -> u64 {
        match self.data {
            Data::Legacy(_) => replay_protection::encode(self.signature.v, self.chain_id),
            _ => self.signature.v as u64,
        }
    }

    pub fn recover_author(&mut self) -> Result<(), publickey::Error> {
        let signature_hash = keccak(self.data.encode(self.chain_id, None));
        self.author = Some(self.signature.recover_author(&signature_hash)?);
        Ok(())
    }

    pub fn has_author(&self) -> bool {
        self.author.is_some()
    }
}

pub struct InsertedTransaction {
    tx: Transaction,
    block_number: BlockNumber,
    block_hash: H256,
    tx_index: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crypto::publickey::Public;
    use ethereum_types::{Address, H160, H512, U256};

    /// Dummy address defined in EIP-86.
    pub const DUMMY_AUTHOR: (Address, Public) = (H160([0xff; 20]), H512([0xff; 64]));

    /// Legacy EIP-86 compatible empty signature.
    /// It is used in json tests
    pub fn null_sign(data: Data, chain_id: u64) -> Transaction {
        let mut tx = Transaction::new(
            data,
            Signature::new(0, U256::zero(), U256::zero()),
            Some(chain_id),
            H256::zero(),
        );
        tx.author = Some(DUMMY_AUTHOR);
        tx.recompute_hash();
        tx
    }

    pub fn fake_sign(data: Data, author: Address) -> Transaction {
        let mut tx = Transaction::new(
            data,
            Signature::new(4, U256::one(), U256::one()),
            None,
            H256::zero(),
        );
        tx.author = Some((author, H512::zero()));
        tx.recompute_hash();
        tx
    }

    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
