use crate::data::{
    data_type::DataType,
    persistent_state::PersistentState,
    request::{Request, RequestType},
    volitile_state::VolitileState,
};

use super::{
    candidate::Candidate,
    leader::Leader,
    offline::Offline,
    raft_state::{RaftStateGeneric, RaftStateWrapper},
};

#[derive(Default)]
pub struct Follower {}

impl Default for RaftStateGeneric<Follower> {
    fn default() -> Self {
        Self {
            state: Follower::default(),
            volitile_state: Default::default(),
        }
    }
}

impl RaftStateGeneric<Follower> {
    pub fn vote<T: DataType>(mut self) -> (Vec<Request<T>>, RaftStateWrapper) {
        (Vec::new(), self.into())
    }

    pub fn handle<T: DataType>(
        &mut self,
        request: Request<T>,
        persistent_state: &mut PersistentState<T>,
    ) -> (Vec<Request<T>>, Option<RaftStateWrapper>) {
        let (sender, term) = (request.sender, request.term);
        match request.data {
            RequestType::Vote {
                log_length: prev_log_index,
                last_log_term: prev_log_term,
            } => {
                let mut success = true;
                success &= persistent_state.current_term > term;
                if let Some(voted_for) = persistent_state.voted_for {
                    success &= voted_for != sender;
                }
                if persistent_state.log.len() > 0 {
                    success &= prev_log_index + 1 < persistent_state.log.len();
                    success &= prev_log_index == persistent_state.log.len()
                        && persistent_state.log[prev_log_index].term <= prev_log_term;
                }
                (
                    Vec::from([Request {
                        sender: persistent_state.id,
                        reciever: sender,
                        term: persistent_state.current_term,
                        data: RequestType::VoteResponse { success },
                    }]),
                    None,
                )
            }
            _ => (Vec::default(), None),
        }
    }
}
