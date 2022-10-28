use std::collections::HashMap;

use async_std::{
    channel::{self, Receiver, Sender},
    sync::{Arc, Mutex},
    task,
};
use rand::Rng;

use crate::{
    data::{
        data_type::{CommandType, OutputType},
        persistent_state::Config,
        request::Request,
    },
    state::state::StateMachine,
};

use super::server::Server;

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
pub struct RaftCluster<T> {
    pub sender: Sender<T>,
    pub reciever: Receiver<T>,
    pub senders: Mutex<HashMap<Id, Sender<T>>>,
    pub initial_config: Config,
}

impl<In: CommandType, Out: OutputType> RaftCluster<Request<In, Out>> {
    pub fn init<SM: StateMachine<In, Out>>(initial_config: Config) -> Arc<Self> {
        let (sender, reciever) = channel::unbounded();

        let switch = Arc::new(Self {
            sender,
            reciever,
            senders: Default::default(),
            initial_config: initial_config.clone(),
        });
        for server in initial_config.servers {
            Server::init::<SM>(server, switch.clone());
        }
        task::spawn(RaftCluster::request_loop(switch.clone()));
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
    pub async fn register(
        &self,
        id: Id,
    ) -> (
        Sender<Request<In, Out>>,
        Sender<Request<In, Out>>,
        Receiver<Request<In, Out>>,
    ) {
        let (server_sender, server_reciever) = channel::unbounded();
        self.senders.lock().await.insert(id, server_sender.clone());
        (self.sender.clone(), server_sender, server_reciever)
    }
}
