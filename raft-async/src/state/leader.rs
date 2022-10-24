use std::collections::{HashMap, HashSet};

use crate::data::request::Request;

use super::{
    candidate::Candidate,
    follower::Follower,
    raft_state::{PersistentState, RaftStateGeneric, RaftStateWrapper},
};

pub struct Leader {
    pub next_index: HashMap<u32, usize>,
    pub commit_index: HashMap<u32, usize>,
}

impl Leader {
    pub fn new<DataType>(
        persistent_state: &PersistentState<DataType>,
        servers: &HashSet<u32>,
    ) -> Self {
        let other_servers = servers
            .iter()
            .filter(|id| !persistent_state.id.eq(id))
            .map(|id| *id)
            .collect::<Vec<_>>();

        Leader {
            next_index: other_servers
                .iter()
                .map(|id| (*id, persistent_state.log.len()))
                .collect(),
            commit_index: other_servers.iter().map(|id| (*id, 0)).collect(),
        }
    }
}

impl Default for Leader {
    fn default() -> Self {
        Self {
            next_index: Default::default(),
            commit_index: Default::default(),
        }
    }
}

impl<DataType> RaftStateGeneric<DataType, Leader> {
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
        (Vec::default(), self.into())
    }
}

impl<DataType> From<RaftStateGeneric<DataType, Candidate>> for RaftStateGeneric<DataType, Leader> {
    fn from(leader: RaftStateGeneric<DataType, Candidate>) -> Self {
        RaftStateGeneric::<DataType, Leader> {
            state: Leader::default(),
            persistent_state: leader.persistent_state,
            volitile_state: leader.volitile_state,
        }
    }
}
