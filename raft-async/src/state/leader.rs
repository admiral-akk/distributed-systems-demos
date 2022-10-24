use std::collections::HashMap;

use crate::data::{data_type::DataType, persistent_state::PersistentState, request::Request};

use super::{
    candidate::{self, Candidate},
    raft_state::{Handler, RaftStateGeneric, RaftStateWrapper},
};

pub struct Leader {
    pub next_index: HashMap<u32, usize>,
    pub commit_index: HashMap<u32, usize>,
}

impl Leader {}

impl RaftStateGeneric<Leader> {
    pub fn from_candidate<T: DataType>(
        candidate: &RaftStateGeneric<Candidate>,
        persistent_state: &PersistentState<T>,
    ) -> (Vec<Request<T>>, Option<RaftStateWrapper>) {
        let leader = Leader {
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
        };
        let wrapper = RaftStateGeneric {
            state: leader,
            volitile_state: candidate.volitile_state,
        };
        (Vec::new(), Some(wrapper.into()))
    }
}

impl<T: DataType> Handler<T> for RaftStateGeneric<Leader> {
    fn handle(
        &mut self,
        request: Request<T>,
        persistent_state: &mut PersistentState<T>,
    ) -> (Vec<Request<T>>, Option<RaftStateWrapper>) {
        (Vec::default(), None)
    }
}
