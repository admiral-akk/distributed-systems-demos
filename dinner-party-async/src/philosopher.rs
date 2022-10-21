use std::time::Duration;

use async_std::sync::Arc;
use async_std::task::sleep;

use futures::lock::Mutex;
use rand::Rng;

use crate::fork::Fork;

pub struct Philosopher {
    name: String,
}

impl ToString for Philosopher {
    fn to_string(&self) -> String {
        format!("{}", self.name)
    }
}

impl Philosopher {
    pub fn new(name: String) -> Self {
        Self { name }
    }

    pub async fn eat(&self, left: Arc<Mutex<Fork>>, right: Arc<Mutex<Fork>>) {
        let mut attempts = 0;
        loop {
            println!("{} is trying to get left!", self.name);
            if let Some(_left_fork) = left.try_lock() {
                println!("{} has left!", self.name);
                sleep(Duration::from_millis(100)).await;
                println!("{} is trying to get right!", self.name);
                if let Some(_right_fork) = right.try_lock() {
                    println!("{} has right!", self.name);
                    println!("{} is eating!", self.name);
                    return;
                }
            }
            sleep(Duration::from_millis(
                rand::thread_rng().gen_range(10..(100 << attempts)),
            ))
            .await;
            attempts += 1;
        }
    }
}
