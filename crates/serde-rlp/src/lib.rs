// Copyright 2021 Gnosis Ltd.
// SPDX-License-Identifier: Apache-2.0

pub mod de;
pub mod error;
pub mod ser;

pub use error::Result;

use serde::{de::DeserializeOwned, Deserialize, Serialize};

pub fn serialize<E>(object: &E) -> Result<Vec<u8>>
where
    E: Serialize,
{
    todo!();
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
mod tests {
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Serialize, Deserialize)]
    struct Person {
        first_name: String,
        last_name: String,
        age: u64,
    }

    #[test]
    fn struct_serde_rlp_sanity_test() {
        let original = Person {
            first_name: "first".to_owned(),
            last_name: "last".to_owned(),
            age: 99,
        };

        let bytes = super::serialize(&original).unwrap();

        let reconstructed: Person = super::deserialize(&bytes).unwrap();

        assert_eq!(reconstructed.first_name, "first");
        assert_eq!(reconstructed.last_name, "last");
        assert_eq!(reconstructed.age, 99);
    }
}
