use std::fmt::Debug;

use crate::server::switch::{Id, Message};

use super::{
    data_type::CommandType,
    persistent_state::{Entry, LogState},
};

pub struct Request<T: CommandType, Output> {
    // Todo: figure out better framing for sender/reciever/term, since it's not relevant to all events.
    pub sender: u32,
    pub reciever: u32,
    pub term: u32,
    pub event: Event<T, Output>,
}

impl<T: CommandType + Send, Output: Debug + Send + 'static> Message for Request<T, Output> {
    fn recipient(&self) -> Id {
        Id::new(self.reciever)
    }
}

pub enum Event<T: CommandType, Output> {
    Insert(Insert<T>),
    InsertResponse(InsertResponse),
    Vote(Vote),
    VoteResponse(VoteResponse),
    Crash(Crash),
    Tick(Tick),
    Client(Client<T>),
    ClientResponse(ClientResponse<T, Output>),
}
pub struct Crash;
pub struct Client<T: CommandType> {
    pub data: T,
}

pub enum ClientResponse<T: CommandType, Output> {
    Failed { leader_id: Option<u32>, data: T },
    Success { data: Output },
}

pub struct Insert<T: CommandType> {
    pub prev_log_state: LogState,
    pub entries: Vec<Entry<T>>,
    pub leader_commit: usize,
}
impl<T: CommandType> Insert<T> {
    pub fn max_commit_index(&self) -> usize {
        self.leader_commit
            .min(self.prev_log_state.length + self.entries.len())
    }
}

pub struct InsertResponse {
    pub success: bool,
}
pub struct Vote {
    pub log_state: LogState,
}
pub struct VoteResponse {
    pub success: bool,
}
pub struct Tick;
