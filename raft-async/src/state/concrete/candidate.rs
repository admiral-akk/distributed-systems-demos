use std::{collections::HashSet, time::Duration};

use super::{follower::Follower, leader::Leader};
use crate::data::{
    data_type::DataType,
    persistent_state::PersistentState,
    request::{
        Append, AppendResponse, Client, ClientResponse, Event, Request, Timeout, Vote, VoteResponse,
    },
    volitile_state::VolitileState,
};
use crate::state::{
    handler::{EventHandler, Handler, TimeoutHandler},
    raft_state::RaftState,
};

#[derive(Default)]
pub struct Candidate {
    attempts: u32,
    votes: HashSet<u32>,
}
impl TimeoutHandler for Candidate {
    fn timeout_length(&self) -> Duration {
        Duration::from_millis(200)
    }
}

impl<T: DataType> Handler<T> for Candidate {}
impl<T: DataType> EventHandler<Vote, T> for Candidate {}
impl<T: DataType> EventHandler<Client<T>, T> for Candidate {}
impl<T: DataType> EventHandler<ClientResponse<T>, T> for Candidate {}
impl<T: DataType> EventHandler<AppendResponse, T> for Candidate {}
impl<T: DataType> EventHandler<Timeout, T> for Candidate {
    fn handle_event(
        &mut self,
        _volitile_state: &mut VolitileState,
        persistent_state: &mut PersistentState<T>,
        _sender: u32,
        _term: u32,
        _event: Timeout,
    ) -> (Vec<Request<T>>, Option<RaftState>) {
        self.attempts += 1;
        if self.attempts > 10 {
            Candidate::call_election(persistent_state)
        } else {
            (Candidate::request_votes(persistent_state), None)
        }
    }
}

impl<T: DataType> EventHandler<Append<T>, T> for Candidate {
    fn handle_event(
        &mut self,
        _volitile_state: &mut VolitileState,
        persistent_state: &mut PersistentState<T>,
        _sender: u32,
        term: u32,
        _event: Append<T>,
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
        _term: u32,
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
    fn request_votes<T: DataType>(persistent_state: &mut PersistentState<T>) -> Vec<Request<T>> {
        persistent_state
            .other_servers()
            .iter()
            .map(|id| Request::<T> {
                sender: persistent_state.id,
                reciever: *id,
                term: persistent_state.current_term,
                event: Event::Vote(Vote {
                    log_length: persistent_state.log.len(),
                    last_log_term: persistent_state.prev_term(persistent_state.log.len()),
                }),
            })
            .collect()
    }
    pub fn call_election<T: DataType>(
        persistent_state: &mut PersistentState<T>,
    ) -> (Vec<Request<T>>, Option<RaftState>) {
        println!("{} running for office!", persistent_state.id);
        persistent_state.current_term += 1;
        persistent_state.voted_for = Some(persistent_state.id);
        persistent_state.keep_alive += 1;
        (
            Candidate::request_votes(persistent_state),
            Some(Candidate::default().into()),
        )
    }
}
