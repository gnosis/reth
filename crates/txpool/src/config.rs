// Copyright 2021 Gnosis Ltd.
// SPDX-License-Identifier: Apache-2.0

use serde::Deserialize;

/// When doing shift by 3 you get value incress by 12.5%
pub const BUMP_SCORE_BY_12_5_PERC: usize = 3;

/// max amount of transactions that we will keep before we recreate binary heap.
/// Recreating of binary heap is expensive and this is optimization.
pub const MAX_PENDING_TX_REMOVALS: usize = 100;

#[derive(Deserialize)]
pub struct Config {
    pub per_account: usize,
    pub max: usize,
}


impl Default for Config {
    fn default() -> Config {
        Config {
            per_account: 20,
            max: 10000,
        }
    }
}

