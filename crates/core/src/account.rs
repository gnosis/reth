// Copyright 2021 Gnosis Ltd.
// SPDX-License-Identifier: Apache-2.0

use crate::{Keccak, U256};
use serde::{Deserialize, Serialize};

/// https://ethereum.stackexchange.com/questions/268/ethereum-block-architecture
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct Account {
  nonce: U256,
  balance: U256,
  storage_root: Option<Keccak>,
  code_hash: Option<Keccak>,
}
