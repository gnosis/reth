// Copyright 2021 Gnosis Ltd.
// SPDX-License-Identifier: Apache-2.0

use clap::Clap;
use log::*;
use std::sync::Arc;
use toml;
use txpool::Pool;

use crate::{
    announcer::AnnouncerImpl,
    config::{Opts, *},
    grpc_sentry::GrpcSentry,
    grpc_txpool::GrpcPool,
    grpc_world_state::GrpcWorldState,
};

mod announcer;
mod config;
mod grpc_sentry;
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
    let sentry_uri = config.devp2p.as_ref().unwrap().clone();

    let world_state = Arc::new(GrpcWorldState::new(world_state_uri).await);
    let sentry = Arc::new(GrpcSentry::new(sentry_uri).await);

    let config = Arc::new(config.into());

    // announcer for inclusion and removed of tx. Used in GrpcTxPool
    let annon = Arc::new(AnnouncerImpl::new());

    //Create objects
    let pool = Arc::new(Pool::new(config, world_state, annon.clone()));

    let sentry_pool = pool.clone();

    // rust sentry
    tokio::spawn(async move {
        let _ = sentry.run(sentry_pool).await;
    });

    // start grpc
    let pool = GrpcPool::new(pool, annon);
    pool.start().await

    // end it
}

#[cfg(test)]
mod test {

    #[test]
    fn pool_insert_delete() {}
}
