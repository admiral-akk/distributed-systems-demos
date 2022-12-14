use std::{collections::HashSet, fmt::Debug};

use crate::server::raft_cluster::{Id, Message};

use super::{
    data_type::CommandType,
    persistent_state::{Config, Entry, LogState},
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Request<T: CommandType, Output> {
    // Todo: figure out better framing for sender/reciever/term, since it's not relevant to all events.
    pub sender: Id,
    pub reciever: Id,
    pub term: u32,
    pub event: Event<T, Output>,
}

impl<T: CommandType + Send, Output: Debug + Send + 'static> Message for Request<T, Output> {
    fn recipient(&self) -> Id {
        self.reciever
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ActiveConfig {
    Stable(Config),
    Transition { prev: Config, new: Config },
}

impl ActiveConfig {
    pub fn servers(&self) -> HashSet<Id> {
        match self {
            ActiveConfig::Stable(config) => config.servers.clone(),
            ActiveConfig::Transition { prev, new } => {
                prev.servers.union(&new.servers).map(|id| *id).collect()
            }
        }
    }

    pub fn has_quorum(&self, matching: &HashSet<Id>) -> bool {
        match self {
            ActiveConfig::Stable(config) => config.has_quorum(matching),
            ActiveConfig::Transition { prev, new } => {
                prev.has_quorum(matching) && new.has_quorum(matching)
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Data<T> {
    Command(T),
    Config(ActiveConfig),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Transaction<T> {
    pub id: TransactionId,
    pub data: Data<T>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ClientData<T> {
    Command(T),
    Config(Config),
}

#[derive(Clone, Debug, PartialEq, Eq)]
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
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Crash;
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Client<T: CommandType> {
    pub id: TransactionId,
    pub data: ClientData<T>,
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub struct TransactionId(pub Id, pub u32);

impl TransactionId {
    pub const NONE: TransactionId = TransactionId(Id::None, 0);
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ResponseEvent<T: CommandType, Output> {
    Failed {
        leader_id: Option<Id>,
        data: ClientData<T>,
    },
    Success {
        data: Output,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]

pub enum ClientResponse<T: CommandType, Output> {
    Failed {
        id: TransactionId,
        leader_id: Option<Id>,
        data: ClientData<T>,
    },
    Success {
        id: TransactionId,
        data: Output,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InsertResponse {
    pub success: bool,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Vote {
    pub log_state: LogState,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VoteResponse {
    pub success: bool,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Tick;

#[cfg(test)]
pub mod test_util {

    use super::{
        ActiveConfig, Client, ClientData, Crash, Data, Event, Insert, InsertResponse, Request,
        Tick, TransactionId, Vote, VoteResponse,
    };
    use crate::{
        data::{
            data_type::CommandType,
            persistent_state::{
                test_util::{CONFIG, LOG_LEADER, NEW_CONFIG},
                Config, LogState,
            },
        },
        server::raft_cluster::{
            test_util::{CLIENT_0, NONE, SERVER_0, SERVER_1},
            Id,
        },
    };

    impl<In: CommandType, Out> Request<In, Out> {
        pub fn reverse_sender(mut self) -> Self {
            let reciever = self.sender;
            self.sender = self.reciever;
            self.reciever = reciever;
            self
        }

        pub fn set_term(mut self, term: u32) -> Self {
            self.term = term;
            self
        }

        pub fn set_sender(mut self, sender: Id) -> Self {
            self.sender = sender;
            self
        }
    }

    impl<In: CommandType> Insert<In> {}

    pub const TRANSACTION_ID_1: TransactionId = TransactionId(Id::Client(10), 1);
    pub const TRANSACTION_ID_2: TransactionId = TransactionId(Id::Client(10), 2);
    pub const TRANSACTION_ID_NONE: TransactionId = TransactionId(Id::None, 0);

    pub const CRASH: Request<u32, u32> = Request {
        term: 0,
        sender: NONE,
        reciever: NONE,
        event: Event::Crash(Crash),
    };

    pub const TICK: Request<u32, u32> = Request {
        term: 0,
        sender: NONE,
        reciever: NONE,
        event: Event::Tick(Tick),
    };

    pub fn REQUEST_VOTES(term: u32) -> Vec<Request<u32, u32>> {
        let request = VOTE.reverse_sender().set_term(term);
        CONFIG()
            .servers
            .iter()
            .filter(|id| !request.sender.eq(id))
            .map(|id| {
                let mut request = request.clone();
                request.reciever = *id;
                request
            })
            .collect()
    }

    pub const VOTE: Request<u32, u32> = Request {
        term: 4,
        sender: SERVER_0,
        reciever: SERVER_1,
        event: Event::Vote(Vote {
            log_state: LogState { term: 3, length: 3 },
        }),
    };

    pub const VOTE_NEW_SHORT: Request<u32, u32> = Request {
        term: 4,
        sender: SERVER_0,
        reciever: SERVER_1,
        event: Event::Vote(Vote {
            log_state: LogState { term: 4, length: 2 },
        }),
    };

    pub const VOTE_OLD_EQUAL: Request<u32, u32> = Request {
        term: 4,
        sender: SERVER_0,
        reciever: SERVER_1,
        event: Event::Vote(Vote {
            log_state: LogState { term: 2, length: 3 },
        }),
    };
    pub const VOTE_OLD_LONG: Request<u32, u32> = Request {
        term: 4,
        sender: SERVER_0,
        reciever: SERVER_1,
        event: Event::Vote(Vote {
            log_state: LogState { term: 2, length: 4 },
        }),
    };

    pub const VOTE_NO_RESPONSE: Request<u32, u32> = Request {
        term: 4,
        sender: SERVER_0,
        reciever: SERVER_1,
        event: Event::VoteResponse(VoteResponse { success: false }),
    };

    pub const VOTE_YES_RESPONSE: Request<u32, u32> = Request {
        term: 4,
        sender: SERVER_0,
        reciever: SERVER_1,
        event: Event::VoteResponse(VoteResponse { success: true }),
    };

    pub const INSERT_SUCCESS_RESPONSE: Request<u32, u32> = Request {
        term: 4,
        sender: SERVER_0,
        reciever: SERVER_1,
        event: Event::InsertResponse(InsertResponse { success: true }),
    };

    pub fn INSERT(prev_length: usize) -> Request<u32, u32> {
        let log = LOG_LEADER();
        let max_prev_index = (prev_length - 1).min(log.len() - 1);
        Request {
            term: 4,
            sender: SERVER_0,
            reciever: SERVER_1,
            event: Event::Insert(Insert {
                prev_log_state: LogState {
                    term: log[max_prev_index].term,
                    length: prev_length,
                },
                entries: log[(max_prev_index + 1)..(max_prev_index + 2)].into(),
                leader_commit: 4,
            }),
        }
    }

    pub fn MASS_HEARTBEAT(term: u32) -> Vec<Request<u32, u32>> {
        MASS_HEARTBEAT_WITH_RANGE(term, CONFIG())
    }

    pub fn MASS_HEARTBEAT_WITH_RANGE(term: u32, config: Config) -> Vec<Request<u32, u32>> {
        let request = INSERT_HEARTBEAT.reverse_sender().set_term(term);
        config
            .servers
            .iter()
            .filter(|id| !request.sender.eq(id))
            .map(|id| {
                let mut request = request.clone();
                request.reciever = *id;
                request
            })
            .collect()
    }

    pub const INSERT_HEARTBEAT: Request<u32, u32> = Request {
        term: 4,
        sender: SERVER_0,
        reciever: SERVER_1,
        event: Event::Insert(Insert {
            prev_log_state: LogState { term: 3, length: 3 },
            entries: Vec::new(),
            leader_commit: 2,
        }),
    };

    pub const INSERT_FAILED_RESPONSE: Request<u32, u32> = Request {
        term: 4,
        sender: SERVER_0,
        reciever: SERVER_1,
        event: Event::InsertResponse(InsertResponse { success: false }),
    };

    pub const CLIENT_COMMAND: Request<u32, u32> = Request {
        term: 0,
        sender: CLIENT_0,
        reciever: SERVER_1,
        event: Event::Client(Client {
            id: TRANSACTION_ID_1,
            data: ClientData::Command(100),
        }),
    };

    pub const CLIENT_RESPONSE_NO_LEADER: Request<u32, u32> = Request {
        term: 0,
        sender: SERVER_1,
        reciever: CLIENT_0,
        event: Event::ClientResponse(super::ClientResponse::Failed {
            leader_id: None,
            id: TRANSACTION_ID_1,
            data: ClientData::Command(100),
        }),
    };

    pub const CLIENT_RESPONSE_WITH_LEADER: Request<u32, u32> = Request {
        term: 0,
        sender: SERVER_1,
        reciever: CLIENT_0,
        event: Event::ClientResponse(super::ClientResponse::Failed {
            leader_id: Some(SERVER_0),
            id: TRANSACTION_ID_1,
            data: ClientData::Command(100),
        }),
    };

    pub fn CLIENT_CONFIG() -> Request<u32, u32> {
        Request {
            term: 0,
            sender: CLIENT_0,
            reciever: SERVER_1,
            event: Event::Client(Client {
                id: TRANSACTION_ID_1,
                data: ClientData::Config(NEW_CONFIG()),
            }),
        }
    }
}
