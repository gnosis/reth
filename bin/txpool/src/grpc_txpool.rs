
use interfaces::devp2p::Adapter;
use async_trait::async_trait;
use tokio::time;
use std::{sync::Arc, time::Duration};
use txpool::Pool;

pub struct GrpcPool {
    pool: Arc<Pool>,
}

impl GrpcPool {
    pub fn new(pool: Arc<Pool>) -> Self {
        Self {
            pool,
        }
    }

    pub async fn start(&self) {
        loop {
            time::sleep(Duration::from_secs(100)).await;
        }
    }
}