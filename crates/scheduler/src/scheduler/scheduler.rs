// Copyright 2020 Gnosis Ltd.
// SPDX-License-Identifier: Apache-2.0

use super::{
    handshake::Handshake,
    peer_organizer::{ErrorAct, PeerCapability, PeerId, PeerOrganizer, Task, TaskType},
    protocol::{EthMessageId, MessageId, ParityMessageId, ProtocolId},
};
use crate::{
    block_manager::BlockManager,
    client_adapter::{
        Blockchain,
        client_info::{Client, Snapshot},
        headers_in_memory::HeadersInMemory,
    },
    devp2p_adapter::{
        adapter::{Devp2pAdapter, Devp2pInbound},
        PeerPenal,
    },
};
use log::*;
use std::{
    sync::{
        mpsc::{channel, RecvTimeoutError, Sender},
        Arc, Condvar, Mutex,
    },
    thread,
    time::Duration,
};

pub enum SchedulerState {
    WaitingPeer,
    Warping,
    ActiveSync,
    PassiveSync,
}

pub struct Scheduler {
    handshake: Mutex<Handshake>,
    state: Mutex<SchedulerState>,

    peer_organizer: Arc<Mutex<PeerOrganizer>>,
    client: Arc<dyn Client>,
    snapshot: Arc<dyn Snapshot>,

    block_manager: Arc<Mutex<BlockManager>>,
    //pending_packages: u32,
    /*
    block_manager,
    snapshot_manager
    transaction_manager,
    brodcaster,
    PendingMessages
    */
    // peer org thread,
    // organizer thread.
    main_loop_trigger: Mutex<Sender<LoopMsg>>,
    thread_handle: Mutex<Option<thread::JoinHandle<()>>>,
}

pub enum LoopMsg {
    TrigerLoop,
    EndLoop,
}

impl Scheduler {
    pub fn new(
        devp2p: Box<dyn Devp2pAdapter>,
        client: Arc<dyn Client>,
        snapshot: Arc<dyn Snapshot>,
    ) -> Arc<Scheduler> {
        let devp2p = Arc::new(devp2p);
        let (tx, rx) = channel::<LoopMsg>();
        let chain = Arc::new(Mutex::new(HeadersInMemory::new()));
        let peer_organizer = PeerOrganizer::new(devp2p.clone());
        let block_manager = BlockManager::new(chain);
        let org = Arc::new(Scheduler {
            peer_organizer: peer_organizer,
            state: Mutex::new(SchedulerState::WaitingPeer),
            handshake: Mutex::new(Handshake::new()),
            block_manager: block_manager,
            main_loop_trigger: Mutex::new(tx),
            thread_handle: Mutex::new(None),
            client,
            snapshot,
        });
        let org_exec = org.clone();
        *(org.thread_handle.lock().unwrap()) = Some(
            thread::Builder::new()
                .name("Scheduler".to_string())
                .spawn(move || loop {
                    {
                        match rx.recv_timeout(Duration::from_secs(1)) {
                            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => (),
                            Ok(LoopMsg::TrigerLoop) => (),
                            Ok(LoopMsg::EndLoop) => break,
                            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
                        }
                        org_exec.main_loop();
                    }
                })
                .expect("Expect to run thread"),
        );
        let org_handler = org.clone();
        devp2p.register_handler(org_handler);
        org
    }

    pub fn start(&self) {
        self.peer_organizer.lock().unwrap().start();
    }

    pub fn stop(&self) {
        let handle = {
            self.main_loop_trigger
                .lock()
                .unwrap()
                .send(LoopMsg::EndLoop)
                .unwrap();
            self.thread_handle.lock().unwrap().take()
        };
        self.peer_organizer.lock().unwrap().stop();
        if let Some(handle) = handle {
            handle.join().expect("Expect for thread to end gracefully.");
        }
        //TODO clean all states
    }

    pub fn main_loop(&self) {
        let mut org = self.peer_organizer.lock().unwrap();
        let block_mgr = self.block_manager.lock().unwrap();
        if let Some(task) = block_mgr.next_sync_task() {
            org.schedule_to_free_peer(task);
        }
        let failed_tasks = org.tick();
        if failed_tasks.len() != 0 {
            info!("Failed tasks: {:?}", failed_tasks);
        }
        for fail_task in failed_tasks.iter() {
            match fail_task {
                Task::WaitForStatus(peer, _) => {
                    org.push_task(
                        Task::PenalPeer(*peer, PeerPenal::Kick, "Timeouted".to_string()),
                        None,
                    );
                }
                _ => (),
            }
        }
        if org.peers().len() != 0 {
            info!("Current peer number:{}", org.peers().len());
        }
        //wait for n number of peer
        /*
            match self.state {
            WaitingPeer => //check timeout or N number of peers,
            Warping => self.snapshot_manager.i_want(),
            ActiveSync => self.block_manager.i_want(),
            PassiveSync => {
                self.block_manager.i_want()
                self.transaction_manager.i_want()
            }
        }
        */
    }

    fn process_eth_message(
        &self,
        id: EthMessageId,
        peer: &PeerId,
        data: &[u8],
    ) -> Result<Task, ErrorAct> {
        match id {
            EthMessageId::Status => {
                let mut handshake = self.handshake.lock().unwrap();
                let peer_handshake = handshake.peers.get(peer).clone();
                if let Some((task_id, _)) = peer_handshake {
                    // handshake has specific flow
                    // this should be only place where we interlock handshake and peer_organizer
                    let mut org = self.peer_organizer.lock().unwrap();
                    if org.check_response_with_task_id(peer, TaskType::StatusMsg, task_id) {
                        org.push_task(
                            handshake
                                .handle_status_message(peer, data)
                                .unwrap_or_else(|act| {
                                    Task::PenalPeer(*peer, act.penal(), act.reason())
                                }),
                            None,
                        );
                    };
                };
            }
            EthMessageId::NewBlockHashes => {
                info!("Got NewBlockHashes message from {}", peer);
                self.block_manager.lock().unwrap().api_new_block_hashes(peer, data);
            }
            EthMessageId::Transactions => {}
            EthMessageId::GetBlockHeaders => {
                info!("Responding peer {} with dummy BlockHeaders message", peer);
                return self.block_manager.lock().unwrap().api_get_block_headers(peer, &data);
            }
            EthMessageId::BlockHeaders => {
                info!("Got BlockHeaders message from {}", peer);
                self.block_manager.lock().unwrap().process_block_headers(&data);
            }
            EthMessageId::GetBlockBodies => {
                info!("Responding peer {} with dummy BlockBodies message", peer);
                return self.block_manager.lock().unwrap().api_get_block_bodies(peer, &data);
            }
            EthMessageId::BlockBodies => {
                info!("Got BlockBodies message from {} with {} bytes", peer, data.len());
                self.block_manager.lock().unwrap().process_block_bodies(&data);
            }
            EthMessageId::NewBlock => {
                info!("Got NewBlock message from {} with {} bytes", peer, data.len());
                self.block_manager.lock().unwrap().api_new_block_hashes(peer, data);
            }
            // NewPooledTransactionHashes = 0x08, // eth/65 protocol
            // GetPooledTransactions = 0x09, // eth/65 protocol
            // PooledTransactions  = 0x0a, // eth/65 protocol
            //EthMessageId::GetNodeData => {} // ommited it can overburder client.
            //EthMessageId::NodeData => {}    // ommited it can overburder client
            EthMessageId::GetReceipts => {}
            EthMessageId::Receipts => {}
        }
        Ok(Task::None)
    }
}

impl Devp2pInbound for Scheduler {
    /// Called when new network packet received.
    fn receive_message(&self, peer: &PeerId, protocol_id: ProtocolId, message_id: u8, data: &[u8]) {
        info!(
            "recv msg: peer:{} msg:{}, ver:{:?}",
            peer, message_id, protocol_id
        );
        match protocol_id {
            ProtocolId::Eth => {
                // transform message id
                let message_id: Option<EthMessageId> = num::FromPrimitive::from_u8(message_id);
                let message_id = match message_id {
                    Some(id) => id,
                    None => return, //TODO disconnect peer. but for now just ignore it.
                };

                if message_id.is_response() {
                    if !self
                        .peer_organizer
                        .lock()
                        .unwrap()
                        .check_response(peer, MessageId::Eth(message_id))
                    {
                        return;
                    }
                }

                let tasks = self.process_eth_message(message_id, peer, data);
                let mut peer_org = self.peer_organizer.lock().unwrap();
                for task in tasks {
                    peer_org.push_task(task, None);
                }
            }
            ProtocolId::Parity => {
                // transform message id
                let message_id: Option<ParityMessageId> = num::FromPrimitive::from_u8(message_id);
                let message_id = match message_id {
                    Some(id) => id,
                    None => return, //TODO disconnect peer. but for now just ignore it.
                };

                if message_id.is_response() {
                    if !self
                        .peer_organizer
                        .lock()
                        .unwrap()
                        .check_response(peer, MessageId::Parity(message_id))
                    {
                        return;
                    }
                }

                match message_id {
                    ParityMessageId::GetSnapshotManifest => {}
                    ParityMessageId::SnapshotManifest => {}
                    ParityMessageId::GetSnapshotData => {}
                    ParityMessageId::SnapshotData => {}
                    ParityMessageId::ConsensusData => {}
                }
            }
        }
    }
    /// Called when new peer is connected. Only called when peer supports the same protocol.
    fn connected(&self, peer: &PeerId, capability: &PeerCapability) {
        let client_status = self.client.status();
        let snapshot_manifest_status = self.snapshot.manifest_status();
        let task_id = Task::new_id();
        info!("Peer connected with capa:{:?}", capability);
        let data = self
            .handshake
            .lock()
            .unwrap()
            .connect_and_create_status_message(
                peer,
                task_id,
                capability,
                &client_status,
                snapshot_manifest_status,
            );
        self.peer_organizer
            .lock()
            .unwrap()
            .push_task(Task::WaitForStatus(*peer, data), Some(task_id));
    }

    /// Called when a previously connected peer disconnects.
    fn disconnected(&self, peer: &PeerId) {
        info!("disconnected:{}", peer);
        let task_id = self.handshake.lock().unwrap().disconnect(peer);

        let mut peer_org = self.peer_organizer.lock().unwrap();
        match task_id {
            Some(task_id) => peer_org.remove_task(&task_id),
            None => peer_org.disconnect(peer),
        }
    }
}
