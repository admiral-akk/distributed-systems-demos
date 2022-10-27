use std::{collections::HashMap, fmt::Display};

use crate::{
    data::data_type::OutputType,
    state::{
        handler::{EventHandler, Handler},
        raft_state::RaftState,
    },
};
use crate::{
    data::{
        data_type::CommandType,
        persistent_state::PersistentState,
        request::{Client, Event, InsertResponse, Request, Tick},
        volitile_state::VolitileState,
    },
    state::state::StateMachine,
};

use super::candidate::Candidate;

pub struct Leader {
    pub next_index: HashMap<u32, usize>,
    pub match_index: HashMap<u32, usize>,
}

impl Leader {
    pub fn send_heartbeat<T: CommandType, Output>(
        &self,
        volitile_state: &mut VolitileState,
        persistent_state: &mut PersistentState<T>,
    ) -> Vec<Request<T, Output>> {
        persistent_state
            .other_servers()
            .iter()
            .map(|server| self.append_update(volitile_state, persistent_state, *server))
            .collect()
    }

    fn append_update<T: CommandType, Output>(
        &self,
        volitile_state: &mut VolitileState,
        persistent_state: &mut PersistentState<T>,
        server: u32,
    ) -> Request<T, Output> {
        Request {
            sender: persistent_state.id,
            reciever: server,
            term: persistent_state.current_term,
            event: persistent_state.insert(
                self.next_index[&server],
                1,
                volitile_state.get_commit_index(),
            ),
        }
    }

    pub fn from_candidate<T: CommandType, Output>(
        _candidate: &Candidate,
        volitile_state: &mut VolitileState,
        persistent_state: &mut PersistentState<T>,
    ) -> (Vec<Request<T, Output>>, RaftState) {
        persistent_state.current_term += 1;
        volitile_state.tick_since_start = 0;
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
        (heartbeat, leader.into())
    }
}

impl Handler for Leader {}
impl EventHandler for Leader {
    fn handle<T: CommandType, Output: OutputType, SM>(
        mut self,
        volitile_state: &mut VolitileState,
        persistent_state: &mut PersistentState<T>,
        state_machine: &mut SM,
        sender: u32,
        _term: u32,
        request: Request<T, Output>,
    ) -> (Vec<Request<T, Output>>, RaftState)
    where
        SM: StateMachine<T, Output>,
    {
        match request.event {
            Event::Client(Client { data }) => {
                persistent_state.push(data);
                (Vec::new(), self.into())
            }
            Event::Tick(Tick) => (
                self.send_heartbeat(volitile_state, persistent_state),
                self.into(),
            ),
            Event::InsertResponse(InsertResponse { success }) => {
                let next_index = self.next_index[&sender];
                if success {
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

                if matching_servers + 1 > persistent_state.quorum() {
                    if volitile_state.try_update_commit_index(
                        state_machine,
                        persistent_state,
                        next_index,
                    ) {
                        println!(
                            "Leader {} index committed, value: {:?}!",
                            next_index,
                            state_machine.get()
                        );
                    }
                }
                (Vec::default(), self.into())
            }
            _ => (Vec::default(), self.into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use crate::data::persistent_state::{Config, Entry};
    use crate::data::request::{self, Event};
    use crate::Sum;
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
        let mut volitile_state = VolitileState {
            commit_index: 1,
            tick_since_start: 10,
        };

        let (requests, next): (Vec<Request<_, u32>>, _) = Leader::from_candidate(
            &Candidate::default(),
            &mut volitile_state,
            &mut persistent_state,
        );

        if let RaftState::Leader(Leader {
            next_index,
            match_index,
        }) = next
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
        assert_eq!(volitile_state.tick_since_start, 0);
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
    fn test_tick() {
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
        let mut volitile_state = VolitileState {
            commit_index: 1,
            tick_since_start: 0,
        };
        let request: Request<u32, u32> = Request {
            sender: 0,
            reciever: persistent_state.id,
            term: 0,
            event: Event::Tick(request::Tick),
        };
        let leader = Leader {
            next_index: HashMap::from([(0, 2), (2, 2), (3, 2), (4, 2)]),
            match_index: HashMap::from([(0, 0), (2, 0), (3, 0), (4, 0)]),
        };

        let mut state_machine = Sum::default();

        let (requests, next) = leader.handle_request(
            &mut volitile_state,
            &mut persistent_state,
            &mut state_machine,
            request,
        );

        assert_eq!(volitile_state.tick_since_start, 1);
        if let RaftState::Leader(_) = next {
        } else {
            panic!("Didn't transition to leader!");
        }
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
        let mut volitile_state = VolitileState {
            commit_index: 2,
            ..Default::default()
        };
        let request: Request<u32, u32> = Request {
            sender: 4,
            reciever: persistent_state.id,
            term: 3,
            event: Event::InsertResponse(request::InsertResponse { success: true }),
        };

        let leader = Leader {
            next_index: HashMap::from([(0, 2), (2, 2), (3, 2), (4, 1)]),
            match_index: HashMap::from([(0, 0), (2, 0), (3, 0), (4, 0)]),
        };

        let mut state_machine = Sum::default();

        let (requests, next) = leader.handle_request(
            &mut volitile_state,
            &mut persistent_state,
            &mut state_machine,
            request,
        );

        if let RaftState::Leader(leader) = next {
            assert_eq!(leader.next_index[&4], 2);
            assert_eq!(leader.match_index[&4], 2);
        } else {
            panic!("Didn't transition to leader!");
        }
        assert!(requests.is_empty());
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
        let mut volitile_state = VolitileState {
            commit_index: 1,
            ..Default::default()
        };
        let request: Request<u32, u32> = Request {
            sender: 0,
            reciever: persistent_state.id,
            term: 3,
            event: Event::InsertResponse(request::InsertResponse { success: true }),
        };

        let leader = Leader {
            next_index: HashMap::from([(0, 2), (2, 2), (3, 2), (4, 1)]),
            match_index: HashMap::from([(0, 0), (2, 0), (3, 0), (4, 0)]),
        };

        let mut state_machine = Sum::default();

        let (requests, next) = leader.handle_request(
            &mut volitile_state,
            &mut persistent_state,
            &mut state_machine,
            request,
        );

        if let RaftState::Leader(leader) = next {
            assert_eq!(leader.next_index[&0], 2);
            assert_eq!(leader.match_index[&0], 2);
        } else {
            panic!("Didn't transition to leader!");
        }
        assert!(requests.is_empty());
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
        let mut volitile_state = VolitileState {
            commit_index: 1,
            ..Default::default()
        };
        let request: Request<u32, u32> = Request {
            sender: 0,
            reciever: persistent_state.id,
            term: 3,
            event: Event::InsertResponse(request::InsertResponse { success: false }),
        };

        let leader = Leader {
            next_index: HashMap::from([(0, 2), (2, 2), (3, 2), (4, 1)]),
            match_index: HashMap::from([(0, 0), (2, 0), (3, 0), (4, 0)]),
        };

        let mut state_machine = Sum::default();

        let (requests, next) = leader.handle_request(
            &mut volitile_state,
            &mut persistent_state,
            &mut state_machine,
            request,
        );

        if let RaftState::Leader(leader) = next {
            assert_eq!(leader.next_index[&0], 1);
            assert_eq!(leader.match_index[&0], 0);
        } else {
            panic!("Didn't transition to leader!");
        }
        assert!(requests.is_empty());
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
        let mut volitile_state = VolitileState {
            commit_index: 1,
            ..Default::default()
        };
        let request: Request<u32, u32> = Request {
            sender: 0,
            reciever: persistent_state.id,
            term: 0,
            event: Event::Client(request::Client { data: 2 }),
        };

        let leader = Leader {
            next_index: HashMap::from([(0, 2), (2, 2), (3, 2), (4, 1)]),
            match_index: HashMap::from([(0, 0), (2, 0), (3, 0), (4, 0)]),
        };

        let mut state_machine = Sum::default();

        let (requests, next) = leader.handle_request(
            &mut volitile_state,
            &mut persistent_state,
            &mut state_machine,
            request,
        );

        if let RaftState::Leader(_) = next {
        } else {
            panic!("Didn't transition to leader!");
        }
        assert!(requests.is_empty());
        assert_eq!(volitile_state.get_commit_index(), 1);
        assert_eq!(persistent_state.log.len(), 3);
        assert!(log.iter().eq(persistent_state.log[0..2].iter()));
        assert!(Entry {
            term: 3,
            command: 2
        }
        .eq(&persistent_state.log[2]));
    }
}
