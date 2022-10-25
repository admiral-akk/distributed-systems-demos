use std::time::{Duration, SystemTime};

use async_std::channel::Sender;

use crate::data::{
    data_type::DataType, entry::Entry, persistent_state::PersistentState, request::Request,
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
pub trait Handler<T: DataType> {
    fn append(
        &mut self,
        volitile_state: &mut VolitileState,
        persistent_state: &mut PersistentState<T>,
        sender: u32,
        term: u32,
        prev_log_length: usize,
        prev_log_term: u32,
        entries: Vec<Entry<T>>,
        leader_commit: usize,
    ) -> (Vec<Request<T>>, Option<RaftState>) {
        (Vec::default(), None)
    }

    fn append_response(
        &mut self,
        volitile_state: &mut VolitileState,
        persistent_state: &mut PersistentState<T>,
        sender: u32,
        term: u32,
        success: bool,
    ) -> (Vec<Request<T>>, Option<RaftState>) {
        (Vec::default(), None)
    }

    fn vote(
        &mut self,
        volitile_state: &mut VolitileState,
        persistent_state: &mut PersistentState<T>,
        sender: u32,
        term: u32,
        log_length: usize,
        last_log_term: u32,
    ) -> (Vec<Request<T>>, Option<RaftState>) {
        (Vec::default(), None)
    }

    fn vote_response(
        &mut self,
        volitile_state: &mut VolitileState,
        persistent_state: &mut PersistentState<T>,
        sender: u32,
        term: u32,
        success: bool,
    ) -> (Vec<Request<T>>, Option<RaftState>) {
        (Vec::default(), None)
    }

    fn timeout(
        &mut self,
        volitile_state: &mut VolitileState,
        persistent_state: &mut PersistentState<T>,
        term: u32,
    ) -> (Vec<Request<T>>, Option<RaftState>) {
        (Vec::default(), None)
    }

    fn handle(
        &mut self,
        volitile_state: &mut VolitileState,
        persistent_state: &mut PersistentState<T>,
        request: Request<T>,
    ) -> (Vec<Request<T>>, Option<RaftState>) {
        let (sender, term) = (request.sender, request.term);
        match request.data {
            crate::data::request::RequestType::Append {
                prev_log_length,
                prev_log_term,
                entries,
                leader_commit,
            } => self.append(
                volitile_state,
                persistent_state,
                sender,
                term,
                prev_log_length,
                prev_log_term,
                entries,
                leader_commit,
            ),
            crate::data::request::RequestType::AppendResponse { success } => {
                self.append_response(volitile_state, persistent_state, sender, term, success)
            }
            crate::data::request::RequestType::Vote {
                log_length,
                last_log_term,
            } => self.vote(
                volitile_state,
                persistent_state,
                sender,
                term,
                log_length,
                last_log_term,
            ),
            crate::data::request::RequestType::VoteResponse { success } => {
                self.vote_response(volitile_state, persistent_state, sender, term, success)
            }
            crate::data::request::RequestType::Timeout {} => {
                self.timeout(volitile_state, persistent_state, term)
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
    pub fn timeout(&self) -> Duration {
        Duration::from_millis(1000)
    }

    pub fn trigger_timeout(&mut self) -> Vec<Request<T>> {
        Vec::new()
    }

    pub fn handle(&mut self, request: Request<T>) -> Vec<Request<T>> {
        if request.term > self.persistent_state.current_term {
            self.persistent_state.current_term = request.term;
            self.persistent_state.voted_for = None;
            self.raft_state = RaftState::Follower(Follower::default());
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

impl RaftState {
    pub fn handle<T: DataType>(
        &mut self,
        request: Request<T>,
        volitile_state: &mut VolitileState,
        persistent_state: &mut PersistentState<T>,
    ) -> (Vec<Request<T>>, Option<Self>) {
        let handler: Box<&mut dyn Handler<T>> = match self {
            RaftState::Offline(offline) => Box::new(offline),
            RaftState::Candidate(candidate) => Box::new(candidate),
            RaftState::Leader(leader) => Box::new(leader),
            RaftState::Follower(follower) => Box::new(follower),
        };
        handler.handle(volitile_state, persistent_state, request)
    }
}
