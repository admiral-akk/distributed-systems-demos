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

pub struct Follower {}

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

impl From<RaftStateGeneric<Candidate>> for RaftStateGeneric<Follower> {
    fn from(candidate: RaftStateGeneric<Candidate>) -> Self {
        RaftStateGeneric::<Follower> {
            state: Follower {},
            volitile_state: candidate.volitile_state,
        }
    }
}

impl From<RaftStateGeneric<Offline>> for RaftStateGeneric<Follower> {
    fn from(offline: RaftStateGeneric<Offline>) -> Self {
        RaftStateGeneric::<Follower> {
            state: Follower {},
            volitile_state: VolitileState::default(),
        }
    }
}

impl From<RaftStateGeneric<Leader>> for RaftStateGeneric<Follower> {
    fn from(leader: RaftStateGeneric<Leader>) -> Self {
        RaftStateGeneric::<Follower> {
            state: Follower {},
            volitile_state: leader.volitile_state,
        }
    }
}
