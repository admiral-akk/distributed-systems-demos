use std::fmt::Debug;

use crate::raft_server::{PersistentState, VolitileState};

#[derive(Copy, Clone, Debug)]
pub struct RaftRequest<DataType>
where
    DataType: Copy + Clone + Debug,
{
    pub term: u32,
    pub sender: u32,
    pub request: RequestType<DataType>,
}

#[derive(Copy, Clone, Debug)]
pub enum RequestType<DataType>
where
    DataType: Copy + Clone + Debug,
{
    Append {
        prev_length: usize,
        prev_term: u32,
        commit_index: usize,
        entry: Option<LogEntry<DataType>>,
    },
    AppendResponse {
        success: bool,
    },
    Vote {
        log_length: usize,
        log_term: u32,
    },
    VoteResponse {
        vote: bool,
    },
}

impl<DataType> RequestType<DataType>
where
    DataType: Copy + Clone + Debug,
{
    pub fn heartbeat(log: &PersistentState<DataType>, volitile: &VolitileState) -> Self {
        Self::append(log, log.log_length(), volitile)
    }
    pub fn append(log: &PersistentState<DataType>, index: usize, volitile: &VolitileState) -> Self {
        RequestType::Append {
            prev_length: index,
            prev_term: log.prev_term(index),
            commit_index: volitile.commit_index,
            entry: log.entry(index),
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub struct LogEntry<DataType> {
    pub data: DataType,
    pub term: u32,
}
