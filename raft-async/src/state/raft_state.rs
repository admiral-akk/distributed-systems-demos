use std::collections::HashSet;

use crate::data::{
    data_type::DataType,
    entry::Entry,
    request::{Request, RequestType},
};

use super::{candidate::Candidate, follower::Follower, leader::Leader, offline::Offline};

pub struct RaftStateGeneric<T: DataType, StateType> {
    pub state: StateType,
    pub persistent_state: PersistentState<T>,
    pub volitile_state: VolitileState,
}

// Makes all of the states fixed size (since I think the enum pre-allocates space for the largest).
pub enum RaftStateWrapper<T: DataType> {
    Offline(RaftStateGeneric<T, Offline>),
    Candidate(RaftStateGeneric<T, Candidate>),
    Leader(RaftStateGeneric<T, Leader>),
    Follower(RaftStateGeneric<T, Follower>),
}
impl<T: DataType> From<RaftStateGeneric<T, Leader>> for RaftStateWrapper<T> {
    fn from(offline: RaftStateGeneric<T, Leader>) -> Self {
        RaftStateWrapper::Leader(offline)
    }
}
impl<T: DataType> From<RaftStateGeneric<T, Offline>> for RaftStateWrapper<T> {
    fn from(offline: RaftStateGeneric<T, Offline>) -> Self {
        RaftStateWrapper::Offline(offline)
    }
}

impl<T: DataType> From<RaftStateGeneric<T, Follower>> for RaftStateWrapper<T> {
    fn from(offline: RaftStateGeneric<T, Follower>) -> Self {
        RaftStateWrapper::Follower(offline)
    }
}

impl<T: DataType> From<RaftStateGeneric<T, Candidate>> for RaftStateWrapper<T> {
    fn from(offline: RaftStateGeneric<T, Candidate>) -> Self {
        RaftStateWrapper::Candidate(offline)
    }
}

impl<T: DataType> RaftStateWrapper<T> {
    fn handle(mut self, request: Request<T>) -> Self {
        let (requests, next) = match self {
            RaftStateWrapper::Offline(offline) => offline.handle(request),
            RaftStateWrapper::Candidate(candidate) => candidate.handle(request),
            RaftStateWrapper::Leader(leader) => leader.handle(request),
            RaftStateWrapper::Follower(follower) => follower.handle(request),
        };
        // Handle requests

        next
    }
}

pub struct Config {
    pub servers: HashSet<u32>,
}

pub struct PersistentState<T: DataType> {
    pub id: u32,
    pub current_term: u32,
    pub voted_for: Option<u32>,
    pub log: Vec<Entry<T>>,
    pub config: Config,
}

impl<T: DataType> PersistentState<T> {
    pub fn append(&self, volitile_state: &VolitileState, index: usize, server: u32) -> Request<T> {
        Request {
            sender: self.id,
            term: self.current_term,
            data: RequestType::Append {
                prev_log_length: index,
                prev_log_term: match self.log.len() >= index {
                    true => 0,
                    false => self.log[index].term,
                },
                entries: Vec::from(&self.log[index..self.log.len()]),
                leader_commit: volitile_state.commit_index,
            },
        }
    }

    pub fn quorum(&self) -> usize {
        self.config.servers.len() / 2 + 1
    }

    pub fn other_servers(&self) -> Vec<u32> {
        self.config
            .servers
            .iter()
            .filter(|id| !self.id.eq(id))
            .map(|id| *id)
            .collect()
    }
}

#[derive(Default)]
pub struct VolitileState {
    pub commit_index: usize,
}
