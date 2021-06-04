// Copyright 2021 Gnosis Ltd.
// SPDX-License-Identifier: Apache-2.0

use std::time::Duration;

use clap::*;
use serde::Deserialize;
use txpool::Config as TxPoolConfig;

/// When doing shift by 3 you get value incress by 12.5%
pub const BUMP_SCORE_BY_12_5_PERC: usize = 3;

/// max amount of transactions that we will keep before we recreate binary heap.
/// Recreating of binary heap is expensive and this is optimization.
pub const MAX_PENDING_TX_REMOVALS: usize = 100;
// #[derive(Clone, Copy)]
// pub struct Config {
//     pub max_tx_per_account: usize,
//     pub max_tx_count_global: usize,
// }

#[derive(Clap, Clone)]
#[clap(version = "1.0", author = "Gnosis Devs")]
#[clap(setting = AppSettings::ColoredHelp)]
pub struct Opts {
    #[clap(
        short,
        long,
        default_value = "default.config.tml",
        about = "Config file"
    )]
    pub config: String,
}

#[derive(Deserialize, Clone)]
pub struct Config {
    pub serve_address: Option<String>,
    pub sentry: Option<String>,
    pub world_state: Option<String>,
    pub per_account: Option<usize>,
    pub max: Option<usize>,
    pub tx_timeout: Option<Duration>,
    pub serve_ip: Option<String>,
}

impl Default for Config {
    fn default() -> Config {
        Config {
            serve_address: None,
            sentry: None,
            world_state: None,
            per_account: Some(20),
            max: Some(10000),
            tx_timeout: Some(Duration::from_secs(300)),
            serve_ip: Some("[::1]:50001".into()),
        }
    }
}

impl Into<TxPoolConfig> for Config {
    fn into(self) -> TxPoolConfig {
        TxPoolConfig {
            per_account: self.per_account.unwrap_or(16),
            max: self.per_account.unwrap_or(100),
            timeout: self.tx_timeout.unwrap_or(Duration::from_secs(300)),
        }
    }
}
