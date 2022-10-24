use super::raft_state::{Handler, RaftStateGeneric, RaftStateWrapper};
use crate::data::{data_type::DataType, persistent_state::PersistentState, request::Request};

pub struct Offline {}

// Does nothing. Only exists as a starting point and a transition point.
impl<T: DataType> Handler<T> for RaftStateGeneric<Offline> {
    fn handle(
        &mut self,
        request: Request<T>,
        persistent_state: &mut PersistentState<T>,
    ) -> (Vec<Request<T>>, Option<RaftStateWrapper>) {
        (Vec::default(), None)
    }
}
