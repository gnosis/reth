// Copyright 2021 Gnosis Ltd.
// SPDX-License-Identifier: Apache-2.0

use crate as serde_rlp;

use ethereum_types::U256;
use hex_literal::hex;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct Person {
    first_name: String,
    last_name: String,
    age: u64,
}

#[test]
fn struct_simple_test() -> serde_rlp::Result<()> {
    
    #[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
    struct Item {
        a: String
    }

    let item = Item { a: "cat".into() };
    let expected = vec![0xc4, 0x83, b'c', b'a', b't'];
    let out = serde_rlp::serialize(&item)?;
    
    assert_eq!(out, expected);

    let decoded = serde_rlp::deserialize(&expected)?;
    assert_eq!(item, decoded);

    Ok(())
}

#[test]
fn struct_complex_test() -> serde_rlp::Result<()> {

    #[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
    struct Item {
        a: String,
        b: u64,
        c: ethereum_types::U256
    }

	let item = Item { 
        a: "cat".into(),
        b: 1599u64,
        c: U256::from(208090)
    };

	let out = serde_rlp::serialize(&item)?;
    let deserialized: Item = serde_rlp::deserialize(&out)?;
    assert_eq!(item, deserialized);

    Ok(())
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

    let decoded_s: String = serde_rlp::deserialize(&encoded_s)?;
    assert_eq!(s, decoded_s);

    Ok(())
}
