use std::{collections::HashSet, hash::Hash};

use super::{
    follower::Follower,
    leader::Leader,
    raft_state::{RaftStateGeneric, RaftStateWrapper},
};
use crate::data::request::{Request, RequestType};

#[derive(Default)]
pub struct Candidate {
    votes: HashSet<u32>,
}

impl<DataType> RaftStateGeneric<DataType, Candidate> {
    pub fn handle(
        mut self,
        request: Request<DataType>,
    ) -> (Vec<Request<DataType>>, RaftStateWrapper<DataType>) {
        let (sender, term) = (request.sender, request.term);
        if term < self.persistent_state.current_term {
            self.persistent_state.current_term = term;
            self.persistent_state.voted_for = None;
            return (
                Vec::default(),
                RaftStateGeneric::<DataType, Follower>::from(self).into(),
            );
        }
        match request.data {
            RequestType::VoteResponse { success } => {
                if success {
                    self.state.votes.insert(sender);
                }
                if self.state.votes.len() > self.persistent_state.config.quorum() {
                    return (
                        Vec::default(),
                        RaftStateGeneric::<DataType, Leader>::from(self).into(),
                    );
                }
                (Vec::default(), self.into())
            }
            _ => (Vec::default(), self.into()),
        }
    }
}

impl<DataType> From<RaftStateGeneric<DataType, Follower>>
    for RaftStateGeneric<DataType, Candidate>
{
    fn from(follower: RaftStateGeneric<DataType, Follower>) -> Self {
        RaftStateGeneric::<DataType, Candidate> {
            state: Candidate {
                votes: HashSet::from([follower.persistent_state.id]),
            },
            persistent_state: follower.persistent_state,
            volitile_state: follower.volitile_state,
        }
    }
}
