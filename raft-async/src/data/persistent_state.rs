use std::{collections::HashSet, time::SystemTime};

use crate::raft_server::VolitileState;

use super::{
    data_type::DataType,
    entry::Entry,
    request::{Request, RequestType},
};

#[derive(Default)]
pub struct Config {
    pub servers: HashSet<u32>,
}

#[derive(Default)]
pub struct PersistentState<T: DataType> {
    pub id: u32,
    pub current_term: u32,
    pub voted_for: Option<u32>,
    pub log: Vec<Entry<T>>,
    pub config: Config,
    pub keep_alive: u32,
}

impl<T: DataType> PersistentState<T> {
    fn request(&self, data: RequestType<T>, reciever: u32) -> Request<T> {
        Request {
            sender: self.id,
            reciever,
            term: self.current_term,
            data,
        }
    }

    fn prev_term(&self, index: usize) -> u32 {
        if self.log.is_empty() || index == 0 {
            0
        } else if index <= self.log.len() {
            self.log[index - 1].term
        } else {
            self.log.last().unwrap().term
        }
    }
    pub fn append(&self, volitile_state: &VolitileState, index: usize, server: u32) -> Request<T> {
        self.request(
            RequestType::Append {
                prev_log_length: index,
                prev_log_term: self.prev_term(index),
                entries: Vec::from(&self.log[index..self.log.len()]),
                leader_commit: volitile_state.commit_index,
            },
            server,
        )
    }

    pub fn request_votes(&self) -> Vec<Request<T>> {
        self.other_servers()
            .iter()
            .map(|server| {
                let log_length = self.log.len();
                self.request(
                    RequestType::Vote {
                        log_length,
                        last_log_term: self.prev_term(log_length),
                    },
                    *server,
                )
            })
            .collect()
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
