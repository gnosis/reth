// Copyright 2021 Gnosis Ltd.
// SPDX-License-Identifier: Apache-2.0

/// When doing shift by 3 you get value incress by 12.5%
pub const BUMP_SCORE_BY_12_5_PERC: usize = 3;

/// max amount of transactions that we will keep before we recreate binary heap.
/// Recreating of binary heap is expensive and this is optimization.
pub const MAX_PENDING_TX_REMOVALS: usize = 100;
#[derive(Clone, Copy)]
pub struct Config {
    pub to_use: bool,
    pub max_tx_per_account: usize,
    pub max_tx_count_global: usize,
}

impl Default for Config {
    fn default() -> Config {
        Config {
            to_use: true,
            max_tx_per_account: 8,
            max_tx_count_global: 1024,
        }
    }
}
