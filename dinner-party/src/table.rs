use std::sync::{
    mpsc::{self, Receiver, Sender},
    Arc, Mutex,
};

use crate::{fork::Fork, philosopher::Philosopher};

pub struct Table {
    total_seats: usize,
    forks: Vec<Arc<Mutex<Fork>>>,
    channels: Vec<Sender<(Arc<Mutex<Fork>>, Arc<Mutex<Fork>>)>>,
}

impl Table {
    pub fn new(total_seats: usize) -> Arc<Mutex<Self>> {
        Arc::new(Mutex::new(Self {
            total_seats,
            forks: (0..total_seats).map(|_| Fork::new()).collect(),
            channels: Vec::new(),
        }))
    }

    pub fn sit(
        &mut self,
        _philosopher: &Philosopher,
    ) -> Receiver<(Arc<Mutex<Fork>>, Arc<Mutex<Fork>>)> {
        let (sender, reciever) = mpsc::channel();
        self.channels.push(sender);
        if self.channels.len() == self.total_seats {
            self.dinner_bell();
        }
        reciever
    }

    pub fn dinner_bell(&self) {
        println!("Dinner is served!");
        for (index, channel) in self.channels.iter().enumerate() {
            channel
                .send((
                    self.forks[index].clone(),
                    self.forks[(index + 1) % self.forks.len()].clone(),
                ))
                .unwrap();
        }
    }
}
