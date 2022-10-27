use std::{collections::HashMap, time::Duration};

use crate::data::{
    data_type::CommandType,
    persistent_state::PersistentState,
    request::{
        Client, ClientResponse, Event, Insert, InsertResponse, Request, Timeout, Vote, VoteResponse,
    },
    volitile_state::VolitileState,
};
use crate::state::{
    handler::{EventHandler, Handler, TimeoutHandler},
    raft_state::RaftState,
};

use super::candidate::Candidate;

pub struct Leader {
    pub next_index: HashMap<u32, usize>,
    pub match_index: HashMap<u32, usize>,
}

impl TimeoutHandler for Leader {
    fn timeout_length(&self) -> Duration {
        Duration::from_millis(150)
    }
}

impl Leader {
    pub fn send_heartbeat<T: CommandType>(
        &self,
        volitile_state: &mut VolitileState,
        persistent_state: &mut PersistentState<T>,
    ) -> Vec<Request<T>> {
        persistent_state
            .other_servers()
            .iter()
            .map(|server| self.append_update(volitile_state, persistent_state, *server))
            .collect()
    }

    fn append_update<T: CommandType>(
        &self,
        volitile_state: &mut VolitileState,
        persistent_state: &mut PersistentState<T>,
        server: u32,
    ) -> Request<T> {
        Request {
            sender: persistent_state.id,
            reciever: server,
            term: persistent_state.current_term,
            event: persistent_state.insert(
                self.next_index[&server],
                1,
                volitile_state.commit_index,
            ),
        }
    }

    pub fn from_candidate<T: CommandType>(
        _candidate: &Candidate,
        volitile_state: &mut VolitileState,
        persistent_state: &mut PersistentState<T>,
    ) -> (Vec<Request<T>>, Option<RaftState>) {
        persistent_state.current_term += 1;
        persistent_state.keep_alive += 1;
        println!("{} elected leader!", persistent_state.id);
        let leader = Leader {
            next_index: persistent_state
                .other_servers()
                .iter()
                .map(|id| (*id, persistent_state.log.len()))
                .collect(),
            match_index: persistent_state
                .other_servers()
                .iter()
                .map(|id| (*id, 0))
                .collect(),
        };
        let heartbeat = leader.send_heartbeat(volitile_state, persistent_state);
        (heartbeat, Some(leader.into()))
    }
}

impl<T: CommandType> Handler<T> for Leader {}
impl<T: CommandType> EventHandler<Vote, T> for Leader {}
impl<T: CommandType> EventHandler<Client<T>, T> for Leader {
    fn handle_event(
        &mut self,
        _volitile_state: &mut VolitileState,
        persistent_state: &mut PersistentState<T>,
        _sender: u32,
        _term: u32,
        event: Client<T>,
    ) -> (Vec<Request<T>>, Option<RaftState>) {
        persistent_state.push(event.data);
        (Vec::new(), None)
    }
}
impl<T: CommandType> EventHandler<ClientResponse<T>, T> for Leader {}
impl<T: CommandType> EventHandler<VoteResponse, T> for Leader {}
impl<T: CommandType> EventHandler<Timeout, T> for Leader {
    fn handle_event(
        &mut self,
        volitile_state: &mut VolitileState,
        persistent_state: &mut PersistentState<T>,
        _sender: u32,
        _term: u32,
        _event: Timeout,
    ) -> (Vec<Request<T>>, Option<RaftState>) {
        (self.send_heartbeat(volitile_state, persistent_state), None)
    }
}
impl<T: CommandType> EventHandler<Insert<T>, T> for Leader {}
impl<T: CommandType> EventHandler<InsertResponse, T> for Leader {
    fn handle_event(
        &mut self,
        volitile_state: &mut VolitileState,
        persistent_state: &mut PersistentState<T>,
        sender: u32,
        _term: u32,
        event: InsertResponse,
    ) -> (Vec<Request<T>>, Option<RaftState>) {
        let next_index = self.next_index[&sender];
        if event.success {
            if next_index < persistent_state.log.len() {
                self.next_index.insert(sender, next_index + 1);
            }
            self.match_index.insert(sender, self.next_index[&sender]);
        } else if next_index > 0 {
            self.next_index.insert(sender, next_index - 1);
        }
        let matching_servers = self
            .match_index
            .iter()
            .filter(|(_, v)| **v >= next_index)
            .count();

        if matching_servers + 1 > persistent_state.quorum()
            && volitile_state.commit_index < next_index
        {
            volitile_state.commit_index = next_index;
            println!("{} index committed!", next_index);
        }
        (Vec::default(), None)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use crate::data::persistent_state::{Config, Entry};
    use crate::data::request;
    // Note this useful idiom: importing names from outer (for mod tests) scope.
    use super::*;

    #[test]
    fn test_elected() {
        let config = Config {
            servers: HashSet::from([0, 1, 2, 3, 4]),
        };
        let mut persistent_state: PersistentState<u32> = PersistentState {
            config,
            id: 1,
            current_term: 3,
            log: Vec::from([
                Entry {
                    term: 1,
                    command: 10,
                },
                Entry {
                    term: 3,
                    command: 4,
                },
            ]),
            ..Default::default()
        };
        let mut volitile_state = VolitileState { commit_index: 1 };

        let (requests, next) = Leader::from_candidate(
            &Candidate::default(),
            &mut volitile_state,
            &mut persistent_state,
        );

        assert!(next.is_some());
        if let Some(RaftState::Leader(Leader {
            next_index,
            match_index,
        })) = next
        {
            for (_, v) in next_index {
                assert_eq!(v, persistent_state.log.len());
            }
            for (_, v) in match_index {
                assert_eq!(v, 0);
            }
        } else {
            panic!("Transitioned to non-leader state!");
        }
        assert_eq!(persistent_state.keep_alive, 1);
        assert!(requests.len() == 4);
        for request in requests {
            assert!(request.sender == persistent_state.id);
            assert!(request.term == persistent_state.current_term);
            match request.event {
                Event::Insert(event) => {
                    assert_eq!(event.prev_log_state.length, 2);
                    assert_eq!(event.prev_log_state.term, 3);
                    assert_eq!(event.leader_commit, 1);
                    assert!(event.entries.is_empty());
                }
                _ => {
                    panic!("Non-append event!")
                }
            }
        }
    }

    #[test]
    fn test_timeout() {
        let config = Config {
            servers: HashSet::from([0, 1, 2, 3, 4]),
        };
        let mut persistent_state: PersistentState<u32> = PersistentState {
            config,
            id: 1,
            current_term: 3,
            log: Vec::from([
                Entry {
                    term: 1,
                    command: 10,
                },
                Entry {
                    term: 3,
                    command: 4,
                },
            ]),
            ..Default::default()
        };
        let mut volitile_state = VolitileState { commit_index: 1 };
        let request: Request<u32> = Request {
            sender: 0,
            reciever: persistent_state.id,
            term: 0,
            event: Event::Timeout(request::Timeout),
        };
        let mut leader = Leader {
            next_index: HashMap::from([(0, 2), (2, 2), (3, 2), (4, 2)]),
            match_index: HashMap::from([(0, 0), (2, 0), (3, 0), (4, 0)]),
        };

        let (requests, next) =
            leader.handle_request(&mut volitile_state, &mut persistent_state, request);

        assert_eq!(persistent_state.keep_alive, 0);
        assert!(next.is_none());
        assert!(requests.len() == 4);
        for request in requests {
            assert!(request.sender == persistent_state.id);
            assert!(request.term == persistent_state.current_term);
            match request.event {
                Event::Insert(event) => {
                    assert_eq!(event.prev_log_state.length, 2);
                    assert_eq!(event.prev_log_state.term, 3);
                    assert_eq!(event.leader_commit, 1);
                    assert!(event.entries.is_empty());
                }
                _ => {
                    panic!("Non-append event!")
                }
            }
        }
    }

    #[test]
    fn test_append_response_success() {
        let config = Config {
            servers: HashSet::from([0, 1, 2, 3, 4]),
        };
        let mut persistent_state: PersistentState<u32> = PersistentState {
            config,
            id: 1,
            current_term: 3,
            log: Vec::from([
                Entry {
                    term: 1,
                    command: 10,
                },
                Entry {
                    term: 3,
                    command: 4,
                },
            ]),
            ..Default::default()
        };
        let mut volitile_state = VolitileState { commit_index: 2 };
        let request: Request<u32> = Request {
            sender: 4,
            reciever: persistent_state.id,
            term: 3,
            event: Event::InsertResponse(request::InsertResponse { success: true }),
        };

        let mut leader = Leader {
            next_index: HashMap::from([(0, 2), (2, 2), (3, 2), (4, 1)]),
            match_index: HashMap::from([(0, 0), (2, 0), (3, 0), (4, 0)]),
        };

        let (requests, next) =
            leader.handle_request(&mut volitile_state, &mut persistent_state, request);

        assert_eq!(persistent_state.keep_alive, 0);
        assert!(next.is_none());
        assert!(requests.is_empty());
        assert_eq!(leader.next_index[&4], 2);
        assert_eq!(leader.match_index[&4], 2);
    }

    #[test]
    fn test_append_response_suceeds_up_to_date() {
        let config = Config {
            servers: HashSet::from([0, 1, 2, 3, 4]),
        };
        let mut persistent_state: PersistentState<u32> = PersistentState {
            config,
            id: 1,
            current_term: 3,
            log: Vec::from([
                Entry {
                    term: 1,
                    command: 10,
                },
                Entry {
                    term: 3,
                    command: 4,
                },
            ]),
            ..Default::default()
        };
        let mut volitile_state = VolitileState { commit_index: 1 };
        let request: Request<u32> = Request {
            sender: 0,
            reciever: persistent_state.id,
            term: 3,
            event: Event::InsertResponse(request::InsertResponse { success: true }),
        };

        let mut leader = Leader {
            next_index: HashMap::from([(0, 2), (2, 2), (3, 2), (4, 1)]),
            match_index: HashMap::from([(0, 0), (2, 0), (3, 0), (4, 0)]),
        };

        let (requests, next) =
            leader.handle_request(&mut volitile_state, &mut persistent_state, request);

        assert_eq!(persistent_state.keep_alive, 0);
        assert!(next.is_none());
        assert!(requests.is_empty());
        assert_eq!(leader.next_index[&0], 2);
        assert_eq!(leader.match_index[&0], 2);
    }

    #[test]
    fn test_append_response_fails() {
        let config = Config {
            servers: HashSet::from([0, 1, 2, 3, 4]),
        };
        let mut persistent_state: PersistentState<u32> = PersistentState {
            config,
            id: 1,
            current_term: 3,
            log: Vec::from([
                Entry {
                    term: 1,
                    command: 10,
                },
                Entry {
                    term: 3,
                    command: 4,
                },
            ]),
            ..Default::default()
        };
        let mut volitile_state = VolitileState { commit_index: 1 };
        let request: Request<u32> = Request {
            sender: 0,
            reciever: persistent_state.id,
            term: 3,
            event: Event::InsertResponse(request::InsertResponse { success: false }),
        };

        let mut leader = Leader {
            next_index: HashMap::from([(0, 2), (2, 2), (3, 2), (4, 1)]),
            match_index: HashMap::from([(0, 0), (2, 0), (3, 0), (4, 0)]),
        };

        let (requests, next) =
            leader.handle_request(&mut volitile_state, &mut persistent_state, request);

        assert_eq!(persistent_state.keep_alive, 0);
        assert!(next.is_none());
        assert!(requests.is_empty());
        assert_eq!(leader.next_index[&0], 1);
        assert_eq!(leader.match_index[&0], 0);
    }

    #[test]
    fn test_client_request() {
        let config = Config {
            servers: HashSet::from([0, 1, 2, 3, 4]),
        };
        let log = Vec::from([
            Entry {
                term: 1,
                command: 10,
            },
            Entry {
                term: 3,
                command: 4,
            },
        ]);
        let mut persistent_state: PersistentState<u32> = PersistentState {
            config,
            id: 1,
            current_term: 3,
            log: log.clone(),
            ..Default::default()
        };
        let mut volitile_state = VolitileState { commit_index: 1 };
        let request: Request<u32> = Request {
            sender: 0,
            reciever: persistent_state.id,
            term: 0,
            event: Event::Client(request::Client { data: 2 }),
        };

        let mut leader = Leader {
            next_index: HashMap::from([(0, 2), (2, 2), (3, 2), (4, 1)]),
            match_index: HashMap::from([(0, 0), (2, 0), (3, 0), (4, 0)]),
        };

        let (requests, next) =
            leader.handle_request(&mut volitile_state, &mut persistent_state, request);

        assert_eq!(persistent_state.keep_alive, 0);
        assert!(next.is_none());
        assert!(requests.is_empty());
        assert_eq!(volitile_state.commit_index, 1);
        assert_eq!(persistent_state.log.len(), 3);
        assert!(log.iter().eq(persistent_state.log[0..2].iter()));
        assert!(Entry {
            term: 3,
            command: 2
        }
        .eq(&persistent_state.log[2]));
    }
}
