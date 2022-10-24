use crate::data::request::Request;

use super::raft_state::{RaftStateGeneric, RaftStateWrapper};

pub struct Follower {}

impl<DataType> RaftStateGeneric<DataType, Follower> {
    pub fn handle(
        mut self,
        request: Request<DataType>,
    ) -> (Vec<Request<DataType>>, RaftStateWrapper<DataType>) {
        match request {
            Request::Vote {
                sender,
                term,
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
                    Vec::from([Request::VoteResponse {
                        sender: 0,
                        term: self.persistent_state.current_term,
                        success,
                    }]),
                    self.into(),
                )
            }
            _ => (Vec::default(), self.into()),
        }
    }
}
