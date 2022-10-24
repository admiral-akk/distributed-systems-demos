use crate::data::request::Request;

use super::raft_state::{RaftStateGeneric, RaftStateWrapper};

pub struct Offline {}

// Does nothing. Only exists as a starting point and a transition point.
impl<DataType> RaftStateGeneric<DataType, Offline> {
    pub fn handle(
        mut self,
        request: Request<DataType>,
    ) -> (Vec<Request<DataType>>, RaftStateWrapper<DataType>) {
        (Vec::default(), self.into())
    }
}
