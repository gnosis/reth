use super::*;
use ethereum_types::{U256};
use rlp::{RlpStream};

/// Components that constitute transaction signature
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Signature {
  /// The V field of the signature; which half of the curve our point falls in.
  pub v: SigV,
  /// The R field of the signature; helps describe the point on the curve.
  pub r: U256,
  /// The S field of the signature; helps describe the point on the curve.
  pub s: U256,
}

impl Signature {
    pub fn rlp_append(&self, s: &mut RlpStream) {
        s.append(&self.v);
        s.append(&self.r);
        s.append(&self.s);
    }
}