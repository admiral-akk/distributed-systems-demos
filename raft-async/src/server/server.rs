use std::{collections::HashSet, time::Duration};

use async_std::{
    channel::{Receiver, Sender},
    sync::{Arc, Mutex},
    task,
};
use rand::Rng;

use crate::{
    data::{
        data_type::CommandType,
        persistent_state::{Config, PersistentState},
        request::{Crash, Event, Request, Tick},
    },
    state::state::State,
};

use super::switch::{Id, Message, Switch};

pub struct Server<T: CommandType> {
    pub id: u32,
    pub input: Receiver<Request<T>>,
    pub output: Sender<Request<T>>,
    pub server_sender: Sender<Request<T>>,
}

const SERVER_FAILURE: Duration = Duration::from_millis(10000);

const AVERAGE_TICK_LENGTH: Duration = Duration::from_millis(50);
fn tick() -> Duration {
    rand::thread_rng().gen_range((AVERAGE_TICK_LENGTH / 2)..(3 * AVERAGE_TICK_LENGTH / 2))
}

impl<T: CommandType> Server<T>
where
    Request<T>: Message,
    PersistentState<T>: Default,
{
    pub async fn new(id: u32, switch: Arc<Switch<Request<T>>>) -> Self {
        let (output, server_sender, input) = switch.register(Id::new(id)).await;
        Self {
            input,
            output,
            id,
            server_sender,
        }
    }

    pub fn init(server: Arc<Self>) {
        task::spawn(Server::random_shutdown(server.clone()));
        task::spawn(Server::request_loop(server.clone()));
        task::spawn(Server::tick_loop(server.clone()));
    }

    async fn random_shutdown(server: Arc<Self>) {
        loop {
            let timeout = rand::thread_rng().gen_range((SERVER_FAILURE / 2)..(2 * SERVER_FAILURE));
            task::sleep(timeout).await;
            server
                .server_sender
                .send(Request {
                    event: Event::Crash(Crash),
                    sender: 0,
                    reciever: 0,
                    term: 0,
                })
                .await;
        }
    }

    async fn tick_loop(server: Arc<Self>) {
        loop {
            task::sleep(tick()).await;
            // Check if keep alive has been incremented. If not, then we've timed out.
            server
                .server_sender
                .send(Request {
                    event: Event::Tick(Tick),
                    sender: 0,
                    reciever: 0,
                    term: 0,
                })
                .await;
        }
    }

    async fn request_loop(server: Arc<Self>) {
        let mut state: State<T> = State::new(
            server.id,
            Config {
                servers: HashSet::from([0, 1, 2, 3, 4]),
            },
        );
        let mut responses;
        loop {
            let request = server.input.recv().await;
            if let Ok(request) = request {
                (state, responses) = state.handle_request(request);
                for response in responses {
                    server.output.send(response).await;
                }
            }
        }
    }
}
