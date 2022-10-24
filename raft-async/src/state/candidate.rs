use std::{collections::HashSet, hash::Hash};

use super::{
    follower::Follower,
    leader::Leader,
    raft_state::{RaftStateGeneric, RaftStateWrapper},
};
use crate::data::{
    data_type::DataType,
    request::{Request, RequestType},
};

#[derive(Default)]
pub struct Candidate {
    votes: HashSet<u32>,
}

impl<T: DataType> RaftStateGeneric<T, Candidate> {
    pub fn handle(mut self, request: Request<T>) -> (Vec<Request<T>>, RaftStateWrapper<T>) {
        let (sender, term) = (request.sender, request.term);
        if term < self.persistent_state.current_term {
            self.persistent_state.current_term = term;
            self.persistent_state.voted_for = None;
            let follower = RaftStateGeneric::<T, Follower>::from(self);
            return follower.handle(request);
        }
        match request.data {
            RequestType::VoteResponse { success } => {
                if success {
                    self.state.votes.insert(sender);
                }
                if self.state.votes.len() > self.persistent_state.quorum() {
                    return RaftStateGeneric::<T, Leader>::from(self).heartbeat();
                }
                (Vec::default(), self.into())
            }
            _ => (Vec::default(), self.into()),
        }
    }
}

impl<T: DataType> From<RaftStateGeneric<T, Follower>> for RaftStateGeneric<T, Candidate> {
    fn from(follower: RaftStateGeneric<T, Follower>) -> Self {
        RaftStateGeneric::<T, Candidate> {
            state: Candidate {
                votes: HashSet::from([follower.persistent_state.id]),
            },
            persistent_state: follower.persistent_state,
            volitile_state: follower.volitile_state,
        }
    }
}
