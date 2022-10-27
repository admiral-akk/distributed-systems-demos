use std::{collections::HashSet};

use super::{follower::Follower, leader::Leader};
use crate::data::{
    data_type::CommandType,
    persistent_state::PersistentState,
    request::{
        Event, Request, Tick, Vote,
        VoteResponse,
    },
    volitile_state::VolitileState,
};
use crate::state::{
    handler::{EventHandler, Handler},
    raft_state::RaftState,
};

#[derive(Default)]
pub struct Candidate {
    votes: HashSet<u32>,
}
const TICK_TILL_NEW_ELECTION: u32 = 10;

impl Handler for Candidate {}
impl EventHandler for Candidate {
    fn handle<T: CommandType>(
        mut self,
        volitile_state: &mut VolitileState,
        persistent_state: &mut PersistentState<T>,
        sender: u32,
        term: u32,
        request: Request<T>,
    ) -> (Vec<Request<T>>, RaftState) {
        match request.event {
            Event::Insert(_) => {
                if term >= persistent_state.current_term {
                    volitile_state.tick_since_start = 0;
                    persistent_state.voted_for = Some(sender);
                    (Vec::new(), Follower::default().into())
                } else {
                    (Vec::default(), self.into())
                }
            }
            Event::Tick(Tick) => {
                if volitile_state.tick_since_start < TICK_TILL_NEW_ELECTION {
                    (Candidate::request_votes(persistent_state), self.into())
                } else {
                    Candidate::call_election(volitile_state, persistent_state)
                }
            }
            Event::VoteResponse(VoteResponse { success }) => {
                if success {
                    println!("{} voted for {}", sender, persistent_state.id);
                    self.votes.insert(sender);
                }
                if self.votes.len() + 1 >= persistent_state.quorum() {
                    return Leader::from_candidate(&self, volitile_state, persistent_state);
                }
                (Vec::default(), self.into())
            }
            _ => (Vec::default(), self.into()),
        }
    }
}

impl Candidate {
    fn request_votes<T: CommandType>(persistent_state: &mut PersistentState<T>) -> Vec<Request<T>> {
        persistent_state
            .other_servers()
            .iter()
            .map(|id| Request::<T> {
                sender: persistent_state.id,
                reciever: *id,
                term: persistent_state.current_term,
                event: Event::Vote(Vote {
                    log_state: persistent_state.log_state(),
                }),
            })
            .collect()
    }
    pub fn call_election<T: CommandType>(
        volitile_state: &mut VolitileState,
        persistent_state: &mut PersistentState<T>,
    ) -> (Vec<Request<T>>, RaftState) {
        println!("{} running for office!", persistent_state.id);
        persistent_state.current_term += 1;
        persistent_state.voted_for = Some(persistent_state.id);
        volitile_state.tick_since_start = 0;
        (
            Candidate::request_votes(persistent_state),
            Candidate::default().into(),
        )
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
    fn test_tick() {
        let config = Config {
            servers: HashSet::from([0, 1, 2]),
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
        let mut volitile_state = VolitileState::default();
        let candidate = Candidate {
            ..Default::default()
        };
        let request: Request<u32> = Request {
            sender: 0,
            reciever: persistent_state.id,
            term: 0,
            event: Event::Tick(request::Tick),
        };

        let (requests, next) =
            candidate.handle_request(&mut volitile_state, &mut persistent_state, request);

        if let RaftState::Candidate(_) = next {
        } else {
            panic!("Didn't transition to candidate!");
        }
        assert!(requests.len() == 2);
        assert_eq!(volitile_state.tick_since_start, 1);
        for request in requests {
            assert!(request.sender == persistent_state.id);
            assert!(request.term == persistent_state.current_term);
            match request.event {
                Event::Vote(event) => {
                    assert!(event.log_state.term == 3);
                    assert!(event.log_state.length == 2);
                }
                _ => {
                    panic!("Non-vote event!")
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
        let mut volitile_state = VolitileState {
            commit_index: 0,
            tick_since_start: 100000,
        };
        let candidate = Candidate {
            ..Default::default()
        };
        let request: Request<u32> = Request {
            sender: 0,
            reciever: persistent_state.id,
            term: 0,
            event: Event::Tick(request::Tick),
        };

        let (requests, next) =
            candidate.handle_request(&mut volitile_state, &mut persistent_state, request);

        assert_eq!(volitile_state.tick_since_start, 0);
        if let RaftState::Candidate(Candidate { votes }) = next {
            assert!(votes.is_empty());
            assert!(persistent_state.current_term == 4);
        } else {
            panic!("Transitioned to non-candidate!");
        }

        assert!(requests.len() == 4);
        for request in requests {
            assert!(request.sender == persistent_state.id);
            assert!(request.term == persistent_state.current_term);
            match request.event {
                Event::Vote(event) => {
                    assert!(event.log_state.term == 3);
                    assert!(event.log_state.length == 2);
                }
                _ => {
                    panic!("Non-vote event!")
                }
            }
        }
    }

    #[test]
    fn test_request_vote_rejection() {
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
        let mut volitile_state = VolitileState::default();
        let candidate = Candidate {
            ..Default::default()
        };
        let request: Request<u32> = Request {
            sender: 0,
            reciever: persistent_state.id,
            term: 0,
            event: Event::VoteResponse(request::VoteResponse { success: false }),
        };

        let (requests, next) =
            candidate.handle_request(&mut volitile_state, &mut persistent_state, request);

        if let RaftState::Candidate(candidate) = next {
            assert!(candidate.votes.len() == 0);
        } else {
            panic!("Didn't transition to candidate!");
        }

        assert!(requests.len() == 0);
    }

    #[test]
    fn test_request_vote_successful() {
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
        let mut volitile_state = VolitileState::default();
        let candidate = Candidate {
            ..Default::default()
        };
        let request: Request<u32> = Request {
            sender: 0,
            reciever: persistent_state.id,
            term: 0,
            event: Event::VoteResponse(request::VoteResponse { success: true }),
        };

        let (requests, next) =
            candidate.handle_request(&mut volitile_state, &mut persistent_state, request);

        if let RaftState::Candidate(candidate) = next {
            assert!(candidate.votes.eq(&HashSet::from([0])));
        } else {
            panic!("Didn't transition to candidate!");
        }

        assert!(requests.len() == 0);
    }

    #[test]
    fn test_request_vote_successful_redundant() {
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
        let mut volitile_state = VolitileState::default();
        let candidate = Candidate {
            votes: HashSet::from([0]),
        };
        let request: Request<u32> = Request {
            sender: 0,
            reciever: persistent_state.id,
            term: 0,
            event: Event::VoteResponse(request::VoteResponse { success: true }),
        };

        let (requests, next) =
            candidate.handle_request(&mut volitile_state, &mut persistent_state, request);

        if let RaftState::Candidate(candidate) = next {
            assert!(candidate.votes.eq(&HashSet::from([0])));
        } else {
            panic!("Didn't transition to candidate!");
        }

        assert!(requests.len() == 0);
    }

    #[test]
    fn test_request_vote_successful_elected() {
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
        let mut volitile_state = VolitileState::default();
        let candidate = Candidate {
            votes: HashSet::from([0]),
        };
        let request: Request<u32> = Request {
            sender: 2,
            reciever: persistent_state.id,
            term: 0,
            event: Event::VoteResponse(request::VoteResponse { success: true }),
        };

        let (_, next) =
            candidate.handle_request(&mut volitile_state, &mut persistent_state, request);

        // We can verify the elected Leader values in the leader tests.
        if let RaftState::Leader(_) = next {
        } else {
            panic!("Didn't become a leader!");
        }
        assert_eq!(volitile_state.tick_since_start, 0);
    }

    #[test]
    fn test_append_old_leader() {
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
            current_term: 4,
            log: log.clone(),
            ..Default::default()
        };
        let mut volitile_state = VolitileState::default();
        let candidate = Candidate {
            votes: HashSet::from([0]),
        };
        let entries = Vec::from([Entry {
            term: 4,
            command: 5,
        }]);
        let request: Request<u32> = Request {
            sender: 0,
            reciever: persistent_state.id,
            term: 3,
            event: Event::Insert(request::Insert {
                prev_log_state: persistent_state.log_state(),
                entries: entries.clone(),
                leader_commit: 12,
            }),
        };

        let (_, next) =
            candidate.handle_request(&mut volitile_state, &mut persistent_state, request);
        if let RaftState::Candidate(_) = next {
        } else {
            panic!("Failed to transition to follower");
        }
        assert!(persistent_state.log.iter().eq(log.iter()));
    }

    #[test]
    fn test_append_current_leader() {
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
            current_term: 4,
            log: log.clone(),
            ..Default::default()
        };
        let mut volitile_state = VolitileState::default();
        let candidate = Candidate {
            votes: HashSet::from([0]),
        };
        let entries = Vec::from([Entry {
            term: 4,
            command: 5,
        }]);
        let request: Request<u32> = Request {
            sender: 0,
            reciever: persistent_state.id,
            term: 4,
            event: Event::Insert(request::Insert {
                prev_log_state: persistent_state.log_state(),
                entries: entries.clone(),
                leader_commit: 12,
            }),
        };

        let (_, next) =
            candidate.handle_request(&mut volitile_state, &mut persistent_state, request);

        if let RaftState::Follower(_) = next {
        } else {
            panic!("Failed to transition to follower");
        }
        assert_eq!(volitile_state.tick_since_start, 0);
        assert!(persistent_state.voted_for == Some(0));
        assert!(persistent_state.log[0..2].iter().eq(log[0..2].iter()));
    }
}
