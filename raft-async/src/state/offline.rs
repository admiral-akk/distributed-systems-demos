use crate::data::{data_type::DataType, request::Request};

use super::raft_state::{RaftStateGeneric, RaftStateWrapper};

pub struct Offline {}

// Does nothing. Only exists as a starting point and a transition point.
impl<T: DataType> RaftStateGeneric<T, Offline> {
    pub fn handle(mut self, request: Request<T>) -> (Vec<Request<T>>, RaftStateWrapper<T>) {
        (Vec::default(), self.into())
    }
}
