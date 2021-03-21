// Copyright 2021 Gnosis Ltd.
// SPDX-License-Identifier: Apache-2.0

use crate as serde_rlp;

use hex_literal::hex;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct Person {
    first_name: String,
    last_name: String,
    age: u64,
}

#[test]
fn struct_sanity_test() {
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

#[test]
fn vec_u64_test() -> serde_rlp::Result<()> {
    let empty_v: Vec<u64> = vec![];
    let single_v = vec![15_u64];
    let many_v = vec![1, 2, 3, 7, 0xff];

    let encoded_empty_v = serde_rlp::serialize(&empty_v)?;
    let encoded_single_v = serde_rlp::serialize(&single_v)?;
    let encoded_many_v = serde_rlp::serialize(&many_v)?;

    assert_eq!(encoded_empty_v, hex!("c0"));
    assert_eq!(encoded_single_v, hex!("c10f"));
    assert_eq!(encoded_many_v, hex!("c60102030781ff"));

    let decoded_many_v: Vec<u64> = serde_rlp::deserialize(&encoded_many_v)?;
    let decoded_empty_v: Vec<u64> = serde_rlp::deserialize(&encoded_empty_v)?;
    let decoded_single_v: Vec<u64> = serde_rlp::deserialize(&encoded_single_v)?;

    assert_eq!(empty_v, decoded_empty_v);
    assert_eq!(single_v, decoded_single_v);
    assert_eq!(many_v, decoded_many_v);

    Ok(())
}

#[test]
fn str_test() -> serde_rlp::Result<()> {
    let s = "Lorem ipsum dolor sit amet, consectetur adipisicing elit";
    let encoded_s = serde_rlp::serialize(s)?;

    assert_eq!(
        encoded_s,
        vec![
            0xb8, 0x38, b'L', b'o', b'r', b'e', b'm', b' ', b'i', b'p', b's', b'u', b'm', b' ',
            b'd', b'o', b'l', b'o', b'r', b' ', b's', b'i', b't', b' ', b'a', b'm', b'e', b't',
            b',', b' ', b'c', b'o', b'n', b's', b'e', b'c', b't', b'e', b't', b'u', b'r', b' ',
            b'a', b'd', b'i', b'p', b'i', b's', b'i', b'c', b'i', b'n', b'g', b' ', b'e', b'l',
            b'i', b't',
        ]
    );

    let decoded_s: &str = serde_rlp::deserialize(&encoded_s)?;
    assert_eq!(s, decoded_s);

    Ok(())
}
