// Copyright 2021 Gnosis Ltd.
// SPDX-License-Identifier: Apache-2.0

use crate::{Address, U256};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
  /// Create creates new contract.
  Create,
  /// Calls contract at given address.
  /// In the case of a transfer, this is the receiver's address.'
  Call(Address),
}

impl<'de> serde::Deserialize<'de> for Action {
  fn deserialize<D>(_deserializer: D) -> Result<Self, D::Error>
  where
    D: serde::Deserializer<'de>,
  {
    todo!()
  }
}

impl serde::Serialize for Action {
  fn serialize<S>(&self, _serializer: S) -> Result<S::Ok, S::Error>
  where
    S: serde::Serializer,
  {
    todo!()
  }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum V {
  V27,
  V28,
}

impl<'de> serde::Deserialize<'de> for V {
  fn deserialize<D>(_deserializer: D) -> Result<Self, D::Error>
  where
    D: serde::Deserializer<'de>,
  {
    todo!()
  }
}

impl serde::Serialize for V {
  fn serialize<S>(&self, _serializer: S) -> Result<S::Ok, S::Error>
  where
    S: serde::Serializer,
  {
    todo!()
  }
}

/// Components that constitute transaction signature
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct SignatureComponents {
  pub v: V,    // The V field of the signature; which half of the curve our point falls in.
  pub r: U256, // The R field of the signature; helps describe the point on the curve.
  pub s: U256, // The S field of the signature; helps describe the point on the curve.
}

/// https://ethereum.stackexchange.com/questions/268/ethereum-block-architecture
#[derive(Debug, Eq, Clone, PartialEq, Serialize, Deserialize)]
pub struct Transaction {
  pub nonce: U256,
  pub gas: U256,
  pub gas_price: U256,
  pub action: Action,
  pub value: U256,
  pub signature: SignatureComponents,
  pub data: Vec<u8>,
}
