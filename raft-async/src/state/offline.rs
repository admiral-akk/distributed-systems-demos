use super::raft_state::Handler;
use crate::data::data_type::DataType;

pub struct Offline {}

// Does nothing. Only request it handles is timeout, which it assumes is a reboot request.
impl<T: DataType> Handler<T> for Offline {}
