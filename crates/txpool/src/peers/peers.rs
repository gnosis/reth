use std::{
    collections::{hash_map::Entry, HashMap},
    sync::Arc,
    time::Duration,
};

use crate::Announcer;

use super::peer::Peer;

use async_trait::async_trait;
use bytes::Bytes;
use interfaces::{
    sentry::{Sentry, TxMessage},
    txpool::TransactionPool,
};
use reth_core::{Transaction, H256, H512};
use tokio::{
    sync::{mpsc::UnboundedSender, Mutex, Notify, RwLock},
    task::JoinHandle,
};

pub const MAX_KNOWN_TX: usize = 1024;

pub type PeerId = H512;

type PeerHandle = (UnboundedSender<PeerMsg>, JoinHandle<()>);

pub struct Peers {
    peers: RwLock<HashMap<PeerId, PeerHandle>>,
    sentry: Arc<dyn Sentry>,
    pool: Arc<dyn TransactionPool>,
    tx_buffer: Arc<Mutex<Vec<Arc<Transaction>>>>,
    notify_tx_buffer: Arc<Notify>,
}

#[derive(Clone)]
pub enum PeerMsg {
    InboundNewPooledTxHashes(Bytes),
    InboundPooledTx(Bytes),
    InboundGetPooledTxs(Bytes),
    IncludedTxs(Arc<Vec<Arc<Transaction>>>),
}

impl Peers {
    pub fn new(sentry: Arc<dyn Sentry>, pool: Arc<dyn TransactionPool>) -> Arc<Self> {
        let peers = Arc::new(Self {
            peers: RwLock::new(HashMap::new()),
            sentry,
            pool,
            tx_buffer: Arc::new(Mutex::new(Vec::new())),
            notify_tx_buffer: Arc::new(Notify::new()),
        });

        let peers2 = peers.clone();

        tokio::task::spawn(async move {
            // empty buffer
            let txs: Vec<_> = {
                peers2.notify_tx_buffer.notified().await;
                // sleep 50ms after waking up so that we can wait for new incoming tx.
                tokio::time::sleep(Duration::from_millis(50)).await;
                std::mem::take(peers2.tx_buffer.lock().await.as_mut())
            };
            // create peer msg with buffered new tx
            let peer_msg = PeerMsg::IncludedTxs(Arc::new(txs));

            // if there are errors on send, remove peer after sending is finished.
            let mut disconnected_peers = Vec::new();

            // iterate over all peers and send them Arc pointer to transaction list.
            for (peer_id, (ch, _)) in peers2.peers.read().await.iter() {
                if ch.send(peer_msg.clone()).is_err() {
                    disconnected_peers.push(*peer_id);
                }
            }
            {
                // remove disconnected peers from HashMap
                if !disconnected_peers.is_empty() {
                    let mut peers = peers2.peers.write().await;
                    for dis in disconnected_peers.iter() {
                        peers.remove(dis);
                    }
                }
            }

            // sleep when sending of new transaction is over so that we are not sending only one tx.
            tokio::time::sleep(Duration::from_secs(1))
        });

        peers
    }

    pub async fn inbound(&self, peer_id: &PeerId, message_id: TxMessage, data: Bytes) {
        let peer_msg = match message_id {
            TxMessage::NewPooledTransactionHashes => PeerMsg::InboundNewPooledTxHashes(data),
            TxMessage::PooledTransactions => PeerMsg::InboundPooledTx(data),
            TxMessage::GetPooledTransactions => PeerMsg::InboundGetPooledTxs(data),
            _ => {
                return;
            } //TODO error
        };

        let res = if let Some(handle) = self.peers.read().await.get(peer_id) {
            handle.0.send(peer_msg)
        } else {
            let mut peer = self.peers.write().await;
            match peer.entry(*peer_id) {
                Entry::Occupied(occ) => occ.get().0.send(peer_msg),
                Entry::Vacant(vac) => vac.insert(self.new_peer(peer_id)).0.send(peer_msg),
            }
        };
        // if there is a error in sending the msg, this means that receiver is closed and we can remove peer from peers.
        if let Err(_) = res {
            self.peers.write().await.remove(peer_id);
            return;
        }
    }

    pub async fn disconnect_peer(&self, peer_id: &PeerId) {
        self.peers.write().await.remove(peer_id);
    }

    pub fn new_peer(&self, peer_id: &PeerId) -> PeerHandle {
        let sentry = self.sentry.clone();
        let pool = self.pool.clone();
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<PeerMsg>();
        let peer_id = *peer_id;
        let join = tokio::spawn(async move {
            let mut peer = Peer::new(peer_id, pool, sentry);
            peer.run_loop(&mut rx).await
        });
        (tx, join)
    }
}

#[async_trait]
impl Announcer for Peers {
    async fn inserted(&self, tx: Arc<Transaction>) {
        self.tx_buffer.lock().await.push(tx);
        self.notify_tx_buffer.notify_one();
    }

    async fn reinserted(&self, tx: Arc<Transaction>) {}

    async fn removed(&self, tx: Arc<Transaction>, error: crate::Error) {}
}
