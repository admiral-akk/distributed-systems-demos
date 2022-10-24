use super::{
    follower::Follower,
    raft_state::{Handler, RaftStateGeneric, RaftStateWrapper},
};
use crate::data::{
    data_type::DataType,
    persistent_state::PersistentState,
    request::{Request, RequestType},
};

pub struct Offline {}

// Does nothing. Only request it handles is booting up.
impl<T: DataType> Handler<T> for RaftStateGeneric<Offline> {
    fn handle(
        &mut self,
        request: Request<T>,
        _: &mut PersistentState<T>,
    ) -> (Vec<Request<T>>, Option<RaftStateWrapper>) {
        match request.data {
            RequestType::Bootup {} => (
                Vec::default(),
                Some(RaftStateGeneric::<Follower>::default().into()),
            ),
            _ => (Vec::default(), None),
        }
    }
}
