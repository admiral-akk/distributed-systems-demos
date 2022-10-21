use std::{
    sync::{Arc, Mutex, MutexGuard},
    thread,
    time::Duration,
};

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

pub trait Eat {
    fn try_eat(&self, left: Arc<Mutex<Fork>>, right: Arc<Mutex<Fork>>);
    fn backoff(&self, attempts: u32) {
        let backoff = rand::thread_rng().gen_range(10..(128 << attempts));
        thread::sleep(Duration::from_millis(backoff));
    }
    fn eat(&self, left: &MutexGuard<Fork>, right: &MutexGuard<Fork>) {
        thread::sleep(Duration::from_millis(1000));
    }
}

impl Eat for Philosopher {
    fn try_eat(&self, left: Arc<Mutex<Fork>>, right: Arc<Mutex<Fork>>) {
        let mut attempts = 0;
        loop {
            {
                println!("{} trying to pick up left fork!", self.to_string());
                let left_fork = left.lock().unwrap();
                println!("{} picked up left fork!", self.to_string());
                thread::sleep(Duration::from_millis(1000));
                println!("{} trying to pick up right fork!", self.to_string());
                let try_right = right.try_lock();
                if let Ok(right_fork) = try_right {
                    println!("{} picked up right fork!", self.to_string());
                    println!("{} is eating.", self.to_string());
                    self.eat(&left_fork, &right_fork);
                    println!("{} is done eating.", self.to_string());
                    return;
                }
            }
            attempts += 1;
            self.backoff(attempts);
        }
    }
}

impl Philosopher {
    pub fn new(name: String) -> Self {
        Self { name }
    }
}
