// Copyright 2021 Gnosis Ltd.
// SPDX-License-Identifier: Apache-2.0

use std::{error::Error, str::FromStr};

// Copyright 2021 Gnosis Ltd.
// SPDX-License-Identifier: Apache-2.0
use clap::Clap;
use interfaces::txpool::TransactionPool;
use log::{info, trace};
use std::{sync::Arc, time::Duration};
use tokio::time::sleep;
use toml;
use txpool::{config::*, Pool};

#[tokio::main]
async fn main() {
    info!("Running TXPOOL");

    let config: Arc<Config> = Arc::new(
        toml::from_str(&std::fs::read_to_string(Opts::parse().config).unwrap_or_default())
            .unwrap_or_default(),
    );

    if config.world_state.is_none() || config.devp2p.is_none() {
        //panic!("World state and devp2p needs to be set in config");
    }

    //Create objects
    // let pool = Arc::new(Pool::new(config));
    // {
    //     let mut tasks = Vec::new();
    //     tasks.push(pool.import(vec![]));


    //     for task in tasks {
    //         task.await;
    //     }
    // }


    // end it
}

#[cfg(test)]
mod test {

    #[test]
    fn pool_insert_delete() {}
}
