use async_std::channel::{self, Receiver, Sender};

use crate::raft_request::RaftRequest;

pub struct RaftSocket {
    pub id: u32,
    pub sender: Sender<RaftRequest>,
    pub reciever: Receiver<RaftRequest>,
}

impl RaftSocket {
    pub fn new(id: u32) -> Self {
        let (sender, reciever) = channel::unbounded();
        Self {
            id,
            sender,
            reciever,
        }
    }
}
