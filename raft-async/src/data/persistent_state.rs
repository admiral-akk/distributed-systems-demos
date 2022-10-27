use std::collections::HashSet;

use super::{
    data_type::CommandType,
    request::{Event, Insert, Vote},
};

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
}

#[derive(PartialEq)]
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

    pub fn insert(&self, index: usize, max_length: usize, commit_index: usize) -> Event<T> {
        let entries = match index < self.log.len() {
            true => Vec::from(&self.log[index..(index + max_length).min(self.log.len())]),
            false => Vec::new(),
        };
        Event::Insert(Insert {
            prev_log_state: self.log_state_at(index).unwrap(),
            entries,
            leader_commit: commit_index,
        })
    }

    pub fn try_insert(&mut self, event: Insert<T>) -> bool {
        let log_state = self.log_state_at(event.prev_log_state.length);

        // If we don't have an entry at the prev_index, or if the terms don't match, we fail.
        if log_state.is_none() {
            return false;
        } else if !log_state.unwrap().eq(&event.prev_log_state) {
            return false;
        }
        for (index, entry) in event.entries.into_iter().enumerate() {
            let log_index = event.prev_log_state.length + index;
            if self.log.len() > log_index {
                if self.log[log_index].term != entry.term {
                    self.log.drain(log_index..self.log.len());
                }
            }
            if self.log.len() > log_index {
                self.log[log_index] = entry;
            } else {
                self.log.push(entry);
            }
        }
        true
    }

    pub fn try_vote_for(&mut self, event: Vote, id: u32) -> bool {
        if let Some(voted_for) = self.voted_for {
            return voted_for == id;
        }

        let log_state = self.log_state();

        // Candidate log is at least as long as follower log or same length but at least the same term.
        if log_state.length > event.log_state.length {
            return false;
        } else if log_state.length == event.log_state.length
            && log_state.term > event.log_state.term
        {
            return false;
        }
        self.voted_for = Some(id);
        true
    }

    pub fn log_state(&self) -> LogState {
        self.log_state_at(self.log.len()).unwrap()
    }

    pub fn log_state_at(&self, length: usize) -> Option<LogState> {
        match length > self.log.len() {
            true => None,
            false => Some(LogState {
                term: match length {
                    0 => 0,
                    length => self.log[length - 1].term,
                },
                length,
            }),
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
