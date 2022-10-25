use std::time::Duration;

use crate::data::{
    data_type::DataType,
    persistent_state::PersistentState,
    request::{self, ClientResponse, Event, Request},
};
use async_std::{
    channel::{Receiver, Sender},
    sync::{Arc, Mutex},
    task,
};
use rand::Rng;

use super::switch::Switch;

pub struct Client<T: DataType> {
    pub id: u32,
    pub leader_id: Mutex<u32>,
    pub input: Receiver<Request<T>>,
    pub output: Sender<Request<T>>,
    pub server_sender: Sender<Request<T>>,
}

const WAIT: Duration = Duration::from_millis(500);

impl<T: DataType> Client<T>
where
    PersistentState<T>: Default,
{
    pub async fn new(id: u32, switch: Arc<Switch<T>>) -> Self {
        let (output, server_sender, input) = switch.register(id).await;
        Self {
            id,
            leader_id: Mutex::new(0),
            input,
            output,
            server_sender,
        }
    }

    pub fn init(server: Arc<Client<T>>) {
        task::spawn(Client::request_loop(server.clone()));
        task::spawn(Client::response_loop(server.clone()));
    }

    async fn request_loop(client: Arc<Client<T>>) {
        loop {
            let wait_duration = rand::thread_rng().gen_range((WAIT / 2)..(2 * WAIT));
            task::sleep(wait_duration).await;
            let request = Request {
                sender: client.id,
                reciever: *client.leader_id.lock().await,
                term: 0,
                event: Event::Client(request::Client { data: T::default() }),
            };
            client.output.send(request).await;
        }
    }

    async fn response_loop(client: Arc<Client<T>>) {
        loop {
            let request = client.input.recv().await;
            if let Ok(request) = request {
                match request.event {
                    Event::ClientResponse(r) => match r {
                        ClientResponse::Failed { leader_id, data } => {
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
                        _ => {}
                    },
                    _ => {}
                }
            }
        }
    }
}
