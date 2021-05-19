
use interfaces::devp2p::Adapter;
use async_trait::async_trait;
use tokio::time;
use std::time::Duration;

pub struct GrpcDevp2p {
    
}

impl GrpcDevp2p {
    pub async fn get_transaction(&self) -> Vec<Vec<u8>> {
        time::sleep(Duration::from_secs(100)).await;
        Vec::new()
    }
}