use std::collections::HashSet;

use super::{
    data_type::CommandType,
    request::{Data, Event, Insert, Vote},
};

#[derive(Default, Clone, PartialEq)]
pub struct Config {
    pub servers: HashSet<u32>,
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
            data: Data::Config(config),
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

pub enum ActiveConfig {
    None,
    Stable(Config),
    Transition { prev: Config, latest: Config },
}

impl<T: CommandType> PersistentState<T> {
    fn latest_configs(&self) -> ActiveConfig {
        let mut configs = self.log.iter().rev().filter(|entry| match entry.data {
            Data::Command(_) => false,
            Data::Config(_) => true,
        });
        match configs.next() {
            Some(Entry {
                data: Data::Config(latest),
                ..
            }) => {
                let latest = latest.clone();
                match configs.next() {
                    Some(Entry {
                        data: Data::Config(prev),
                        ..
                    }) => {
                        let prev = prev.clone();
                        ActiveConfig::Transition { prev, latest }
                    }
                    _ => ActiveConfig::Stable(latest.clone()),
                }
            }
            _ => ActiveConfig::None,
        }
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
        let latest = self.latest_configs();
        match latest {
            ActiveConfig::None => false,
            ActiveConfig::Stable(config) => {
                let valid_votes = config.servers.intersection(&matching).count();
                valid_votes >= config.servers.len() / 2 + 1
            }
            ActiveConfig::Transition { prev, latest } => false,
        }
    }

    pub fn other_servers(&self) -> Vec<u32> {
        let latest = self
            .log
            .iter()
            .rev()
            .filter(|e| match e.data {
                Data::Config(_) => true,
                _ => false,
            })
            .next();
        match latest {
            Some(Entry {
                data: Data::Config(Config { servers }),
                ..
            }) => servers
                .iter()
                .filter(|id| !self.id.eq(id))
                .map(|id| *id)
                .collect(),
            _ => Vec::new(),
        }
    }
}
