use crate::data::request::Request;

use super::raft_state::{RaftStateGeneric, RaftStateWrapper, VolitileState};

pub struct Leader {}

impl<DataType> RaftStateGeneric<DataType, Leader> {
    pub fn handle(
        mut self,
        request: Request<DataType>,
    ) -> (Vec<Request<DataType>>, RaftStateWrapper<DataType>) {
        (Vec::default(), self.into())
    }
}
