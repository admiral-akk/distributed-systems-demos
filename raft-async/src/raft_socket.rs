use crate::raft_request::RaftRequest;
use async_std::channel::{self, Receiver, Sender};
use std::fmt::Debug;

pub struct RaftSocket<DataType>
where
    DataType: Copy + Clone + Debug,
{
    pub id: u32,
    pub sender: Sender<RaftRequest<DataType>>,
    pub reciever: Receiver<RaftRequest<DataType>>,
}

impl<DataType> RaftSocket<DataType>
where
    DataType: Copy + Clone + Debug,
{
    pub fn new(id: u32) -> Self {
        let (sender, reciever) = channel::unbounded();
        Self {
            id,
            sender,
            reciever,
        }
    }
}
