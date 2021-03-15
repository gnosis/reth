// Copyright 2021 Gnosis Ltd.
// SPDX-License-Identifier: Apache-2.0

use std::error::Error;
use std::str::FromStr;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
  let num = "0xf0000000000f000000000000000000000000000000000000000000000000000f";
  let n = core::U256::from_str(&num)?;
  println!("n = {}", n);
  println!("Hello, world!");

  Ok(())
}
