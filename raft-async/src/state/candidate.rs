use std::{collections::HashSet, time::SystemTime};

use super::{
    follower::Follower,
    leader::Leader,
    raft_state::{Handler, RaftState},
};
use crate::data::{
    data_type::DataType,
    persistent_state::PersistentState,
    request::{Request, RequestType},
    volitile_state::VolitileState,
};

#[derive(Default)]
pub struct Candidate {
    votes: HashSet<u32>,
}
const CANDIDATE_TIMEOUT: u128 = 5000;

impl<T: DataType> Handler<T> for Candidate {
    fn handle(
        &mut self,
        volitile_state: &mut VolitileState,
        persistent_state: &mut PersistentState<T>,
        request: Request<T>,
    ) -> (Vec<Request<T>>, Option<RaftState>) {
        let (sender, term) = (request.sender, request.term);
        if term > persistent_state.current_term {
            persistent_state.current_term = term;
            persistent_state.voted_for = None;
            return (Vec::default(), Some(Follower::default().into()));
        }
        match request.data {
            RequestType::Append { .. } => {
                if request.term >= persistent_state.current_term {
                    return (Vec::default(), Some(Follower::default().into()));
                }
                (Vec::default(), None)
            }
            RequestType::VoteResponse { success } => {
                if success {
                    println!("{} voted for {}", request.sender, persistent_state.id);
                    self.votes.insert(request.sender);
                }
                if self.votes.len() > persistent_state.quorum() {
                    return Leader::from_candidate(&self, volitile_state, persistent_state);
                }
                (Vec::default(), None)
            }
            _ => (Vec::default(), None),
        }
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
        (
            persistent_state.request_votes(),
            Some(Candidate::default().into()),
        )
    }
}
