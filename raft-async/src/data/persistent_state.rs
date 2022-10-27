use std::collections::HashSet;

use super::{data_type::CommandType, entry::Entry};

#[derive(Default)]
pub struct Config {
    pub servers: HashSet<u32>,
}

#[derive(Default)]
pub struct PersistentState<T: Clone> {
    pub id: u32,
    pub current_term: u32,
    pub voted_for: Option<u32>,
    pub log: Vec<Entry<T>>,
    pub config: Config,
    pub keep_alive: u32,
}

impl<T: CommandType> PersistentState<T> {
    pub fn push(&mut self, data: T) {
        self.log.push(Entry {
            term: self.current_term,
            command: data,
        });
    }

    pub fn log_term(&self) -> u32 {
        self.prev_log_term(self.log.len())
    }

    pub fn prev_log_term(&self, length: usize) -> u32 {
        match length {
            0 => 0,
            length => self.log[length - 1].term,
        }
    }

    pub fn quorum(&self) -> usize {
        self.config.servers.len() / 2 + 1
    }

    pub fn other_servers(&self) -> Vec<u32> {
        self.config
            .servers
            .iter()
            .filter(|id| !self.id.eq(id))
            .map(|id| *id)
            .collect()
    }
}
