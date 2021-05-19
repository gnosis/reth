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
use txpool::{Config as TxPoolConfig, Pool};

use crate::config::*;
use crate::grpc_txpool::GrpcPool;
use crate::grpc_world_state::GrpcWorldState;
use crate::{config::Opts, grpc_devp2p::GrpcDevp2p};

mod config;
mod grpc_devp2p;
mod grpc_txpool;
mod grpc_world_state;

#[tokio::main]
async fn main() {
    info!("Starting TXPOOL");

    let config: Config =
        toml::from_str(&std::fs::read_to_string(Opts::parse().config).unwrap_or_default())
            .unwrap_or_default();

    if config.world_state.is_none() || config.devp2p.is_none() {
        panic!("World state and devp2p needs to be set in config");
    }
    let world_state_uri = config.world_state.as_ref().unwrap().clone();
    let _devp2p_uri = config.devp2p.as_ref().unwrap().clone();

    let world_state = Arc::new(GrpcWorldState::new(world_state_uri).await);
    let devp2p = Arc::new(GrpcDevp2p {});

    let config = Arc::new(config.into());

    //Create objects
    let pool = Arc::new(Pool::new(config, world_state));

    let pool2 = pool.clone();
    // connect devp2p to pool
    tokio::spawn(async move {
        let _ = pool2.import(devp2p.get_transaction().await);
    });

    // start grpc
    let pool = GrpcPool::new(pool);
    pool.start().await

    // end it
}

#[cfg(test)]
mod test {

    #[test]
    fn pool_insert_delete() {}
}
