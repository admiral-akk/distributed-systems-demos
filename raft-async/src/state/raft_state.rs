use std::time::Duration;

use crate::data::{
    data_type::DataType, persistent_state::PersistentState, request::Request,
    volitile_state::VolitileState,
};

use super::{
    concrete::{candidate::Candidate, follower::Follower, leader::Leader, offline::Offline},
    handler::{Handler, TimeoutHandler},
};
pub enum RaftState {
    Offline(Offline),
    Candidate(Candidate),
    Leader(Leader),
    Follower(Follower),
}
impl Default for RaftState {
    fn default() -> Self {
        RaftState::Offline(Offline {})
    }
}

impl From<Leader> for RaftState {
    fn from(leader: Leader) -> Self {
        RaftState::Leader(leader)
    }
}
impl From<Offline> for RaftState {
    fn from(offline: Offline) -> Self {
        RaftState::Offline(offline)
    }
}

impl From<Follower> for RaftState {
    fn from(follower: Follower) -> Self {
        RaftState::Follower(follower)
    }
}

impl From<Candidate> for RaftState {
    fn from(candidate: Candidate) -> Self {
        RaftState::Candidate(candidate)
    }
}

impl RaftState {
    pub fn timeout_length(&self) -> Duration {
        match self {
            RaftState::Offline(state) => state.timeout_length(),
            RaftState::Candidate(state) => state.timeout_length(),
            RaftState::Leader(state) => state.timeout_length(),
            RaftState::Follower(state) => state.timeout_length(),
        }
    }

    pub fn handle_request<T: DataType>(
        &mut self,
        request: Request<T>,
        volitile_state: &mut VolitileState,
        persistent_state: &mut PersistentState<T>,
    ) -> (Vec<Request<T>>, Option<Self>) {
        match self {
            RaftState::Offline(state) => {
                state.handle_request(volitile_state, persistent_state, request)
            }
            RaftState::Candidate(state) => {
                state.handle_request(volitile_state, persistent_state, request)
            }
            RaftState::Leader(state) => {
                state.handle_request(volitile_state, persistent_state, request)
            }
            RaftState::Follower(state) => {
                state.handle_request(volitile_state, persistent_state, request)
            }
        }
    }
}
