use std::collections::HashMap;

use crate::data::{
    data_type::DataType,
    request::{Request, RequestType},
};

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
    pub fn new<T: DataType>(persistent_state: &PersistentState<T>) -> Self {
        Leader {
            next_index: persistent_state
                .other_servers()
                .iter()
                .map(|id| (*id, persistent_state.log.len()))
                .collect(),
            commit_index: persistent_state
                .other_servers()
                .iter()
                .map(|id| (*id, 0))
                .collect(),
        }
    }
}

impl<T: DataType> RaftStateGeneric<T, Leader> {
    fn heartbeat_request(&self, server: u32) -> Request<T> {
        let index = self.state.next_index[&server];
        self.persistent_state
            .append(&self.volitile_state, index, server)
    }

    pub fn heartbeat(mut self) -> (Vec<Request<T>>, RaftStateWrapper<T>) {
        (
            self.persistent_state
                .other_servers()
                .iter()
                .map(|id| self.heartbeat_request(*id))
                .collect(),
            self.into(),
        )
    }

    pub fn handle(mut self, request: Request<T>) -> (Vec<Request<T>>, RaftStateWrapper<T>) {
        let (sender, term) = (request.sender, request.term);
        if term < self.persistent_state.current_term {
            self.persistent_state.current_term = term;
            self.persistent_state.voted_for = None;
            let follower = RaftStateGeneric::<T, Follower>::from(self);
            return follower.handle(request);
        }
        (Vec::default(), self.into())
    }
}

impl<T: DataType> From<RaftStateGeneric<T, Candidate>> for RaftStateGeneric<T, Leader> {
    fn from(leader: RaftStateGeneric<T, Candidate>) -> Self {
        RaftStateGeneric::<T, Leader> {
            state: Leader::new(&leader.persistent_state),
            persistent_state: leader.persistent_state,
            volitile_state: leader.volitile_state,
        }
    }
}
