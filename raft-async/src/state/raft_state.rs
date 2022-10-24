use crate::data::{entry::Entry, request::Request};

use super::{candidate::Candidate, follower::Follower, leader::Leader, offline::Offline};

pub struct RaftStateGeneric<DataType, StateType> {
    pub state: StateType,
    pub persistent_state: PersistentState<DataType>,
    pub volitile_state: VolitileState,
}

pub enum RaftStateWrapper<DataType> {
    Offline(RaftStateGeneric<DataType, Offline>),
    Candidate(RaftStateGeneric<DataType, Candidate>),
    Leader(RaftStateGeneric<DataType, Leader>),
    Follower(RaftStateGeneric<DataType, Follower>),
}
impl<DataType> From<RaftStateGeneric<DataType, Leader>> for RaftStateWrapper<DataType> {
    fn from(offline: RaftStateGeneric<DataType, Leader>) -> Self {
        RaftStateWrapper::Leader(offline)
    }
}
impl<DataType> From<RaftStateGeneric<DataType, Offline>> for RaftStateWrapper<DataType> {
    fn from(offline: RaftStateGeneric<DataType, Offline>) -> Self {
        RaftStateWrapper::Offline(offline)
    }
}

impl<DataType> From<RaftStateGeneric<DataType, Follower>> for RaftStateWrapper<DataType> {
    fn from(offline: RaftStateGeneric<DataType, Follower>) -> Self {
        RaftStateWrapper::Follower(offline)
    }
}

impl<DataType> From<RaftStateGeneric<DataType, Candidate>> for RaftStateWrapper<DataType> {
    fn from(offline: RaftStateGeneric<DataType, Candidate>) -> Self {
        RaftStateWrapper::Candidate(offline)
    }
}

impl<DataType> RaftStateWrapper<DataType> {
    fn handle(mut self, request: Request<DataType>) -> Self {
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

pub struct PersistentState<DataType> {
    pub current_term: u32,
    pub voted_for: Option<u32>,
    pub log: Vec<Entry<DataType>>,
}

#[derive(Default)]
pub struct VolitileState {
    pub commit_index: usize,
}
