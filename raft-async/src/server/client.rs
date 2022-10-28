use std::time::Duration;

use crate::{
    data::{
        data_type::{CommandType, OutputType},
        request::{self, ClientResponse, Event, Request},
    },
    DataGenerator,
};
use async_std::{
    channel::{Receiver, Sender},
    sync::{Arc, Mutex},
    task,
};
use rand::Rng;

use super::cluster::{Id, Message, RaftCluster};

pub struct Client<T: CommandType, Output>
where
    Request<T, Output>: Message,
{
    pub id: u32,
    pub leader_id: Mutex<u32>,
    pub input: Receiver<Request<T, Output>>,
    pub output: Sender<Request<T, Output>>,
    pub server_sender: Sender<Request<T, Output>>,
}

const WAIT: Duration = Duration::from_millis(500);

impl<T: CommandType, Output: OutputType> Client<T, Output>
where
    Request<T, Output>: Message,
{
    pub async fn new(id: u32, switch: Arc<RaftCluster<Request<T, Output>>>) -> Self {
        let (output, server_sender, input) = switch.register(Id::new(id)).await;
        Self {
            id,
            leader_id: Mutex::new(0),
            input,
            output,
            server_sender,
        }
    }

    pub fn init<Gen: DataGenerator<T>>(server: Arc<Client<T, Output>>) {
        task::spawn(Client::request_loop(server.clone(), Gen::default()));
        task::spawn(Client::response_loop(server.clone()));
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
                    data: request::Data::Command(data_gen.gen()),
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
                        ClientResponse::Failed { leader_id, data } => {
                            if let Some(leader_id) = leader_id {
                                {
                                    *client.leader_id.lock().await = leader_id;
                                }
                                let response = Request {
                                    sender: client.id,
                                    reciever: leader_id,
                                    term: 0,
                                    event: Event::Client(request::Client { data }),
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
