use std::{collections::HashSet, time::Duration};

use crate::{
    data::{
        data_type::{CommandType, OutputType},
        request::{self, ClientData, ClientResponse, Event, Request, TransactionId},
    },
    DataGenerator,
};
use async_std::{
    channel::{Receiver, Sender},
    sync::{Arc, Mutex},
    task,
};
use rand::Rng;

use super::raft_cluster::{Id, Message, RaftCluster};

pub struct Client<T: CommandType, Output>
where
    Request<T, Output>: Message,
{
    pub id: Id,
    pub leader_id: Mutex<Id>,
    pub transactions: Mutex<HashSet<TransactionId>>,
    pub input: Receiver<Request<T, Output>>,
    pub output: Sender<Request<T, Output>>,
    pub server_sender: Sender<Request<T, Output>>,
}

const WAIT: Duration = Duration::from_millis(500);

impl<T: CommandType, Output: OutputType> Client<T, Output>
where
    Request<T, Output>: Message,
{
    pub async fn new(id: Id, switch: Arc<RaftCluster<Request<T, Output>>>) -> Self {
        let (output, server_sender, input) = switch.register(id).await;
        Self {
            id,
            transactions: Default::default(),
            leader_id: Mutex::new(Id::Server(0)),
            input,
            output,
            server_sender,
        }
    }

    pub fn init<Gen: DataGenerator<T>>(server: Arc<Client<T, Output>>) {
        task::spawn(Client::request_loop(server.clone(), Gen::default()));
        task::spawn(Client::response_loop(server.clone()));
    }

    pub async fn transaction_id(&self) -> TransactionId {
        let mut id = TransactionId(self.id, rand::thread_rng().gen());
        let mut used = self.transactions.lock().await;
        while used.contains(&id) {
            id = TransactionId(self.id, rand::thread_rng().gen());
        }
        used.insert(id.clone());
        id
    }

    async fn request_loop<Gen: DataGenerator<T>>(client: Arc<Client<T, Output>>, data_gen: Gen) {
        loop {
            let wait_duration = rand::thread_rng().gen_range((WAIT / 2)..(2 * WAIT));
            task::sleep(wait_duration).await;
            let request = Request {
                sender: client.id,
                reciever: *client.leader_id.lock().await,
                term: 0,
                event: Event::Client(request::Client {
                    id: client.transaction_id().await,
                    data: ClientData::Command(data_gen.gen()),
                }),
            };
            client.output.send(request).await;
        }
    }

    async fn response_loop(client: Arc<Client<T, Output>>) {
        loop {
            let request = client.input.recv().await;
            if let Ok(request) = request {
                match request.event {
                    Event::ClientResponse(r) => match r {
                        ClientResponse::Failed {
                            id,
                            leader_id,
                            data,
                        } => {
                            if let Some(leader_id) = leader_id {
                                {
                                    *client.leader_id.lock().await = leader_id;
                                }
                                println!("Transaction id failed: {:?}", id);
                                let response = Request {
                                    sender: client.id,
                                    reciever: leader_id,
                                    term: 0,
                                    event: Event::Client(request::Client {
                                        id: client.transaction_id().await,
                                        data,
                                    }),
                                };
                                client.output.send(response).await;
                            }
                        }
                        _ => {}
                    },
                    _ => {}
                }
            }
        }
    }
}
