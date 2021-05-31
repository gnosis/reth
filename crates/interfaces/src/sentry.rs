use async_trait::async_trait;
use reth_core::H512;
use bytes::Bytes;

pub type PeerId = H512;

#[async_trait]
pub trait Sentry: Send + Sync {
    async fn send_message_by_id(&self, peer_id: PeerId, message_id: TxMessage, data: Bytes);
    //TODO add other functions that mimic grpc interface
}



pub enum TxMessage {
    NewPooledTransactionHashes,
    PooledTransactions,
    GetPooledTransactions,
    Transactions, // should we remove it?
}