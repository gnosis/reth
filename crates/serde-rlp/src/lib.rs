// Copyright 2021 Gnosis Ltd.
// SPDX-License-Identifier: Apache-2.0

pub mod de;
pub mod error;
pub mod ser;

pub use error::Result;

use ser::EthereumRlpSerializer;
use serde::{de::DeserializeOwned, Deserialize, Serialize};

pub fn serialize<E>(object: &E) -> Result<Vec<u8>>
where
    E: Serialize + ?Sized,
{
    let mut serializer = EthereumRlpSerializer::new();
    object.serialize(&mut serializer)?;
    Ok(serializer.finalize())
}

pub fn deserialize<'a, T>(bytes: &'a [u8]) -> Result<T>
where
    T: Deserialize<'a>,
{
    todo!();
}

pub fn deserialize_from<R, T>(reader: R) -> Result<T>
where
    R: std::io::Read,
    T: DeserializeOwned,
{
    todo!();
}

#[cfg(test)]
mod tests;