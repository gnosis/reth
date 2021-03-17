// Copyright 2020 Gnosis Ltd.
// SPDX-License-Identifier: Apache-2.0

use super::{
    handshake::HandshakeInfo,
    protocol::{EthMessageId, MessageId, ProtocolId},
};
use crate::devp2p_adapter::{adapter::Devp2pAdapter, PeerPenal};
use std::{
    collections::{HashMap, HashSet},
    sync::{atomic::AtomicUsize, Arc, Mutex},
    time::{Duration, Instant},
};

#[derive(Debug)]
pub struct CustomError {
    msg: String,
}

impl CustomError {
    pub fn new(msg: &str) -> CustomError {
        CustomError {
            msg: msg.to_string(),
        }
    }
}

impl std::fmt::Display for CustomError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "CustomError:{}", self.msg)
    }
}

#[derive(Debug)]
pub struct InitialRequest {
    pub message_id: EthMessageId,
    pub data: MessageData,
}

impl InitialRequest {
    pub fn new(message_id: EthMessageId, data: MessageData) -> Self {
        InitialRequest { message_id, data }
    }
}

#[derive(Debug, Clone)]
pub enum Task {
    InsertPeer(HandshakeInfo),
    PenalPeer(PeerId, PeerPenal, String), //last is reason
    WaitForStatus(PeerId, MessageData),
    InitialRequest(PeerId, EthMessageId, MessageData),
    Responde(PeerId, ProtocolId, MessageId, Vec<u8>),
    None,
}

#[derive(Debug)]
pub struct ErrorAct {
    penal: PeerPenal,
    reason: String,
}

impl ErrorAct {
    pub fn new(penal: PeerPenal, reason: String) -> Result<(), ErrorAct> {
        Err(ErrorAct {
            penal,
            reason: reason,
        })
    }

    pub fn new_kick(reason: String) -> Result<(), ErrorAct> {
        Err(ErrorAct {
            penal: PeerPenal::Kick,
            reason: reason,
        })
    }

    pub fn new_kick_generic<T>(reason: String) -> Result<T, ErrorAct> {
        Err(ErrorAct {
            penal: PeerPenal::Kick,
            reason: reason,
        })
    }

    pub fn penal(&self) -> PeerPenal {
        self.penal
    }

    pub fn reason(&self) -> String {
        self.reason.clone()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum TaskType {
    SendMsg,
    StatusMsg,
    ResponseMsg,
    None,
}

static GLOBAL_TASK_ID: AtomicUsize = AtomicUsize::new(1);

impl Task {
    pub fn new_kick(peer: &PeerId, msg: String) -> Task {
        Task::PenalPeer(*peer, PeerPenal::Kick, msg)
    }

    pub fn task_type(&self) -> TaskType {
        match self {
            Self::InsertPeer(_) => TaskType::SendMsg,
            Self::PenalPeer(_, _, _) => TaskType::SendMsg,
            Self::WaitForStatus(_, _) => TaskType::StatusMsg,
            Self::InitialRequest(_, _, _) => TaskType::SendMsg,
            Self::Responde(_, _, _, _) => TaskType::ResponseMsg,
            Self::None => TaskType::None,
        }
    }

    pub fn peer_id(&self) -> Option<PeerId> {
        match self {
            Self::InsertPeer(_) => None,
            Self::PenalPeer(peer_id, _, _) => Some(*peer_id),
            Self::WaitForStatus(peer_id, _) => Some(*peer_id),
            Self::InitialRequest(peer_id, _, _) => Some(*peer_id),
            Self::Responde(peer_id, _, _, _) => Some(*peer_id),
            Self::None => None,
        }
    }

    pub fn max_retries(&self) -> usize {
        match self {
            Self::InsertPeer(_) => 0,
            Self::PenalPeer(_, _, _) => 0,
            Self::WaitForStatus(_, _) => 0,
            Self::InitialRequest(_, _, _) => 0,
            Self::Responde(_, _, _, _) => 0,
            Self::None => 0,
        }
    }

    pub fn timelimit(&self) -> Option<Duration> {
        match self {
            Self::InsertPeer(_) => None,
            Self::PenalPeer(_, _, _) => None,
            Self::InitialRequest(_, _, _) => None,
            Self::Responde(_, _, _, _) => None,
            Self::WaitForStatus(_, _) => Some(Duration::from_millis(3000)), //timeout after not receiving status msg from peer
            Self::None => None,
        }
    }

    pub fn new_id() -> TaskId {
        GLOBAL_TASK_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    }
}
#[derive(Debug)]
pub struct TaskWrapper {
    task: Task,
    retries: usize, // retrie by sending task to different peer
    timestamp: Instant,
}

impl TaskWrapper {
    pub fn new(task: Task) -> TaskWrapper {
        let max_retries = task.max_retries();
        TaskWrapper {
            task,
            retries: max_retries,
            timestamp: Instant::now(),
        }
    }

    pub fn retry(&mut self) -> bool {
        if self.retries == 0 {
            false
        } else {
            self.retries -= 1;
            true
        }
    }

    pub fn timeouted(&self, now: &Instant) -> bool {
        match self.task.timelimit() {
            Some(timelimit) => self.timestamp + timelimit < *now,
            None => false,
        }
    }
}

pub type TaskId = usize;

pub type PeerId = usize;

pub type PeerCapability = HashMap<ProtocolId, HashSet<u8>>;

pub type MessageData = Vec<u8>;

pub struct Peer {
    peer_id: PeerId,
    info: PeerInfo,
    tasks: HashSet<TaskId>,
}

// TODO expend this to cover all needed information fields
// all field here should be one that are persistent.
pub struct PeerInfo {
    network_id: u64,
}

impl From<HandshakeInfo> for Peer {
    fn from(hi: HandshakeInfo) -> Self {
        Peer {
            peer_id: hi.peer_id,
            tasks: HashSet::new(),
            info: PeerInfo {
                network_id: hi.network_id,
            },
        }
    }
}
pub struct PeerOrganizer {
    peers: HashMap<PeerId, Peer>,
    pending_tasks: HashMap<TaskId, TaskWrapper>,
    devp2p: Arc<Box<dyn Devp2pAdapter>>,
}

impl PeerOrganizer {
    pub fn peers(&self) -> &HashMap<PeerId, Peer> {
        &self.peers
    }

    fn free_peer(&self) -> Option<PeerId> {
        for peer in self.peers.keys() {
            let peer_tasks = self.peers.get(peer).unwrap().tasks.len();
            if peer_tasks == 0 {
                return Some(*peer);
            }
        }
        None
    }

    pub fn schedule_to_free_peer(&mut self, request: InitialRequest) {
        if let Some(ref peer_id) = self.free_peer() {
            let task = Task::InitialRequest(*peer_id, request.message_id, request.data);
            let task_id = Task::new_id();
            let peers_tasks = &mut self.peers.get_mut(peer_id).unwrap().tasks;
            peers_tasks.insert(task_id);
            self.push_task(task, Some(task_id));
        } else {
            info!("No free peer to schedule task {:?} to", &request);
        }
    }

    pub fn random_peer(&self) -> Option<PeerId> {
        match self.peers.keys().next() {
            Some(peer) => Some(*peer),
            None => None,
        }
    }

    pub fn schedule(&mut self, task: Task) {
        let task_id = Task::new_id();
        let peer_id = &task.peer_id().unwrap();
        let peers_tasks = &mut self.peers.get_mut(peer_id).unwrap().tasks;
        if peers_tasks.len() == 0 {
            peers_tasks.insert(task_id);
        }
        self.push_task(task, Some(task_id));
    }

    pub fn new(devp2p: Arc<Box<dyn Devp2pAdapter>>) -> Arc<Mutex<PeerOrganizer>> {
        let peer_org = Arc::new(Mutex::new(PeerOrganizer {
            peers: HashMap::new(),
            pending_tasks: HashMap::new(),
            devp2p,
        }));

        peer_org
    }

    pub fn start(&self) {
        self.devp2p.start();
    }

    pub fn stop(&self) {
        self.devp2p.stop();
    }

    pub fn tick(&mut self) -> Vec<Task> {
        let now = Instant::now();
        let mut timeouted_tasks = Vec::new();
        let mut rem_ids = Vec::new();
        for (id, task) in self.pending_tasks.iter_mut() {
            //check timeout, and if timeouted call organizer to notify managers that request task was not successfull
            if task.timeouted(&now) {
                //disconnect particular peer, nulify all its pending tasks and retry all his pending tasks.

                // if we cant retry tasks send them to failed_task array.
                timeouted_tasks.push(task.task.clone());
                rem_ids.push(id.clone());
            }
        }

        for rem_id in rem_ids {
            self.pending_tasks.remove(&rem_id);
        }
        timeouted_tasks
    }

    // Checks if response is expected. This related to older <eth/65 protocols without requests_id,
    // It is expected for peer to have only one pending task
    pub fn check_response(&mut self, peer: &PeerId, _message_id: MessageId) -> bool {
        let task_id = match self.peers.get_mut(peer) {
            Some(peer) => {
                // expects only one task for older protocol
                if peer.tasks.len() != 1 {
                    //disconnect
                    return false;
                }
                peer.tasks.drain().next().unwrap()
            }
            None => {
                return false;
            }
        };

        trace!("peers:{} task_id:{} removed", peer, task_id);
        match self.pending_tasks.remove(&task_id) {
            Some(_) => {
                // TODO check if task has same message_id that is expected
                // with message_id
                return true;
            }
            None => {
                error!("Unexpected thing happened, peer tasks should be present in pending_tasks");
                return false;
            }
        }
    }

    /// check if this message is expected response from peer.
    /// We need to check several things. If TaskId is present we assume we are using eth/66 or higher protocol.
    /// and then we just need to check pending_tasks map if task is present and cross check it with peer id.
    /// If TaskId is empty we try to find task from peer[i], and expect for peer only to have one task.
    /// After we confirmed task is present remove it from pending_tasks and peer[id].tasks.
    pub fn check_response_with_task_id(
        &mut self,
        peer: &PeerId,
        task_type: TaskType,
        task_id: &TaskId,
    ) -> bool {
        let task = self.pending_tasks.remove(task_id);
        match task {
            Some(task) => {
                if task.task.task_type() != task_type {
                    self.pending_tasks.insert(*task_id, task); // should not happen often, reinsert task
                    return false;
                }
                if let Some(task_peer) = task.task.peer_id() {
                    if task_peer != *peer {
                        self.pending_tasks.insert(*task_id, task); // should not happend often, reinsert task
                        return false;
                    }
                }

                // handshake task is special, and we skips peer check
                if task_type == TaskType::StatusMsg {
                    return true;
                }

                // TODO remove task from peer self self.peers.get_mut(peer).

                return false;
            }
            None => {
                return false;
            }
        }
    }

    pub fn push_task(&mut self, mut task: Task, task_id: Option<TaskId>) -> Option<TaskId> {
        let task_id = match task {
            Task::InsertPeer(hi) => {
                info!("Peer inserted: {:?}", task);
                self.peers.insert(hi.peer_id, Peer::from(hi));
                None
            }
            Task::PenalPeer(peer, _penal, ref reason) => {
                debug!("Peer penalized. Reason:{}", reason);
                self.disconnect(&peer); // for now just disconnect all peers

                None
            }
            Task::WaitForStatus(ref peer, ref mut data) => {
                self.devp2p
                    .send_mesage(ProtocolId::Eth, peer, EthMessageId::Status as u8, &data);
                data.clear();
                if task_id.is_none() {
                    panic!("Task id should be set for status msg");
                }
                task_id
            }
            Task::InitialRequest(ref peer, ref message_id, ref mut data) => {
                self.devp2p
                    .send_mesage(ProtocolId::Eth, peer, *message_id as u8, &data);
                data.clear();
                if task_id.is_none() {
                    panic!("Task id should be set for InitialRequest msg");
                }
                task_id
            }
            Task::Responde(ref peer_id, protocol, msg_id, ref mut msg) => {
                self.devp2p.send_mesage(
                    protocol,
                    peer_id,
                    msg_id.to_u8(),
                    &std::mem::replace(msg, vec![]),
                );
                None
            }
            Task::None => return None,
        };
        if let Some(task_id) = task_id {
            self.pending_tasks
                .insert(task_id, TaskWrapper::new(task.clone()));
        }
        task_id
    }

    pub fn remove_task(&mut self, task_id: &TaskId) {
        self.pending_tasks.remove(task_id);
    }

    //disconnect peer.
    pub fn disconnect(&mut self, peer_id: &PeerId) {
        if let Some(peer) = self.peers.remove(peer_id) {
            for task_id in peer.tasks {
                // should we remove task, or do retrasmision. Best way is to naturally timeout it! TODO.
                // For now lets remove it
                if let Some(task) = self.pending_tasks.get_mut(&task_id) {
                    // sub timestamp with arbitrary big number. It should be bigger then biggest message timeout. 1000 does a job.
                    task.timestamp = task.timestamp - Duration::from_secs(1000);
                }
            }
        }
        self.devp2p.penalize_peer(peer_id, PeerPenal::Kick);
    }
}
