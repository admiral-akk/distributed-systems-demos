use std::collections::HashSet;

use super::{
    follower::Follower,
    leader::Leader,
    raft_state::{RaftStateGeneric, RaftStateWrapper},
};
use crate::data::{
    data_type::DataType,
    persistent_state::PersistentState,
    request::{Request, RequestType},
};

#[derive(Default)]
pub struct Candidate {
    votes: HashSet<u32>,
}

impl RaftStateGeneric<Candidate> {
    pub fn handle<T: DataType>(
        &mut self,
        request: Request<T>,
        persistent_state: &mut PersistentState<T>,
    ) -> (Vec<Request<T>>, Option<RaftStateWrapper>) {
        match request.data {
            RequestType::VoteResponse { success } => {
                if success {
                    self.state.votes.insert(request.sender);
                }
                if self.state.votes.len() > persistent_state.quorum() {
                    return RaftStateGeneric::from_candidate(&self, persistent_state);
                }
                (Vec::default(), None)
            }
            _ => (Vec::default(), None),
        }
    }
}
