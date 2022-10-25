use std::{collections::HashSet, time::Duration};

use super::{
    follower::Follower,
    leader::Leader,
    raft_state::{EventHandler, Handler, RaftState, TimeoutHandler},
};
use crate::data::{
    data_type::DataType,
    persistent_state::PersistentState,
    request::{Append, AppendResponse, Request, Timeout, Vote, VoteResponse},
    volitile_state::VolitileState,
};

#[derive(Default)]
pub struct Candidate {
    votes: HashSet<u32>,
}
impl TimeoutHandler for Candidate {
    fn timeout_length(&self) -> Duration {
        Duration::from_millis(3000)
    }
}

impl<T: DataType> Handler<T> for Candidate {}
impl<T: DataType> EventHandler<Vote, T> for Candidate {}
impl<T: DataType> EventHandler<AppendResponse, T> for Candidate {}
impl<T: DataType> EventHandler<Timeout, T> for Candidate {}

impl<T: DataType> EventHandler<Append<T>, T> for Candidate {
    fn handle_event(
        &mut self,
        volitile_state: &mut VolitileState,
        persistent_state: &mut PersistentState<T>,
        sender: u32,
        term: u32,
        event: Append<T>,
    ) -> (Vec<Request<T>>, Option<RaftState>) {
        if term >= persistent_state.current_term {
            return (Vec::default(), Some(Follower::default().into()));
        }
        (Vec::default(), None)
    }
}
impl<T: DataType> EventHandler<VoteResponse, T> for Candidate {
    fn handle_event(
        &mut self,
        volitile_state: &mut VolitileState,
        persistent_state: &mut PersistentState<T>,
        sender: u32,
        term: u32,
        event: VoteResponse,
    ) -> (Vec<Request<T>>, Option<RaftState>) {
        if event.success {
            println!("{} voted for {}", sender, persistent_state.id);
            self.votes.insert(sender);
        }
        if self.votes.len() > persistent_state.quorum() {
            return Leader::from_candidate(&self, volitile_state, persistent_state);
        }
        (Vec::default(), None)
    }
}

impl Candidate {
    pub fn call_election<T: DataType>(
        persistent_state: &mut PersistentState<T>,
    ) -> (Vec<Request<T>>, Option<RaftState>) {
        println!("{} running for office!", persistent_state.id);
        persistent_state.current_term += 1;
        persistent_state.voted_for = Some(persistent_state.id);
        persistent_state.keep_alive += 1;
        (Vec::new(), Some(Candidate::default().into()))
    }
}
