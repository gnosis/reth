use async_trait::async_trait;
use grpc_interfaces::{
    txpool::{
        txpool_server::Txpool, AddReply, AddRequest, OnAddReply, OnAddRequest, TransactionsReply,
        TransactionsRequest, TxHashes,
    },
    types::{VersionReply, H256 as gH256},
};
use interfaces::txpool::TransactionPool;
use prost::bytes::Bytes;
use reth_core::*;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Request, Response, Status};
use txpool::Pool;

use reth_core::Transaction;
use tokio::sync::{mpsc::Sender, RwLock};
use txpool::{Announcer, Error};

pub struct GrpcPool {
    pool: Arc<Pool>,
    announcer: Arc<GrpcAnnouncer>,
}

pub struct GrpcAnnouncer {
    subscribers: RwLock<Vec<Sender<Result<OnAddReply, Status>>>>,
}

impl GrpcPool {
    pub fn new(pool: Arc<Pool>) -> Self {
        Self {
            pool,
            announcer: Arc::new(GrpcAnnouncer::new()),
        }
    }

    pub fn announcer(&self) -> Arc<dyn Announcer> {
        self.announcer.clone()
    }
}

#[async_trait]
impl Txpool for GrpcPool {
    async fn version(&self, _: Request<()>) -> Result<Response<VersionReply>, Status> {
        Ok(Response::new(VersionReply {
            major: 0,
            minor: 1,
            patch: 0,
        }))
    }

    async fn find_unknown(&self, request: Request<TxHashes>) -> Result<Response<TxHashes>, Status> {
        let hashes = request
            .get_ref()
            .hashes
            .iter()
            .map(|hash| from_grpc_h256(hash))
            .collect::<Vec<H256>>();
        let hashes: Vec<gH256> = self
            .pool
            .filter_by_negative(&hashes)
            .await
            .into_iter()
            .map(|h| h.into())
            .collect();
        Ok(Response::new(TxHashes { hashes: hashes }))
    }

    async fn add(&self, request: Request<AddRequest>) -> Result<Response<AddReply>, Status> {
        let vec: Vec<_> = request
            .get_ref()
            .rlp_txs
            .iter()
            .map(|t| Transaction::decode(t))
            .filter(|t| t.is_ok())
            .map(|t| t.unwrap())
            .collect();

        let mut txs = Vec::with_capacity(vec.len());
        for mut tx in vec.into_iter() {
            if let Err(_) = tx.recover_author() {
                return Err(Status::new(
                    tonic::Code::Internal,
                    "Recovering author failed",
                ));
            }
            txs.push(Arc::new(tx));
        }
        let _ = self.pool.import(txs).await;
        Ok(Response::new(AddReply {
            imported: vec![0],
            errors: vec!["String".into()],
        })) //TODO fix returns
    }

    async fn transactions(
        &self,
        request: Request<TransactionsRequest>,
    ) -> Result<Response<TransactionsReply>, Status> {
        let hashes = request
            .get_ref()
            .hashes
            .iter()
            .map(|h| from_grpc_h256(h))
            .collect::<Vec<_>>();
        let tx = self.pool.find(&hashes).await;
        Ok(Response::new(TransactionsReply {
            rlp_txs: tx
                .into_iter()
                .filter(|t| t.is_some())
                .map(|t| Bytes::from(t.unwrap().encode()))
                .collect::<Vec<_>>(),
        }))
    }

    type OnAddStream = ReceiverStream<Result<OnAddReply, Status>>;

    async fn on_add(
        &self,
        _: Request<OnAddRequest>,
    ) -> Result<Response<Self::OnAddStream>, Status> {
        // TODO see what to do when/if buffer gets full.
        let (tx, rx) = mpsc::channel(1000);
        self.announcer.subscribe(tx).await;

        Ok(Response::new(ReceiverStream::new(rx)))
    }
}

/// It is currently simplified announcer.

impl GrpcAnnouncer {
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
impl Announcer for GrpcAnnouncer {
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

    async fn reinserted(&self, _tx: Arc<Transaction>) {}

    async fn removed(&self, _tx: Arc<Transaction>, _error: Error) {}
}
