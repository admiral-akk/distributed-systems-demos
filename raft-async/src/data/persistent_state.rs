use std::collections::HashSet;

use super::data_type::CommandType;

#[derive(Default)]
pub struct Config {
    pub servers: HashSet<u32>,
}

#[derive(Clone, PartialEq)]
pub struct Entry<T: Clone> {
    pub term: u32,
    pub command: T,
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

pub struct LogState {
    pub term: u32,
    pub length: usize,
}

impl<T: CommandType> PersistentState<T> {
    pub fn push(&mut self, data: T) {
        self.log.push(Entry {
            term: self.current_term,
            command: data,
        });
    }

    pub fn log_state(&self) -> LogState {
        self.log_state_at(self.log.len())
    }

    pub fn log_state_at(&self, length: usize) -> LogState {
        LogState {
            term: match length {
                0 => 0,
                length => self.log[length - 1].term,
            },
            length,
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
