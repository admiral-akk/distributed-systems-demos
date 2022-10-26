use std::collections::HashSet;

use super::{data_type::CommandType, entry::Entry};

#[derive(Default)]
pub struct Config {
    pub servers: HashSet<u32>,
}

#[derive(Default)]
pub struct PersistentState<T: CommandType> {
    pub id: u32,
    pub current_term: u32,
    pub voted_for: Option<u32>,
    pub log: Vec<Entry<T>>,
    pub config: Config,
    pub keep_alive: u32,
}

impl<T: CommandType> PersistentState<T> {
    pub fn prev_term(&self, index: usize) -> u32 {
        if self.log.is_empty() || index == 0 {
            0
        } else if index <= self.log.len() {
            self.log[index - 1].term
        } else {
            self.log.last().unwrap().term
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
