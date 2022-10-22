use std::{collections::HashMap, time::Duration};

use async_std::{
    channel::{SendError, Sender},
    task,
};
use rand::Rng;

use crate::{raft_request::RaftRequest, raft_socket::RaftSocket};

pub struct RaftChannel {
    pub id: u32,
    senders: HashMap<u32, Sender<RaftRequest>>,
}

impl RaftChannel {
    pub fn new(id: u32) -> Self {
        Self {
            id,
            senders: HashMap::new(),
        }
    }

    pub fn register_socket(&mut self, other: &mut RaftSocket) {
        self.senders.insert(other.id, other.sender.clone());
    }

    pub async fn send(
        &self,
        target_id: u32,
        mut message: RaftRequest,
    ) -> Result<(), SendError<RaftRequest>> {
        message.sender = self.id;
        self.random_delay().await;
        if let Some(sender) = self.senders.get(&target_id) {
            println!("Sending:\n{:?}", message);
            sender.send(message).await?;
        }
        Ok(())
    }
    async fn random_delay(&self) {
        let rand_len = rand::thread_rng().gen_range(50..100);
        task::sleep(Duration::from_millis(rand_len)).await;
    }

    pub async fn broadcast(&self, mut message: RaftRequest) -> Result<(), SendError<RaftRequest>> {
        message.sender = self.id;
        self.random_delay().await;
        println!("Broadcasting:\n{:?}", message);
        for (_, sender) in &self.senders {
            sender.send(message).await?;
        }
        Ok(())
    }

    pub fn server_count(&self) -> usize {
        self.senders.len() + 1
    }
}
