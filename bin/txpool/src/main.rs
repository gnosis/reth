// Copyright 2021 Gnosis Ltd.
// SPDX-License-Identifier: Apache-2.0

use clap::Clap;
use grpc_interfaces::txpool::txpool_server::{Txpool, TxpoolServer};
use interfaces::world_state::WorldState;
use log::*;
use std::sync::Arc;
use toml;
use tonic::transport::Server;
use txpool::{Peers, Pool};

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

pub async fn configure() -> Config {
    let config: Config =
        toml::from_str(&std::fs::read_to_string(Opts::parse().config).unwrap_or_default())
            .unwrap_or_default();

    config
        .serve_address
        .as_ref()
        .unwrap_or_else(|| panic!("Config should contain serve address"));
    config
        .world_state
        .as_ref()
        .unwrap_or_else(|| panic!("Config should contain world state uri"));
    config
        .sentry
        .as_ref()
        .unwrap_or_else(|| panic!("Config should contain sentry uri"));

    config
}

pub async fn init(
    config: Arc<Config>,
) -> (Arc<GrpcSentry>, Arc<Peers>, Arc<Pool>, GrpcPool, Arc<GrpcWorldState>) {
    let world_state =
        Arc::new(GrpcWorldState::new(config.world_state.as_ref().unwrap().clone()).await);
    let sentry = Arc::new(GrpcSentry::new(config.sentry.as_ref().unwrap().clone()).await);

    // to pool config
    let config: Arc<txpool::Config> = Arc::new(config.as_ref().clone().into());

    // announcer for inclusion and removed of tx. Used in GrpcTxPool
    let annon = Arc::new(AnnouncerImpl::new());

    //Create objects
    let pool = Arc::new(Pool::new(config, world_state.clone(), annon.clone()));

    let peers = Peers::new(sentry.clone(), pool.clone());

    let pool_server = GrpcPool::new(pool.clone(), annon.clone());
    (sentry, peers, pool, pool_server, world_state)
}

pub async fn run(
    config: Arc<Config>,
    sentry: Arc<GrpcSentry>,
    world_state: Arc<GrpcWorldState>,
    peers: Arc<Peers>,
    pool: Arc<Pool>,
    pool_server: GrpcPool,
) {
    // rust sentry
    let _ = tokio::spawn(async move {
        let _ = sentry.run(peers).await;
    });

    // run world state for block update
    let _ = tokio::spawn(async move {
        let _ = world_state.run(pool).await;
    });

    let addr = config.serve_ip.as_ref().unwrap().parse().unwrap();

    // start grpc server
    let _res = Server::builder()
        .add_service(TxpoolServer::new(pool_server))
        .serve(addr)
        .await;

    //TODO res; should we wait or not
}

#[tokio::main]
async fn main() {
    info!("Starting TXPOOL");

    let config = Arc::new(configure().await);

    let (sentry, peers, pool, pool_service, world_state) = init(config.clone()).await;

    run(config, sentry, world_state, peers, pool, pool_service).await;

    //cleanup
}
