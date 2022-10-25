use std::collections::HashSet;

use async_std::{
    channel::{Receiver, Sender},
    sync::{Arc, Mutex},
    task,
};
use rand::Rng;

use crate::{
    data::{
        data_type::DataType,
        persistent_state::{Config, PersistentState},
        request::{Event, Request, Timeout},
        volitile_state::VolitileState,
    },
    state::raft_state::{RaftState, State},
};

use super::switch::Switch;

pub struct Server<T: DataType> {
    pub state: Mutex<State<T>>,
    pub input: Receiver<Request<T>>,
    pub output: Sender<Request<T>>,
    pub server_sender: Sender<Request<T>>,
}

impl<T: DataType> Server<T>
where
    PersistentState<T>: Default,
{
    pub async fn new(id: u32, switch: Arc<Switch<T>>) -> Self {
        let (output, server_sender, input) = switch.register(id).await;
        Self {
            input,
            output,
            server_sender,
            state: Mutex::new(State {
                persistent_state: PersistentState {
                    id,
                    config: Config {
                        servers: HashSet::from([0, 1, 2, 3, 4]),
                    },
                    ..Default::default()
                },
                raft_state: RaftState::default(),
                volitile_state: VolitileState::default(),
            }),
        }
    }

    pub fn init(server: Arc<Server<T>>) {
        task::spawn(Server::request_loop(server.clone()));
        task::spawn(Server::timeout_loop(server.clone()));
    }

    async fn timeout_loop(server: Arc<Server<T>>) {
        loop {
            let (timeout, keep_alive) = {
                let state = server.state.lock().await;
                let timeout = state.timeout_length();
                (
                    rand::thread_rng().gen_range(timeout..(2 * timeout)),
                    state.persistent_state.keep_alive,
                )
            };
            task::sleep(timeout).await;
            // Check if keep alive has been incremented. If not, then we've timed out.

            let timed_out = {
                let state = server.state.lock().await;
                state.persistent_state.keep_alive == keep_alive
            };
            if timed_out {
                server
                    .server_sender
                    .send(Request {
                        event: Event::Timeout(Timeout),
                        sender: 0,
                        reciever: 0,
                        term: 0,
                    })
                    .await;
            }
        }
    }

    async fn request_loop(server: Arc<Server<T>>) {
        loop {
            let request = server.input.recv().await;
            if let Ok(request) = request {
                let responses = {
                    let mut state = server.state.lock().await;
                    state.handle_request(request)
                };

                for response in responses {
                    server.output.send(response).await;
                }
            }
        }
    }
}