// Copyright 2021 Gnosis Ltd.
// SPDX-License-Identifier: Apache-2.0
use ethereum_types::U64;

#[derive(Debug, Copy, Clone)]
#[repr(u8)]
pub enum TxType {
    AccessList = 0x01,
    Legacy = 0x00,
}

impl TxType {
    // used in json tets
    #![allow(dead_code)]
    pub fn from_u8_id(n: u8) -> Option<Self> {
        match n {
            0 => Some(Self::Legacy),
            1 => Some(Self::AccessList),
            _ => None,
        }
    }

    pub fn try_from_wire_byte(n: u8) -> Result<Self, ()> {
        match n {
            x if x == Self::AccessList as u8 => Ok(Self::AccessList),
            x if (x & 0x80) != 0x00 => Ok(Self::Legacy),
            _ => Err(()),
        }
    }

    #[allow(non_snake_case)]
    pub fn from_U64_option_id(n: Option<U64>) -> Option<Self> {
        match n.map(|t| t.as_u64()) {
            None => Some(Self::Legacy),
            Some(0x01) => Some(Self::AccessList),
            _ => None,
        }
    }

    #[allow(non_snake_case)]
    pub fn to_U64_option_id(self) -> Option<U64> {
        match self {
            Self::Legacy => None,
            _ => Some(U64::from(self as u8)),
        }
    }
}

impl Default for TxType {
    fn default() -> Self {
        Self::Legacy
    }
}
