use anyhow::Result;
use grpc_interfaces::sentry::{sentry_client::SentryClient, MessageId};
use interfaces::txpool::TransactionPool;
use rlp::Rlp;
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

        // TODO check how are are handling eth/65 and who has list of known txs (sentry or txpool)?
        while let Some(msg) = stream.message().await? {
            match MessageId::from_i32(msg.id) {
                Some(MessageId::Transactions) => {
                    let data = msg.data.to_vec();
                    let rlp = Rlp::new(data.as_slice());
                    let list = rlp.as_list();
                    match list {
                        Ok(txs) => {
                            pool.import(txs.as_slice()).await;
                        }
                        _ => (),
                    }
                    //TODO how to handle decode error, should we send penal to peer_id.
                }
                Some(_) => {}
                None => {}
            }
        }
        Ok(())
    }
}
