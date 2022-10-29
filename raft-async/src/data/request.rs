use std::{collections::HashSet, fmt::Debug};

use crate::server::cluster::{Id, Message};

use super::{
    data_type::CommandType,
    persistent_state::{Config, Entry, LogState},
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

#[derive(Clone, PartialEq)]
pub enum ActiveConfig {
    Stable(Config),
    Transition { prev: Config, new: Config },
}

impl ActiveConfig {
    pub fn servers(&self) -> HashSet<u32> {
        match self {
            ActiveConfig::Stable(config) => config.servers.clone(),
            ActiveConfig::Transition { prev, new } => {
                prev.servers.union(&new.servers).map(|id| *id).collect()
            }
        }
    }

    pub fn has_quorum(&self, matching: &HashSet<u32>) -> bool {
        match self {
            ActiveConfig::Stable(config) => config.has_quorum(matching),
            ActiveConfig::Transition { prev, new } => {
                prev.has_quorum(matching) && new.has_quorum(matching)
            }
        }
    }
}

#[derive(Clone, PartialEq)]
pub enum Data<T> {
    Command(T),
    Config(ActiveConfig),
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
    pub data: Data<T>,
}

pub enum ClientResponse<T: CommandType, Output> {
    Failed {
        leader_id: Option<u32>,
        data: Data<T>,
    },
    Success {
        data: Output,
    },
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

#[cfg(test)]
pub mod test_util {
    use crate::data::persistent_state::{Entry, LogState};

    use super::{Crash, Event, Insert, InsertResponse, Request, Tick, Vote, VoteResponse};

    pub const CRASH: Request<u32, u32> = Request {
        term: 0,
        sender: 0,
        reciever: 0,
        event: Event::Crash(Crash),
    };

    pub const TICK: Request<u32, u32> = Request {
        term: 0,
        sender: 0,
        reciever: 0,
        event: Event::Tick(Tick),
    };
    pub const VOTE: Request<u32, u32> = Request {
        term: 4,
        sender: 0,
        reciever: 1,
        event: Event::Vote(Vote {
            log_state: LogState { term: 2, length: 3 },
        }),
    };

    pub const VOTE_NO_RESPONSE: Request<u32, u32> = Request {
        term: 4,
        sender: 0,
        reciever: 1,
        event: Event::VoteResponse(VoteResponse { success: false }),
    };

    pub const INSERT_SUCCESS_RESPONSE: Request<u32, u32> = Request {
        term: 4,
        sender: 0,
        reciever: 1,
        event: Event::InsertResponse(InsertResponse { success: true }),
    };

    pub const INSERT_FAILED_RESPONSE: Request<u32, u32> = Request {
        term: 4,
        sender: 0,
        reciever: 1,
        event: Event::InsertResponse(InsertResponse { success: false }),
    };
    pub fn INSERT() -> Request<u32, u32> {
        Request {
            term: 4,
            sender: 0,
            reciever: 1,
            event: Event::Insert(Insert {
                prev_log_state: LogState { term: 2, length: 3 },
                entries: [Entry::command(3, 5)].into(),
                leader_commit: 4,
            }),
        }
    }
    pub const INSERT_HEARTBEAT: Request<u32, u32> = Request {
        term: 4,
        sender: 0,
        reciever: 1,
        event: Event::Insert(Insert {
            prev_log_state: LogState { term: 2, length: 3 },
            entries: Vec::new(),
            leader_commit: 3,
        }),
    };
}
