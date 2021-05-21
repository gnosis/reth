use anyhow::Result;
use grpc_interfaces::sentry::sentry_client::SentryClient;
use log::trace;
use std::sync::Arc;
use tokio::sync::Mutex;
use tonic::{transport::Channel, Request};
use txpool::Pool;

pub struct GrpcSentry {
    client: Mutex<SentryClient<Channel>>,
}

impl GrpcSentry {
    pub async fn new(address: String) -> Self {
        let client = Mutex::new(SentryClient::connect(address).await.unwrap());
        Self { client }
    }

    pub async fn run(&self, pool: Arc<Pool>) -> Result<()> {
        //self.client.lock().await.receive_tx_messages(request)
        let mut stream = self
            .client
            .lock()
            .await
            .receive_tx_messages(Request::new(()))
            .await?
            .into_inner();

        while let Some(tx) = stream.message().await? {
            // TODO
            trace!("Received tx msg");
        }
        Ok(())
    }
}
