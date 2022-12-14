use crate::data::{
    data_type::{CommandType, OutputType},
    persistent_state::PersistentState,
    request::Request,
    volitile_state::VolitileState,
};

use super::{
    concrete::{candidate::Candidate, follower::Follower, leader::Leader, offline::Offline},
    handler::Handler,
    state::StateMachine,
};

#[derive(Debug, PartialEq, Clone)]
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
    pub fn handle_request<T: CommandType, Output: OutputType, SM>(
        self,
        request: Request<T, Output>,
        volitile_state: &mut VolitileState,
        persistent_state: &mut PersistentState<T>,
        state_machine: &mut SM,
    ) -> (Vec<Request<T, Output>>, Self)
    where
        SM: StateMachine<T, Output>,
    {
        match self {
            RaftState::Offline(state) => {
                state.handle_request(volitile_state, persistent_state, state_machine, request)
            }
            RaftState::Candidate(state) => {
                state.handle_request(volitile_state, persistent_state, state_machine, request)
            }
            RaftState::Leader(state) => {
                state.handle_request(volitile_state, persistent_state, state_machine, request)
            }
            RaftState::Follower(state) => {
                state.handle_request(volitile_state, persistent_state, state_machine, request)
            }
        }
    }
}
