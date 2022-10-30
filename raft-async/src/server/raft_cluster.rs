use std::{collections::HashMap, time::Duration};

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
    DataGenerator,
};

use super::{client::Client, server::Server};

// todo: change id into an enum so you can seperate client/server
#[derive(Clone, Copy, Default, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Id {
    id: u32,
}

impl Id {
    pub const NONE: Id = Id::new(10000);
    pub const fn new(id: u32) -> Id {
        Id { id }
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
    pub curr_config: Mutex<Config>,
}

impl<In: CommandType, Out: OutputType> RaftCluster<Request<In, Out>> {
    pub fn init<SM: StateMachine<In, Out>>(initial_size: u32) -> Arc<Self> {
        let initial_config = Config {
            servers: (0..initial_size).map(|id| Id::new(id)).collect(),
        };
        let (sender, reciever) = channel::unbounded();

        let switch = Arc::new(Self {
            sender,
            reciever,
            senders: Default::default(),
            curr_config: Mutex::new(initial_config.clone()),
            initial_config: initial_config.clone(),
        });
        for server in initial_config.servers {
            task::spawn(Server::init::<SM>(server, switch.clone()));
        }
        task::spawn(RaftCluster::request_loop(switch.clone()));
        switch
    }

    pub fn add_client<DataGen: DataGenerator<In>>(switch: Arc<Self>) {
        let clients = (10..12)
            .map(|id| Client::<In, Out>::new(Id::new(id), switch.clone()))
            .map(|client| Arc::new(task::block_on(client)))
            .collect::<Vec<_>>();

        for client in clients {
            Client::init::<DataGen>(client);
        }
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
                    if senders.contains_key(&request.recipient()) {
                        Some(senders[&request.recipient()].clone())
                    } else {
                        None
                    }
                };
                match socket {
                    Some(socket) => {
                        socket.send(request).await;
                    }
                    _ => {}
                }
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

#[cfg(test)]
pub mod test_util {
    use super::Id;

    pub const CLIENT_0: Id = Id::new(10);
    pub const SERVER_0: Id = Id::new(0);
    pub const SERVER_1: Id = Id::new(1);
    pub const SERVER_2: Id = Id::new(2);
    pub const SERVER_3: Id = Id::new(3);
    pub const SERVER_4: Id = Id::new(4);
    pub const SERVER_5: Id = Id::new(5);
    pub const SERVER_6: Id = Id::new(6);

    pub const NONE: Id = Id::new(10000);
}
