use anyhow::Result;
use async_trait::async_trait;
use bytes::Bytes;
use grpc_interfaces::sentry::{
    sentry_client::SentryClient, MessageId, OutboundMessageData, SendMessageByIdRequest,
};
use interfaces::sentry::{PeerId, Sentry, TxMessage};
use std::sync::Arc;
use tokio::sync::Mutex;
use tonic::{transport::Channel, Request};
use txpool::{Peers, Pool};

pub struct GrpcSentry {
    client: Mutex<SentryClient<Channel>>,
}

pub fn into_grpc_message_id(id: TxMessage) -> MessageId {
    match id {
        TxMessage::GetPooledTransactions => MessageId::GetPooledTransactions,
        TxMessage::NewPooledTransactionHashes => MessageId::NewPooledTransactionHashes,
        TxMessage::PooledTransactions => MessageId::PooledTransactions,
        TxMessage::Transactions => MessageId::Transactions,
    }
}

pub fn from_grpc_message_id(id: Option<MessageId>) -> Option<TxMessage> {
    match id {
        Some(MessageId::GetPooledTransactions) => Some(TxMessage::GetPooledTransactions),
        Some(MessageId::NewPooledTransactionHashes) => Some(TxMessage::NewPooledTransactionHashes),
        Some(MessageId::PooledTransactions) => Some(TxMessage::PooledTransactions),
        Some(MessageId::Transactions) => Some(TxMessage::Transactions),
        _ => None,
    }
}

#[async_trait]
impl Sentry for GrpcSentry {
    async fn send_message_by_id(&self, peer_id: PeerId, message_id: TxMessage, data: Bytes) {
        let request = SendMessageByIdRequest {
            data: Some(OutboundMessageData {
                id: into_grpc_message_id(message_id) as i32,
                data: data,
            }),
            peer_id: Some(peer_id.into()),
        };
        let _ = self
            .client
            .lock()
            .await
            .send_message_by_id(Request::new(request))
            .await;
        // TODO see if we need to handle response.
        // For example if msg is not send does that mean we need to disconnect that peer.
    }
}

impl GrpcSentry {
    pub async fn new(address: String) -> Self {
        let client = Mutex::new(SentryClient::connect(address).await.unwrap());
        Self { client }
    }

    pub async fn run(&self, peers: Arc<Peers>) -> Result<()> {
        let mut stream = self
            .client
            .lock()
            .await
            .receive_tx_messages(Request::new(()))
            .await?
            .into_inner();

        while let Some(msg) = stream.message().await? {
            if msg.peer_id.is_none() {
                continue;
            }
            match from_grpc_message_id(MessageId::from_i32(msg.id)) {
                Some(id) => {
                    peers
                        .inbound(&msg.peer_id.unwrap().into(), id, msg.data)
                        .await;
                }
                Some(_) => {}
                None => {}
            }
        }
        Ok(())
    }
}
