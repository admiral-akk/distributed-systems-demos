use std::{fs, time::Duration};

use fork::Fork;
use philosopher::Philosopher;
use rand::Rng;

mod fork;
mod philosopher;

async fn philosopher(name: &String) -> Philosopher {
    let wait_time = rand::thread_rng().gen_range(10..1000);
    async_std::task::sleep(Duration::from_millis(wait_time)).await;
    println!("{} has arrived! {} ms delay", name, wait_time);
    Philosopher::new(name.to_string())
}

fn main() {
    let names = fs::read_to_string("philosophers.txt")
        .expect("Where are my philosophers?")
        .split("\n")
        .map(|s| s.to_string())
        .collect::<Vec<_>>();

    let forks = (0..names.len()).map(|_| Fork::new()).collect::<Vec<_>>();

    let philosophers = names
        .iter()
        .map(|name| philosopher(name))
        .collect::<Vec<_>>();

    let philosophers = futures::executor::block_on(futures::future::join_all(philosophers));

    let eating = philosophers
        .iter()
        .enumerate()
        .map(|(i, philosopher)| {
            philosopher.eat(forks[i].clone(), forks[(i + 1) % forks.len()].clone())
        })
        .collect::<Vec<_>>();

    futures::executor::block_on(futures::future::join_all(eating));
}
