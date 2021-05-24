use std::sync::Arc;

use async_trait::async_trait;
use grpc_interfaces::txpool::OnAddReply;
use reth_core::Transaction;
use tokio::sync::{mpsc::Sender, RwLock};
use tonic::Status;
use txpool::{Announcer, Error};

/// It is currently simplified announcer.
pub struct AnnouncerImpl {
    subscribers: RwLock<Vec<Sender<Result<OnAddReply, Status>>>>,
}

impl AnnouncerImpl {
    pub fn new() -> Self {
        Self {
            subscribers: RwLock::new(Vec::new()),
        }
    }

    pub async fn subscribe(&self, sub: Sender<Result<OnAddReply, Status>>) {
        self.subscribers.write().await.push(sub);
    }
}

#[async_trait]
impl Announcer for AnnouncerImpl {
    async fn inserted(&self, tx: Arc<Transaction>) {
        let mut subs = self.subscribers.write().await;
        let mut rem = Vec::new();
        for (i, sub) in subs.iter().enumerate() {
            if sub
                .send(Ok(OnAddReply {
                    rpl_txs: vec![tx.encode().into()],
                }))
                .await
                .is_err()
            {
                rem.push(i);
            }
        }
        //to remove
        if !rem.is_empty() {
            for (i, rem) in rem.into_iter().enumerate() {
                subs.remove(rem - i);
            }
        }
    }

    async fn removed(&self, tx: Arc<Transaction>, error: Error) {}
}
