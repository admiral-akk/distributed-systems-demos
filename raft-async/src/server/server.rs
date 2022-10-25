use std::{collections::HashSet, time::Duration};

use async_std::{
    channel::{self, Receiver, Sender},
    future,
    sync::{Arc, Mutex},
    task,
};
use rand::Rng;

use crate::{
    data::{
        data_type::DataType,
        persistent_state::{Config, PersistentState},
        request::Request,
        volitile_state::VolitileState,
    },
    state::raft_state::{RaftState, State},
};

use super::switch::Switch;

pub struct Server<T: DataType> {
    pub state: Mutex<State<T>>,
    pub input: Receiver<Request<T>>,
    pub output: Sender<Request<T>>,
}

const TIMEOUT_MILLIS_CHECK: u64 = 1000;

impl<T: DataType> Server<T>
where
    PersistentState<T>: Default,
{
    pub async fn new(id: u32, switch: Arc<Switch<T>>) -> Self {
        let (output, input) = switch.register(id).await;
        Self {
            input,
            output,
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
                (state.timeout(), state.persistent_state.keep_alive)
            };
            task::sleep(timeout).await;
            let responses = {
                // Check if keep alive has been incremented. If not, then we've waited too long.
                let mut state = server.state.lock().await;
                match state.persistent_state.keep_alive == keep_alive {
                    true => state.trigger_timeout(),
                    false => Vec::new(),
                }
            };
            for response in responses {
                server.output.send(response).await;
            }
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
