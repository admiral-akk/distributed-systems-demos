use std::time::Duration;

use async_std::sync::Arc;

use async_std::task;
use data::{persistent_state::Config, request::Request};
use rand::Rng;
use server::{client::Client, cluster::RaftCluster, server::Server};
use state::state::StateMachine;

mod data;
mod server;
mod state;

#[derive(Default, Debug, Clone)]
struct Sum {
    total: u32,
}

impl StateMachine<u32, u32> for Sum {
    fn apply(&mut self, command: u32) {
        self.total += command;
    }

    fn get(&self) -> u32 {
        self.total
    }
}

pub trait DataGenerator<T>: Default + Send + 'static {
    fn gen(&self) -> T;
}

#[derive(Default)]
pub struct RandNum {}
impl DataGenerator<u32> for RandNum {
    fn gen(&self) -> u32 {
        rand::thread_rng().gen_range(1..5)
    }
}

fn main() {
    let initial_config = Config {
        servers: [0, 1, 2, 3, 4].into(),
    };
    let switch: Arc<RaftCluster<Request<u32, u32>>> = RaftCluster::init::<Sum>(initial_config);
    let clients = (10..12)
        .map(|id| Client::<u32, u32>::new(id, switch.clone()))
        .map(|client| Arc::new(task::block_on(client)))
        .collect::<Vec<_>>();

    for client in clients {
        Client::init::<RandNum>(client);
    }

    task::block_on(task::spawn(async {
        loop {
            task::sleep(Duration::from_secs(100)).await
        }
    }));
}
