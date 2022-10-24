use crate::data::{
    data_type::DataType, persistent_state::PersistentState, request::Request,
    volitile_state::VolitileState,
};

use super::{candidate::Candidate, follower::Follower, leader::Leader, offline::Offline};

pub struct RaftStateGeneric<StateType> {
    pub state: StateType,
    pub volitile_state: VolitileState,
}

// Makes all of the states fixed size (since I think the enum pre-allocates space for the largest).

pub enum RaftStateWrapper {
    Offline(RaftStateGeneric<Offline>),
    Candidate(RaftStateGeneric<Candidate>),
    Leader(RaftStateGeneric<Leader>),
    Follower(RaftStateGeneric<Follower>),
}
impl Default for RaftStateWrapper {
    fn default() -> Self {
        RaftStateWrapper::Offline(RaftStateGeneric {
            state: Offline {},
            volitile_state: VolitileState::default(),
        })
    }
}

impl From<RaftStateGeneric<Leader>> for RaftStateWrapper {
    fn from(offline: RaftStateGeneric<Leader>) -> Self {
        RaftStateWrapper::Leader(offline)
    }
}
impl From<RaftStateGeneric<Offline>> for RaftStateWrapper {
    fn from(offline: RaftStateGeneric<Offline>) -> Self {
        RaftStateWrapper::Offline(offline)
    }
}

impl From<RaftStateGeneric<Follower>> for RaftStateWrapper {
    fn from(offline: RaftStateGeneric<Follower>) -> Self {
        RaftStateWrapper::Follower(offline)
    }
}

impl From<RaftStateGeneric<Candidate>> for RaftStateWrapper {
    fn from(offline: RaftStateGeneric<Candidate>) -> Self {
        RaftStateWrapper::Candidate(offline)
    }
}

#[derive(Default)]
pub struct State<T: DataType> {
    pub persistent_state: PersistentState<T>,
    pub raft_state: RaftStateWrapper,
}

impl<T: DataType> State<T> {
    pub fn handle(&mut self, request: Request<T>) -> Vec<Request<T>> {
        if request.term > self.persistent_state.current_term {
            self.persistent_state.current_term = request.term;
            self.persistent_state.voted_for = None;
        }
        let (responses, next) = self.raft_state.handle(request, &mut self.persistent_state);
        if let Some(next) = next {
            self.raft_state = next;
        }
        responses
    }
}

pub trait Handler<T: DataType> {
    fn handle(
        &mut self,
        request: Request<T>,
        persistent_state: &mut PersistentState<T>,
    ) -> (Vec<Request<T>>, Option<RaftStateWrapper>);
}

impl RaftStateWrapper {
    pub fn handle<T: DataType>(
        &mut self,
        request: Request<T>,
        persistent_state: &mut PersistentState<T>,
    ) -> (Vec<Request<T>>, Option<Self>) {
        match self {
            RaftStateWrapper::Offline(offline) => offline.handle(request, persistent_state),
            RaftStateWrapper::Candidate(candidate) => candidate.handle(request, persistent_state),
            RaftStateWrapper::Leader(leader) => leader.handle(request, persistent_state),
            RaftStateWrapper::Follower(follower) => follower.handle(request, persistent_state),
        }
    }
}
