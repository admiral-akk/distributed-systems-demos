use std::time::Duration;

use async_std::{
    channel::{Receiver, Sender},
    sync::{Arc, Mutex},
    task,
};
use rand::Rng;

use crate::{
    data::{
        data_type::{CommandType, OutputType},
        persistent_state::{Config, PersistentState},
        request::{Crash, Event, Request, Tick},
    },
    state::state::{State, StateMachine},
};

use super::raft_cluster::{Id, Message, RaftCluster};

pub struct Server<T: CommandType, Output: Send> {
    pub id: u32,
    pub input: Receiver<Request<T, Output>>,
    pub output: Sender<Request<T, Output>>,
    pub server_sender: Sender<Request<T, Output>>,
    pub shutdown: Mutex<bool>,
}

const SERVER_FAILURE: Duration = Duration::from_millis(10000);
const AVERAGE_TICK_LENGTH: Duration = Duration::from_millis(50);

fn tick() -> Duration {
    rand::thread_rng().gen_range((AVERAGE_TICK_LENGTH / 2)..(3 * AVERAGE_TICK_LENGTH / 2))
}

impl<T: CommandType, Output: OutputType> Server<T, Output>
where
    Request<T, Output>: Message,
    PersistentState<T>: Default,
{
    pub async fn init<SM: StateMachine<T, Output>>(
        id: u32,
        switch: Arc<RaftCluster<Request<T, Output>>>,
    ) {
        let (output, server_sender, input) = switch.register(Id::new(id)).await;
        let server = Arc::new(Self {
            input,
            output,
            id,
            server_sender,
            shutdown: Mutex::new(false),
        });
        task::spawn(Server::random_shutdown(server.clone()));
        task::spawn(Server::request_loop::<SM>(
            server.clone(),
            switch.initial_config.clone(),
        ));
        task::spawn(Server::tick_loop(server.clone()));
    }

    async fn random_shutdown(server: Arc<Self>) {
        loop {
            let timeout = rand::thread_rng().gen_range((SERVER_FAILURE / 2)..(2 * SERVER_FAILURE));
            task::sleep(timeout).await;
            if server.shutdown.lock().await.eq(&true) {
                return;
            }
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
            if server.shutdown.lock().await.eq(&true) {
                return;
            }
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

    async fn request_loop<SM: StateMachine<T, Output>>(server: Arc<Self>, initial_config: Config) {
        let mut state: State<T, SM> = State::new(server.id, initial_config);
        let mut responses;
        loop {
            let request = server.input.recv().await;
            {
                let mut shutdown = server.shutdown.lock().await;
                *shutdown |= state.shutdown();
                if *shutdown {
                    return;
                }
            }
            if let Ok(request) = request {
                (responses, state) = state.handle_request(request);
                for response in responses {
                    server.output.send(response).await;
                }
            }
        }
    }
}
