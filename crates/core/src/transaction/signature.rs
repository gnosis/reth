use std::error::Error;

/// Copyright 2021 Gnosis Ltd.
/// SPDX-License-Identifier: Apache-2.0
use super::*;
use crypto::publickey::{self, public_to_address, recover, Secret, Signature as CryptoSig};
use ethereum_types::{Address, BigEndianHash, Public, H256, U256};
use rlp::RlpStream;
use serde::{Deserialize, Serialize};

pub type SigV = u8;
pub type Author = (Address, Public);

/// Components that constitute transaction signature
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct Signature {
    /// The V field of the signature; which half of the curve our point falls in.
    pub v: SigV,
    /// The R field of the signature; helps describe the point on the curve.
    pub r: U256,
    /// The S field of the signature; helps describe the point on the curve.
    pub s: U256,
}

impl Signature {
    pub fn new(v: SigV, r: U256, s: U256) -> Self {
        Signature { v, r, s }
    }

    pub fn for_transaction(
        transaction: &Transaction,
        secret: &Secret,
    ) -> Result<Self, Box<dyn Error>> {
        Ok(publickey::sign(secret, &transaction.hash()?)
            .expect("Expecting valid data and signing of transaction to pass")
            .into())
    }

    pub fn rlp_append(&self, s: &mut RlpStream) {
        s.append(&self.v);
        s.append(&self.r);
        s.append(&self.s);
    }

    pub fn is_zero(&self) -> bool {
        self.r.is_zero() && self.s.is_zero()
    }

    pub fn check_low_s(&self) -> Result<(), publickey::Error> {
        let crypto_sig: CryptoSig = self.into();
        if !crypto_sig.is_low_s() {
            Err(publickey::Error::InvalidSignature)
        } else {
            Ok(())
        }
    }

    pub fn recover_author(&self, hash: &H256) -> Result<Author, publickey::Error> {
        if self.is_zero() {
            return Err(publickey::Error::InvalidSignature);
        }
        let public = recover(&self.into(), &hash)?;
        let address = public_to_address(&public);
        Ok((address, public))
    }
}

impl Default for Signature {
    fn default() -> Signature {
        Signature {
            v: 4,
            r: 0.into(),
            s: 0.into(),
        }
    }
}

impl From<CryptoSig> for Signature {
    fn from(sig: CryptoSig) -> Self {
        Signature {
            v: sig.v(),
            r: sig.r().into(),
            s: sig.s().into(),
        }
    }
}

impl From<&Signature> for CryptoSig {
    fn from(sig: &Signature) -> Self {
        let r: H256 = BigEndianHash::from_uint(&sig.r);
        let s: H256 = BigEndianHash::from_uint(&sig.s);
        CryptoSig::from_rsv(&r, &s, sig.v)
    }
}

// pub mod replay_protection {
//     /// Transaction replay protection
//     use super::SigV;

//     pub type SigVLegacy = u64;
//     pub type ChainId = u64;

//     /// Merge chain_id and signature V
//     pub fn encode(v: SigV, chain_id: Option<ChainId>) -> SigVLegacy {
//         let replay: u64 = if let Some(n) = chain_id {
//             35 + n * 2
//         } else {
//             27
//         };
//         v as u64 + replay
//     }

//     /// Returns standard v from replay protected legacy V
//     pub fn decode_v(v: SigVLegacy) -> SigV {
//         if v == 27 {
//             0
//         } else if v == 28 {
//             1
//         } else if v > 35 {
//             ((v - 1) % 2) as u8
//         } else {
//             4 //invalid value
//         }
//     }

//     // return chain id from replay protected legacy V
//     pub fn decode_chain_id(v: SigVLegacy) -> Option<ChainId> {
//         if v >= 35 {
//             Some((v - 35) / 2 as u64)
//         } else {
//             None
//         }
//     }
// }

#[cfg(test)]
mod tests {}
