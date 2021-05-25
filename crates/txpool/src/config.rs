// Copyright 2021 Gnosis Ltd.
// SPDX-License-Identifier: Apache-2.0

use std::time::Duration;

use serde::Deserialize;

/// Replace tx only if it has 12.5% better score.
pub const BUMP_SCORE_BY_12_5_PERC: usize = 3; // When doing shift by 3 you get value incress by 12.5%

/// max amount of transactions that we will keep before we recreate binary heap.
/// Recreating of binary heap is expensive and this is optimization.
pub const MAX_PENDING_TX_REMOVALS: usize = 100;

#[derive(Deserialize)]
pub struct Config {
    pub per_account: usize,
    pub max: usize,
    pub timeout: Duration,
}

impl Default for Config {
    fn default() -> Config {
        Config {
            per_account: 20,
            max: 10000,
            timeout: Duration::from_secs(300),
        }
    }
}
