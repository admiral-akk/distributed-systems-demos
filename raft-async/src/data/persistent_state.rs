use std::collections::HashSet;

use super::{
    data_type::CommandType,
    request::{ActiveConfig, Data, Event, Insert, Vote},
};

#[derive(Default, Clone, PartialEq)]
pub struct Config {
    pub servers: HashSet<u32>,
}

impl Config {
    pub fn has_quorum(&self, votes: &HashSet<u32>) -> bool {
        self.servers.intersection(votes).count() >= self.servers.len() / 2 + 1
    }
}

#[derive(Clone, PartialEq)]
pub struct Entry<T: Clone> {
    pub term: u32,
    pub data: Data<T>,
}

impl<T: Clone> Entry<T> {
    pub fn command(term: u32, data: T) -> Self {
        Entry {
            term,
            data: Data::Command(data),
        }
    }

    pub fn config(term: u32, config: Config) -> Self {
        Entry {
            term,
            data: Data::Config(ActiveConfig::Stable(config)),
        }
    }
}

#[derive(Default)]
pub struct PersistentState<T: Clone> {
    pub id: u32,
    pub current_term: u32,
    pub voted_for: Option<u32>,
    pub log: Vec<Entry<T>>,
}

#[derive(PartialEq)]
pub struct LogState {
    pub term: u32,
    pub length: usize,
}

pub struct LatestConfig {
    pub config: ActiveConfig,
    pub committed: bool,
}

impl LatestConfig {
    pub fn servers(&self) -> HashSet<u32> {
        match &self.config {
            ActiveConfig::Stable(config) => config.servers.clone(),
            ActiveConfig::Transition { prev, new } => {
                prev.servers.union(&new.servers).map(|id| *id).collect()
            }
        }
    }
}

impl<T: CommandType> PersistentState<T> {
    pub fn latest_config(&self, commit_index: usize) -> LatestConfig {
        self.log
            .iter()
            .enumerate()
            .rev()
            .filter(|(i, entry)| match entry.data {
                Data::Command(_) => false,
                Data::Config(_) => true,
            })
            .map(|(i, entry)| match &entry.data {
                Data::Config(active_config) => LatestConfig {
                    config: active_config.clone(),
                    committed: i < commit_index,
                },
                _ => panic!(),
            })
            .next()
            .unwrap()
    }

    pub fn push(&mut self, data: Data<T>) {
        self.log.push(Entry {
            term: self.current_term,
            data,
        });
    }

    pub fn insert<Output>(
        &self,
        index: usize,
        max_length: usize,
        commit_index: usize,
    ) -> Event<T, Output> {
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

    pub fn has_quorum(&self, matching: &HashSet<u32>) -> bool {
        self.latest_config(self.log.len())
            .config
            .has_quorum(matching)
    }

    pub fn other_servers(&self) -> Vec<u32> {
        self.latest_config(self.log.len())
            .servers()
            .into_iter()
            .filter(|server| !self.id.eq(server))
            .collect()
    }
}
