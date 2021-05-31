use std::{
    collections::{HashMap, HashSet, VecDeque},
    sync::Arc,
};

use bytes::Bytes;
use interfaces::{
    sentry::{PeerId, Sentry, TxMessage},
    txpool::TransactionPool,
};
use reth_core::{transaction::TxType, Transaction, H256};
use rlp::{DecoderError, Rlp, RlpStream};
use tokio::sync::mpsc::UnboundedReceiver;

use super::PeerMsg;

use anyhow::Result;

pub const MAX_KNOWN_TX: usize = 1024;

pub struct Peer {
    peer_id: PeerId,
    requested: HashMap<u64, Vec<H256>>, // Maybe add as Map <RequestId, Vec<H256>>
    known: HashSet<H256>,
    known_sorted: VecDeque<H256>,
    pool: Arc<dyn TransactionPool>,
    sentry: Arc<dyn Sentry>,
}

impl Peer {
    pub fn new(peer_id: PeerId, pool: Arc<dyn TransactionPool>, sentry: Arc<dyn Sentry>) -> Self {
        Self {
            peer_id,
            pool,
            sentry,
            requested: HashMap::new(),
            known: HashSet::new(),
            known_sorted: VecDeque::new(),
        }
    }

    // Only one public fn.
    pub async fn run_loop(&mut self, rc: &mut UnboundedReceiver<PeerMsg>) {
        //First call send known N transactions

        loop {
            let res = match rc.recv().await {
                Some(PeerMsg::InboundPooledTx(data)) => self.inbound_pooled_tx(&data).await,
                Some(PeerMsg::InboundNewPooledTxHashes(data)) => {
                    self.inbound_new_pooled_tx_hashes(&data).await
                }
                Some(PeerMsg::InboundGetPooledTxs(data)) => self.inbound_get_pooled_tx(&data).await,
                Some(PeerMsg::IncludedTxs(txs)) => self.pool_new_tx(txs).await,
                None => break,
            };
            if res.is_err() {
                // TODO penalize peer
            }
        }
    }

    async fn inbound_pooled_tx(&mut self, data: &Bytes) -> Result<()> {
        let (req, mut txs) = {
            let rlp = &Rlp::new(data);
            if rlp.size() != 2 {
                return Err(DecoderError::RlpIncorrectListLen.into());
            }
            let req_id = rlp.val_at(0)?;
            let req = self
                .requested
                .remove(&req_id)
                .ok_or(DecoderError::RlpIncorrectListLen)?; // TODO make proper err

            let txs = Transaction::rlp_decode_list(&rlp.at(1)?)?;
            (req, txs)
        };

        // recover account from txs
        for tx in txs.iter_mut() {
            let _ = tx.recover_author()?;
        }

        let mut req = req.iter();
        let mut got = txs.iter();
        // check if our request is matching with the one we asked
        while let Some(tx) = got.next() {
            let mut is_found = false;
            while let Some(&hash) = req.next() {
                if tx.hash() == hash {
                    is_found = true;
                    break;
                }
            }
            if !is_found {
                //tx is not the one we requested
                return Err(DecoderError::RlpIncorrectListLen.into()); //TODO add proper error
            }
        }

        let txs: Vec<Arc<Transaction>> = txs
            .into_iter()
            .map(|t| {
                self.insert_known(t.hash());
                Arc::new(t)
            })
            .collect();

        let _ = self.pool.import(txs).await;

        Ok(())
    }

    async fn inbound_new_pooled_tx_hashes(&mut self, data: &Bytes) -> Result<()> {
        let hashes: Vec<H256> = Rlp::new(data).as_list()?;
        hashes.iter().for_each(|hash| self.insert_known(*hash));

        let unknown_index: Vec<_> = self
            .pool
            .find(&hashes)
            .await
            .into_iter()
            .enumerate()
            .filter(|res| res.1.is_none())
            .map(|(index, _)| index)
            .collect();

        let mut rlp = RlpStream::new_list(unknown_index.len());
        for index in unknown_index {
            rlp.append(&hashes[index]);
        }
        let freeze = rlp.out().freeze();
        self.sentry
            .send_message_by_id(self.peer_id, TxMessage::GetPooledTransactions, freeze)
            .await;

        Ok(())
    }

    /// mark asked transaction as known. Ask pool to find txs and send it to peer.
    /// TODO checks on number of asked txs. Check if there is more checks that we need to do.
    async fn inbound_get_pooled_tx(&mut self, data: &Bytes) -> Result<()> {
        let hashes: Vec<H256> = Rlp::new(data).as_list()?;

        let txs: Vec<_> = self
            .pool
            .find(&hashes)
            .await
            .into_iter()
            .filter(|tx| tx.is_some())
            .map(|tx| tx.unwrap())
            .collect();

        let mut rlp = RlpStream::new();
        for tx in txs.into_iter() {
            self.insert_known(tx.hash());
            if tx.txtype() == TxType::Legacy {
                rlp.append_raw(&tx.encode(), 1);
            } else {
                rlp.append(&tx.encode());
            }
        }
        self.sentry
            .send_message_by_id(
                self.peer_id,
                TxMessage::PooledTransactions,
                rlp.out().freeze(),
            )
            .await;

        Ok(())
    }

    async fn pool_new_tx(&mut self, new: Arc<Vec<Arc<Transaction>>>) -> Result<()> {
        let mut rlp = RlpStream::new();
        rlp.begin_unbounded_list();
        for tx in new.iter() {
            let hash = &tx.hash();
            if self.is_known(hash) {
                rlp.append(hash);
                self.insert_known(*hash);
            }
        }
        rlp.finalize_unbounded_list();
        let bytes = rlp.out().freeze();
        if bytes.len() == 1 {
            // if list is empty it will contain only one byte x80
            return Ok(());
        }
        self.sentry
            .send_message_by_id(self.peer_id, TxMessage::NewPooledTransactionHashes, bytes)
            .await;
        Ok(())
    }

    fn insert_known(&mut self, hash: H256) {
        if self.known_sorted.len() > MAX_KNOWN_TX {
            let h = self.known_sorted.pop_back().unwrap();
            self.known.remove(&h);
        }
        self.known.insert(hash);
        self.known_sorted.push_front(hash);
    }

    fn is_known(&self, hash: &H256) -> bool {
        self.known.contains(hash)
    }
}
