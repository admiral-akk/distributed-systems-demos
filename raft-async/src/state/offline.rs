use std::time::SystemTime;

use super::{
    follower::Follower,
    raft_state::{Handler, RaftStateGeneric, RaftStateWrapper},
};
use crate::data::{data_type::DataType, persistent_state::PersistentState, request::Request};

pub struct Offline {}

// Does nothing. Only request it handles is timeout, which it assumes is a reboot request.
impl<T: DataType> Handler<T> for RaftStateGeneric<Offline> {
    fn check_timeout(
        &mut self,
        persistent_state: &mut PersistentState<T>,
    ) -> (Vec<Request<T>>, Option<RaftStateWrapper>) {
        println!("{} waking up!", persistent_state.id);
        persistent_state.last_heartbeat = Some(SystemTime::now());
        (
            Vec::default(),
            Some(RaftStateGeneric::<Follower>::default().into()),
        )
    }
}
