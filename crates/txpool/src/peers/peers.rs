use std::{
    collections::{hash_map::Entry, HashMap},
    sync::Arc,
};

use super::peer::Peer;

use bytes::Bytes;
use interfaces::{
    sentry::{Sentry, TxMessage},
    txpool::TransactionPool,
};
use reth_core::{Transaction, H256, H512};
use tokio::{
    sync::{mpsc::UnboundedSender, Mutex, RwLock},
    task::JoinHandle,
};

pub const MAX_KNOWN_TX: usize = 1024;

pub type PeerId = H512;

type PeerHandle = (UnboundedSender<PeerMsg>, JoinHandle<()>);

pub struct Peers {
    peers: RwLock<HashMap<PeerId, PeerHandle>>,
    sentry: Arc<dyn Sentry>,
    pool: Arc<dyn TransactionPool>,
    tx_buffer: Arc<Mutex<Vec<H256>>>,
}

pub enum PeerMsg {
    InboundNewPooledTxHashes(Bytes),
    InboundPooledTx(Bytes),
    InboundGetPooledTxs(Bytes),
    IncludedTxs(Arc<Vec<Arc<Transaction>>>),
}

impl Peers {
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

    pub fn new(sentry: Arc<dyn Sentry>, pool: Arc<dyn TransactionPool>) -> Arc<Self> {
        let tx_buffer = Arc::new(Mutex::new(Vec::new()));

        let peers = Arc::new(Self {
            peers: RwLock::new(HashMap::new()),
            sentry,
            pool,
            tx_buffer,
        });
        peers
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
