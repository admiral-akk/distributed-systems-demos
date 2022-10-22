use std::{collections::HashMap, time::Duration};

use async_std::{
    channel::{self, Receiver, Sender},
    task,
};
use rand::Rng;

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
