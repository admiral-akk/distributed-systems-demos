use super::raft_state::{RaftStateGeneric, RaftStateWrapper};
use crate::data::request::Request;

pub struct Candidate {}

impl<DataType> RaftStateGeneric<DataType, Candidate> {
    pub fn handle(
        mut self,
        request: Request<DataType>,
    ) -> (Vec<Request<DataType>>, RaftStateWrapper<DataType>) {
        (Vec::default(), self.into())
    }
}
