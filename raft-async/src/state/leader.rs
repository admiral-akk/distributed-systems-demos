use std::{collections::HashMap, time::SystemTime};

use crate::data::{
    data_type::DataType,
    persistent_state::PersistentState,
    request::{Request, RequestType},
    volitile_state::{self, VolitileState},
};

use super::{
    candidate::{self, Candidate},
    follower::Follower,
    raft_state::{Handler, RaftStateGeneric, RaftStateWrapper},
};

pub struct Leader {
    pub next_index: HashMap<u32, usize>,
    pub commit_index: HashMap<u32, usize>,
}

impl Leader {}

impl RaftStateGeneric<Leader> {
    pub fn send_heartbeat<T: DataType>(
        &self,
        volitile_state: &mut VolitileState,
        persistent_state: &mut PersistentState<T>,
    ) -> Vec<Request<T>> {
        Self::heartbeat(persistent_state)
    }

    fn heartbeat<T: DataType>(persistent_state: &mut PersistentState<T>) -> Vec<Request<T>> {
        persistent_state
            .other_servers()
            .iter()
            .map(|id| Request {
                sender: persistent_state.id,
                reciever: *id,
                term: persistent_state.current_term,
                data: RequestType::Append {
                    prev_log_length: 0,
                    prev_log_term: 0,
                    entries: Vec::new(),
                    leader_commit: 0,
                },
            })
            .collect()
    }

    pub fn from_candidate<T: DataType>(
        candidate: &RaftStateGeneric<Candidate>,
        volitile_state: &mut VolitileState,
        persistent_state: &mut PersistentState<T>,
    ) -> (Vec<Request<T>>, Option<RaftStateWrapper>) {
        persistent_state.current_term += 1;
        println!("{} elected leader!", persistent_state.id);
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
        let wrapper = RaftStateGeneric { state: leader };
        (Self::heartbeat(persistent_state), Some(wrapper.into()))
    }
}

impl<T: DataType> Handler<T> for RaftStateGeneric<Leader> {
    fn handle(
        &mut self,
        request: Request<T>,
        volitile_state: &mut VolitileState,
        persistent_state: &mut PersistentState<T>,
    ) -> (Vec<Request<T>>, Option<RaftStateWrapper>) {
        let (sender, term) = (request.sender, request.term);
        if term > persistent_state.current_term {
            persistent_state.current_term = term;
            persistent_state.last_heartbeat = Some(SystemTime::now());
            persistent_state.voted_for = None;
            return (
                Vec::default(),
                Some(
                    RaftStateGeneric::<Follower> {
                        state: Default::default(),
                    }
                    .into(),
                ),
            );
        }
        (Vec::default(), None)
    }
}
