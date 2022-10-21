use std::sync::Mutex;
use std::thread;

use fork::Fork;
use philosopher::Philosopher;
use rand::Rng;
use std::fs;
use std::time::Duration;
use table::Table;

use crate::philosopher::Eat;

mod fork;
mod philosopher;
mod table;

fn main() {
    let names = fs::read_to_string("philosophers.txt")
        .expect("Where are my philosophers?")
        .split("\n")
        .map(|s| s.to_string())
        .collect::<Vec<_>>();

    let table = Table::new(names.len());

    let philosophers = names
        .iter()
        .enumerate()
        .map(|(index, name)| {
            let name = name.to_string();
            let arrival_delay = rand::thread_rng().gen_range(1..4000);
            let table = table.clone();
            thread::spawn(move || {
                thread::sleep(Duration::from_millis(arrival_delay));
                let philosopher = Philosopher::new(name);
                println!(
                    "{} has arrived... {} ms late",
                    philosopher.to_string(),
                    arrival_delay
                );
                let reciever;
                {
                    reciever = table.lock().unwrap().sit(&philosopher);
                }
                let (left, right) = reciever.recv().unwrap();
                {
                    philosopher.try_eat(left, right);
                }
            })
        })
        .collect::<Vec<_>>();
    for t in philosophers {
        t.join().unwrap();
    }
}
