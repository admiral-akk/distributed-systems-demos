use std::collections::HashMap;

use async_std::{
    channel::{self, Receiver, Sender},
    sync::{Arc, Mutex},
};

use crate::data::{data_type::DataType, request::Request};

// Responsible for routing requests between servers.
pub struct Switch<T: DataType> {
    pub sender: Sender<Request<T>>,
    pub reciever: Receiver<Request<T>>,
    pub senders: Mutex<HashMap<u32, Sender<Request<T>>>>,
}

impl<T: DataType> Switch<T> {
    pub fn new() -> Self {
        let (sender, reciever) = channel::unbounded();
        Self {
            sender,
            reciever,
            senders: Default::default(),
        }
    }

    pub async fn handle(&self, requests: Vec<Request<T>>) {
        let senders = self.senders.lock().await;
        for request in requests {
            senders[&request.reciever].send(request).await;
        }
    }

    async fn request_loop(switch: Arc<Switch<T>>) {
        loop {
            let request = switch.reciever.recv().await;
            if let Ok(request) = request {
                let senders = switch.senders.lock().await;
                senders[&request.reciever].send(request).await;
            }
        }
    }

    pub async fn register(&self, id: u32) -> (Sender<Request<T>>, Receiver<Request<T>>) {
        let mut senders = self.senders.lock().await;
        let (server_sender, server_reciever) = channel::unbounded();
        senders.insert(id, server_sender);
        (self.sender.clone(), server_reciever)
    }
}
