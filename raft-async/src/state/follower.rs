use crate::data::{
    data_type::DataType,
    request::{Request, RequestType},
};

use super::{
    candidate::Candidate,
    leader::Leader,
    offline::Offline,
    raft_state::{RaftStateGeneric, RaftStateWrapper, VolitileState},
};

pub struct Follower {}

impl<T: DataType> RaftStateGeneric<T, Follower> {
    pub fn handle(mut self, request: Request<T>) -> (Vec<Request<T>>, RaftStateWrapper<T>) {
        let (sender, term) = (request.sender, request.term);
        if term < self.persistent_state.current_term {
            self.persistent_state.current_term = term;
            self.persistent_state.voted_for = None;
            let follower = RaftStateGeneric::<T, Follower>::from(self);
            return follower.handle(request);
        }
        match request.data {
            RequestType::Vote {
                prev_log_index,
                prev_log_term,
            } => {
                let mut success = true;
                success &= self.persistent_state.current_term > term;
                if let Some(voted_for) = self.persistent_state.voted_for {
                    success &= voted_for != sender;
                }
                if self.persistent_state.log.len() > 0 {
                    success &= prev_log_index + 1 < self.persistent_state.log.len();
                    success &= prev_log_index == self.persistent_state.log.len()
                        && self.persistent_state.log[prev_log_index].term <= prev_log_term;
                }
                (
                    Vec::from([Request {
                        sender: self.persistent_state.id,
                        term: self.persistent_state.current_term,
                        data: RequestType::VoteResponse { success },
                    }]),
                    self.into(),
                )
            }
            _ => (Vec::default(), self.into()),
        }
    }
}

impl<T: DataType> From<RaftStateGeneric<T, Candidate>> for RaftStateGeneric<T, Follower> {
    fn from(candidate: RaftStateGeneric<T, Candidate>) -> Self {
        RaftStateGeneric::<T, Follower> {
            state: Follower {},
            persistent_state: candidate.persistent_state,
            volitile_state: candidate.volitile_state,
        }
    }
}

impl<T: DataType> From<RaftStateGeneric<T, Offline>> for RaftStateGeneric<T, Follower> {
    fn from(offline: RaftStateGeneric<T, Offline>) -> Self {
        RaftStateGeneric::<T, Follower> {
            state: Follower {},
            persistent_state: offline.persistent_state,
            volitile_state: VolitileState::default(),
        }
    }
}

impl<T: DataType> From<RaftStateGeneric<T, Leader>> for RaftStateGeneric<T, Follower> {
    fn from(leader: RaftStateGeneric<T, Leader>) -> Self {
        RaftStateGeneric::<T, Follower> {
            state: Follower {},
            persistent_state: leader.persistent_state,
            volitile_state: leader.volitile_state,
        }
    }
}
