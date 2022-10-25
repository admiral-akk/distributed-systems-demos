use std::time::Duration;

use crate::data::{
    data_type::DataType,
    request::{Append, AppendResponse, Timeout, Vote, VoteResponse},
};

use super::raft_state::{EventHandler, Handler, TimeoutHandler};

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

impl<T: DataType> EventHandler<Timeout, T> for Offline {}

impl<T: DataType> EventHandler<AppendResponse, T> for Offline {}
