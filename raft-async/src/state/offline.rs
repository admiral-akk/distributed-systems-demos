use std::time::Duration;

use crate::data::{
    data_type::DataType,
    persistent_state::PersistentState,
    request::{Append, AppendResponse, Request, Timeout, Vote, VoteResponse},
    volitile_state::VolitileState,
};

use super::{
    follower::Follower,
    raft_state::{EventHandler, Handler, RaftState, TimeoutHandler},
};

pub struct Offline {}

// Does nothing. Only request it handles is timeout, which it assumes is a reboot request.
impl TimeoutHandler for Offline {
    fn timeout_length(&self) -> Duration {
        Duration::from_millis(4000)
    }
}

impl<T: DataType> Handler<T> for Offline {}

impl<T: DataType> EventHandler<Append<T>, T> for Offline {}

impl<T: DataType> EventHandler<Vote, T> for Offline {}

impl<T: DataType> EventHandler<VoteResponse, T> for Offline {}

impl<T: DataType> EventHandler<Timeout, T> for Offline {
    fn handle_event(
        &mut self,
        volitile_state: &mut VolitileState,
        persistent_state: &mut PersistentState<T>,
        sender: u32,
        term: u32,
        event: Timeout,
    ) -> (Vec<Request<T>>, Option<RaftState>) {
        (Vec::new(), Some(Follower::default().into()))
    }
}

impl<T: DataType> EventHandler<AppendResponse, T> for Offline {}
