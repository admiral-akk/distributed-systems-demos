use std::time::Duration;

use async_std::sync::Arc;

use async_std::task;
use data::request::Request;
use rand::Rng;
use server::{client::Client, raft_cluster::RaftCluster};
use state::state::StateMachine;

mod data;
mod server;
mod state;

#[derive(Default, Debug, Eq, Clone, PartialEq)]
pub struct Sum {
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
    let switch: Arc<RaftCluster<Request<u32, u32>>> = RaftCluster::init::<Sum>(5);
    RaftCluster::add_client::<RandNum>(switch);

    task::block_on(task::spawn(async {
        loop {
            task::sleep(Duration::from_secs(100)).await
        }
    }));
}

#[cfg(test)]
pub mod test_util {
    use crate::Sum;

    pub fn STATE_MACHINE() -> Sum {
        Sum::default()
    }
}
