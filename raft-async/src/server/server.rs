use std::time::Duration;

use async_std::{
    channel::{Receiver, Sender},
    sync::{Arc, Mutex},
    task,
};

use crate::{
    data::{
        data_type::DataType,
        request::{self, Request},
    },
    state::{offline::Offline, raft_state::State},
};

use super::switch::Switch;

pub struct Server<T: DataType> {
    pub state: Mutex<State<T>>,
    pub input: Receiver<Request<T>>,
    pub output: Sender<Request<T>>,
}

impl<T: DataType + 'static> Server<T>
where
    State<T>: Default,
{
    pub async fn new(id: u32, switch: Arc<Switch<T>>) -> Self {
        let (output, input) = switch.register(id).await;

        Self {
            input,
            output,
            state: Default::default(),
        }
    }

    pub fn init(server: Arc<Server<T>>) {
        task::spawn(Server::request_loop(server.clone()));
        task::spawn(Server::heartbeat_loop(server.clone()));
    }

    async fn heartbeat_loop(server: Arc<Server<T>>) {
        loop {
            task::sleep(Duration::from_millis(1000)).await;
        }
    }

    async fn request_loop(server: Arc<Server<T>>) {
        loop {
            let request = server.input.recv().await;
            if let Ok(request) = request {
                let responses = {
                    let mut state = server.state.lock().await;
                    state.handle(request)
                };
                for response in responses {
                    server.output.send(response).await;
                }
            }
        }
    }
}
