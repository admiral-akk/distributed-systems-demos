use crate::data::request::{Request, RequestType};

use super::{
    candidate::Candidate,
    leader::Leader,
    offline::Offline,
    raft_state::{RaftStateGeneric, RaftStateWrapper, VolitileState},
};

pub struct Follower {}

impl<DataType> RaftStateGeneric<DataType, Follower> {
    pub fn handle(
        mut self,
        request: Request<DataType>,
    ) -> (Vec<Request<DataType>>, RaftStateWrapper<DataType>) {
        let (sender, term) = (request.sender, request.term);
        if term < self.persistent_state.current_term {
            self.persistent_state.current_term = term;
            self.persistent_state.voted_for = None;
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

impl<DataType> From<RaftStateGeneric<DataType, Candidate>>
    for RaftStateGeneric<DataType, Follower>
{
    fn from(candidate: RaftStateGeneric<DataType, Candidate>) -> Self {
        RaftStateGeneric::<DataType, Follower> {
            state: Follower {},
            persistent_state: candidate.persistent_state,
            volitile_state: candidate.volitile_state,
        }
    }
}

impl<DataType> From<RaftStateGeneric<DataType, Offline>> for RaftStateGeneric<DataType, Follower> {
    fn from(offline: RaftStateGeneric<DataType, Offline>) -> Self {
        RaftStateGeneric::<DataType, Follower> {
            state: Follower {},
            persistent_state: offline.persistent_state,
            volitile_state: VolitileState::default(),
        }
    }
}

impl<DataType> From<RaftStateGeneric<DataType, Leader>> for RaftStateGeneric<DataType, Follower> {
    fn from(leader: RaftStateGeneric<DataType, Leader>) -> Self {
        RaftStateGeneric::<DataType, Follower> {
            state: Follower {},
            persistent_state: leader.persistent_state,
            volitile_state: leader.volitile_state,
        }
    }
}
