use std::collections::HashMap;

use async_std::{
    channel::{self, Receiver, Sender},
    sync::{Arc, Mutex},
    task,
};
use rand::Rng;

#[derive(Hash, PartialEq, Eq)]
pub struct Id(u32); // todo: change id into an enum so you can seperate client/server

impl Id {
    pub fn new(id: u32) -> Id {
        Id(id)
    }
}

pub trait Message: Send + 'static {
    fn recipient(&self) -> Id;
}

// Responsible for routing requests between servers.
pub struct Switch<T> {
    pub sender: Sender<T>,
    pub reciever: Receiver<T>,
    pub senders: Mutex<HashMap<Id, Sender<T>>>,
}

impl<T: Message> Switch<T> {
    pub fn init() -> Arc<Self> {
        let (sender, reciever) = channel::unbounded();

        let switch = Arc::new(Self {
            sender,
            reciever,
            senders: Default::default(),
        });
        task::spawn(Switch::request_loop(switch.clone()));
        switch
    }

    async fn request_loop(switch: Arc<Self>) {
        loop {
            let request = switch.reciever.recv().await;
            if let Ok(request) = request {
                if rand::thread_rng().gen_range(0..100) > 80 {
                    continue;
                }
                let socket = {
                    let senders = switch.senders.lock().await;
                    senders[&request.recipient()].clone()
                };
                socket.send(request).await;
            }
        }
    }
    pub async fn register(&self, id: Id) -> (Sender<T>, Sender<T>, Receiver<T>) {
        let (server_sender, server_reciever) = channel::unbounded();
        self.senders.lock().await.insert(id, server_sender.clone());
        (self.sender.clone(), server_sender, server_reciever)
    }
}
