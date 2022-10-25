use std::time::{Duration, SystemTime};

use async_std::channel::Sender;

use crate::data::{
    data_type::DataType,
    entry::Entry,
    persistent_state::PersistentState,
    request::{Append, AppendResponse, Request, Timeout, Vote, VoteResponse},
    volitile_state::VolitileState,
};

use super::{candidate::Candidate, follower::Follower, leader::Leader, offline::Offline};

// Makes all of the states fixed size (since I think the enum pre-allocates space for the largest).
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
pub trait TimeoutHandler {
    fn timeout_length(&self) -> Duration;
}

pub trait EventHandler<EventType, T: DataType> {
    fn handle_event(
        &mut self,
        volitile_state: &mut VolitileState,
        persistent_state: &mut PersistentState<T>,
        sender: u32,
        term: u32,
        event: EventType,
    ) -> (Vec<Request<T>>, Option<RaftState>) {
        (Vec::default(), None)
    }
}

pub trait Handler<T: DataType>:
    EventHandler<Append<T>, T>
    + EventHandler<AppendResponse, T>
    + EventHandler<Timeout, T>
    + EventHandler<Vote, T>
    + EventHandler<VoteResponse, T>
{
    fn handle_request(
        &mut self,
        volitile_state: &mut VolitileState,
        persistent_state: &mut PersistentState<T>,
        request: Request<T>,
    ) -> (Vec<Request<T>>, Option<RaftState>) {
        let (sender, term) = (request.sender, request.term);
        match request.event {
            crate::data::request::Event::Append(event) => {
                self.handle_event(volitile_state, persistent_state, sender, term, event)
            }
            crate::data::request::Event::AppendResponse(event) => {
                self.handle_event(volitile_state, persistent_state, sender, term, event)
            }
            crate::data::request::Event::Vote(event) => {
                self.handle_event(volitile_state, persistent_state, sender, term, event)
            }
            crate::data::request::Event::VoteResponse(event) => {
                self.handle_event(volitile_state, persistent_state, sender, term, event)
            }
            crate::data::request::Event::Timeout(event) => {
                self.handle_event(volitile_state, persistent_state, sender, term, event)
            }
        }
    }
}

pub struct State<T: DataType> {
    pub persistent_state: PersistentState<T>,
    pub raft_state: RaftState,
    pub volitile_state: VolitileState,
}

impl<T: DataType> State<T> {
    pub fn timeout_length(&self) -> Duration {
        self.raft_state.timeout_length()
    }

    pub fn handle_request(&mut self, request: Request<T>) -> Vec<Request<T>> {
        if request.term > self.persistent_state.current_term {
            self.persistent_state.current_term = request.term;
            self.persistent_state.voted_for = None;
            self.raft_state = RaftState::Follower(Follower::default());
        }
        let (responses, next) = self.raft_state.handle_request(
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
