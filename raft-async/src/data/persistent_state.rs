use std::collections::HashSet;

use crate::server::raft_cluster::Id;

use super::{
    data_type::CommandType,
    request::{ActiveConfig, Data, Event, Insert, Transaction, TransactionId, Vote},
};

#[derive(Default, Clone, Debug, PartialEq, Eq)]
pub struct Config {
    pub servers: HashSet<Id>,
}

impl Config {
    pub fn has_quorum(&self, votes: &HashSet<Id>) -> bool {
        self.servers.intersection(votes).count() >= self.servers.len() / 2 + 1
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry<T: Clone> {
    pub term: u32,
    pub data: Transaction<T>,
}

impl<T: Clone> Entry<T> {
    pub fn command(term: u32, id: TransactionId, data: T) -> Self {
        Entry {
            term,
            data: Transaction {
                id,
                data: Data::Command(data),
            },
        }
    }
    pub fn transition_config(term: u32, id: TransactionId, prev: Config, new: Config) -> Self {
        Entry {
            term,
            data: Transaction {
                id,
                data: Data::Config(ActiveConfig::Transition { prev, new }),
            },
        }
    }

    pub fn stable_config(term: u32, id: TransactionId, config: Config) -> Self {
        Entry {
            term,
            data: Transaction {
                id,
                data: Data::Config(ActiveConfig::Stable(config)),
            },
        }
    }
}

#[derive(Default, Debug, PartialEq, Clone)]
pub struct PersistentState<T: Clone> {
    pub id: Id,
    pub current_term: u32,
    pub voted_for: Option<Id>,
    pub log: Vec<Entry<T>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LogState {
    pub term: u32,
    pub length: usize,
}

pub struct LatestConfig {
    pub config: ActiveConfig,
    pub transaction_id: TransactionId,
    pub committed: bool,
}

impl LatestConfig {
    pub fn servers(&self) -> HashSet<Id> {
        match &self.config {
            ActiveConfig::Stable(config) => config.servers.clone(),
            ActiveConfig::Transition { prev, new } => {
                prev.servers.union(&new.servers).map(|id| *id).collect()
            }
        }
    }
}

impl<T: CommandType> PersistentState<T> {
    pub fn latest_config(&self, commit_index: usize) -> (Option<LatestConfig>, LatestConfig) {
        let mut configs = self
            .log
            .iter()
            .enumerate()
            .rev()
            .filter(|(_i, entry)| match entry.data.data {
                Data::Command(_) => false,
                Data::Config(_) => true,
            })
            .map(|(i, entry)| match &entry.data.data {
                Data::Config(config) => LatestConfig {
                    config: config.clone(),
                    committed: i < commit_index,
                    transaction_id: entry.data.id,
                },
                _ => panic!(),
            });
        let next = configs.next().unwrap();
        let prev = configs.next();
        (prev, next)
    }

    pub fn has_quorum(&self, commit_index: usize, matching: &HashSet<Id>) -> bool {
        let (prev, latest) = self.latest_config(commit_index);
        match (prev, &latest) {
            (
                Some(LatestConfig {
                    config,
                    committed: true,
                    ..
                }),
                LatestConfig {
                    committed: false, ..
                },
            ) => config.has_quorum(matching),
            _ => latest.config.has_quorum(matching),
        }
    }
    pub fn servers(&self, commit_index: usize) -> HashSet<Id> {
        let (prev, latest) = self.latest_config(commit_index);
        match (&prev, &latest) {
            (
                Some(LatestConfig {
                    config: previous, ..
                }),
                LatestConfig {
                    committed: false,
                    config: latest,
                    ..
                },
            ) => match latest {
                ActiveConfig::Stable(_) => previous,
                ActiveConfig::Transition { .. } => latest,
            },
            _ => &latest.config,
        }
        .servers()
    }

    pub fn other_servers(&self, commit_index: usize) -> Vec<Id> {
        let (prev, latest) = self.latest_config(commit_index);
        match (&prev, &latest) {
            (
                Some(LatestConfig {
                    config: previous, ..
                }),
                LatestConfig {
                    committed: false,
                    config: latest,
                    ..
                },
            ) => match latest {
                ActiveConfig::Stable(_) => previous,
                ActiveConfig::Transition { .. } => latest,
            },
            _ => &latest.config,
        }
        .servers()
        .into_iter()
        .filter(|server| !self.id.eq(server))
        .collect()
    }

    pub fn push(&mut self, data: Transaction<T>) {
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

    pub fn try_vote_for(&mut self, event: Vote, id: Id) -> bool {
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
}

#[cfg(test)]
pub mod test_util {
    use crate::{
        data::request::{
            test_util::{TRANSACTION_ID_1, TRANSACTION_ID_2, TRANSACTION_ID_NONE},
            ActiveConfig, Data, Transaction,
        },
        server::raft_cluster::{
            test_util::{SERVER_0, SERVER_1, SERVER_2, SERVER_3, SERVER_4, SERVER_5, SERVER_6},
            Id,
        },
    };

    use super::{Config, Entry, PersistentState};

    impl<T: Clone> PersistentState<T> {
        pub fn set_voted(mut self, voted_for: Id) -> Self {
            self.voted_for = Some(voted_for);
            self
        }

        pub fn increment_term(mut self) -> Self {
            self.current_term += 1;
            self
        }
    }

    pub fn CONFIG() -> Config {
        Config {
            servers: [SERVER_0, SERVER_1, SERVER_2, SERVER_3, SERVER_4].into(),
        }
    }
    pub fn NEW_CONFIG() -> Config {
        Config {
            servers: [SERVER_2, SERVER_3, SERVER_4, SERVER_5, SERVER_6].into(),
        }
    }
    pub fn JOINT_CONFIG() -> Config {
        Config {
            servers: [
                SERVER_0, SERVER_1, SERVER_2, SERVER_3, SERVER_4, SERVER_5, SERVER_6,
            ]
            .into(),
        }
    }

    pub fn LOG_TRANSITION_CONFIG() -> Vec<Entry<u32>> {
        Vec::from([
            Entry::stable_config(0, TRANSACTION_ID_NONE, CONFIG()),
            Entry::transition_config(1, TRANSACTION_ID_1, CONFIG(), NEW_CONFIG()),
        ])
    }

    pub fn LOG_TRANSITION_STABLE_CONFIG() -> Vec<Entry<u32>> {
        Vec::from([
            Entry::stable_config(0, TRANSACTION_ID_NONE, CONFIG()),
            Entry::transition_config(1, TRANSACTION_ID_1, CONFIG(), NEW_CONFIG()),
            Entry::stable_config(3, TRANSACTION_ID_1, NEW_CONFIG()),
        ])
    }

    pub fn LOG_LEADER() -> Vec<Entry<u32>> {
        let mut log = LOG();
        log.extend(
            [
                Entry::command(3, TRANSACTION_ID_1, 5),
                Entry::command(3, TRANSACTION_ID_2, 1),
                Entry::command(3, TRANSACTION_ID_2, 2),
            ]
            .into_iter(),
        );
        log
    }

    pub fn LOG() -> Vec<Entry<u32>> {
        Vec::from([
            Entry::stable_config(0, TRANSACTION_ID_NONE, CONFIG()),
            Entry::command(1, TRANSACTION_ID_2, 10),
            Entry::command(3, TRANSACTION_ID_2, 4),
        ])
    }

    pub fn LOG_WITH_CLIENT() -> Vec<Entry<u32>> {
        let mut log = LOG();
        log.extend([Entry::command(4, TRANSACTION_ID_1, 100)].into_iter());
        log
    }

    pub fn MISMATCH_LOG() -> Vec<Entry<u32>> {
        Vec::from([
            Entry::stable_config(0, TRANSACTION_ID_NONE, CONFIG()),
            Entry::command(1, TRANSACTION_ID_2, 10),
            Entry::command(2, TRANSACTION_ID_2, 3),
        ])
    }

    pub fn PERSISTENT_STATE() -> PersistentState<u32> {
        PersistentState {
            id: SERVER_1,
            current_term: 4,
            voted_for: None,
            log: LOG(),
        }
    }

    pub fn PERSISTENT_STATE_LOG(log: Vec<Entry<u32>>) -> PersistentState<u32> {
        PersistentState {
            id: SERVER_1,
            current_term: 4,
            voted_for: None,
            log,
        }
    }

    pub fn PERSISTENT_STATE_VOTED(id: Id) -> PersistentState<u32> {
        PersistentState {
            id: SERVER_1,
            current_term: 4,
            voted_for: Some(id),
            log: LOG(),
        }
    }
}
