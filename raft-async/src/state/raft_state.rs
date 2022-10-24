use std::time::SystemTime;

use crate::data::{
    data_type::DataType, persistent_state::PersistentState, request::Request,
    volitile_state::VolitileState,
};

use super::{candidate::Candidate, follower::Follower, leader::Leader, offline::Offline};

pub struct RaftStateGeneric<StateType> {
    pub state: StateType,
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
        RaftStateWrapper::Offline(RaftStateGeneric { state: Offline {} })
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
    pub volitile_state: VolitileState,
}

impl<T: DataType> State<T> {
    pub fn check_timeout(&mut self) -> Vec<Request<T>> {
        let (responses, next) = self
            .raft_state
            .check_timeout(&mut self.volitile_state, &mut self.persistent_state);
        if let Some(next) = next {
            self.raft_state = next;
        }
        responses
    }

    pub fn handle(&mut self, request: Request<T>) -> Vec<Request<T>> {
        if request.term > self.persistent_state.current_term {
            self.persistent_state.current_term = request.term;
            self.persistent_state.voted_for = None;
            self.persistent_state.last_heartbeat = Some(SystemTime::now());
            self.raft_state = RaftStateWrapper::Follower(RaftStateGeneric::<Follower>::default());
        }
        let (responses, next) = self.raft_state.handle(
            request,
            &mut self.volitile_state,
            &mut self.persistent_state,
        );
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
        volitile_state: &mut VolitileState,
        persistent_state: &mut PersistentState<T>,
    ) -> (Vec<Request<T>>, Option<RaftStateWrapper>) {
        (Vec::default(), None)
    }

    fn check_timeout(
        &mut self,
        volitile_state: &mut VolitileState,
        persistent_state: &mut PersistentState<T>,
    ) -> (Vec<Request<T>>, Option<RaftStateWrapper>) {
        (Vec::default(), None)
    }
}

impl RaftStateWrapper {
    pub fn check_timeout<T: DataType>(
        &mut self,
        volitile_state: &mut VolitileState,
        persistent_state: &mut PersistentState<T>,
    ) -> (Vec<Request<T>>, Option<Self>) {
        match self {
            RaftStateWrapper::Offline(offline) => {
                offline.check_timeout(volitile_state, persistent_state)
            }
            RaftStateWrapper::Candidate(candidate) => {
                candidate.check_timeout(volitile_state, persistent_state)
            }
            RaftStateWrapper::Leader(leader) => {
                leader.check_timeout(volitile_state, persistent_state)
            }
            RaftStateWrapper::Follower(follower) => {
                follower.check_timeout(volitile_state, persistent_state)
            }
        }
    }

    pub fn handle<T: DataType>(
        &mut self,
        request: Request<T>,
        volitile_state: &mut VolitileState,
        persistent_state: &mut PersistentState<T>,
    ) -> (Vec<Request<T>>, Option<Self>) {
        match self {
            RaftStateWrapper::Offline(offline) => {
                offline.handle(request, volitile_state, persistent_state)
            }
            RaftStateWrapper::Candidate(candidate) => {
                candidate.handle(request, volitile_state, persistent_state)
            }
            RaftStateWrapper::Leader(leader) => {
                leader.handle(request, volitile_state, persistent_state)
            }
            RaftStateWrapper::Follower(follower) => {
                follower.handle(request, volitile_state, persistent_state)
            }
        }
    }
}
