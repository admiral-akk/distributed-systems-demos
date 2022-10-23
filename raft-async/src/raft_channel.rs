use std::{collections::HashMap, time::Duration};

use async_std::{
    channel::{SendError, Sender},
    sync::Arc,
    task,
};
use rand::Rng;
use std::fmt::Debug;

use crate::{raft_request::RaftRequest, raft_socket::RaftSocket};

pub struct RaftChannel<DataType>
where
    DataType: Copy + Clone + Debug,
{
    pub id: u32,
    senders: HashMap<u32, Arc<Sender<RaftRequest<DataType>>>>,
}

impl<DataType> RaftChannel<DataType>
where
    DataType: Copy + Clone + Debug + Send + 'static,
{
    pub fn new(id: u32) -> Self {
        Self {
            id,
            senders: HashMap::new(),
        }
    }

    pub fn register_socket(&mut self, other: &mut RaftSocket<DataType>) {
        self.senders
            .insert(other.id, Arc::new(other.sender.clone()));
    }

    pub fn send(&self, target_id: u32, mut message: RaftRequest<DataType>) {
        message.sender = self.id;
        println!("Sending:\n{:?}", message);
        if let Some(sender) = self.senders.get(&target_id) {
            let delay = Duration::from_millis(rand::thread_rng().gen_range(50..1000));
            let sender = sender.clone();
            task::spawn(async move {
                task::sleep(delay).await;
                sender.send(message).await;
            });
        }
    }

    pub fn broadcast(&self, mut message: RaftRequest<DataType>) {
        message.sender = self.id;
        println!("Broadcasting:\n{:?}", message);
        for (_, sender) in &self.senders {
            let delay = Duration::from_millis(rand::thread_rng().gen_range(50..1000));
            let sender = sender.clone();
            let message = message.clone();
            task::spawn(async move {
                task::sleep(delay).await;
                sender.send(message).await;
            });
        }
    }

    pub fn servers(&self) -> Vec<u32> {
        self.senders.keys().map(|id| *id).collect()
    }
}
