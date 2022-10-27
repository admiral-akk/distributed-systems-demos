use std::{collections::HashSet, time::Duration};

use super::{follower::Follower, leader::Leader};
use crate::data::{
    data_type::CommandType,
    persistent_state::PersistentState,
    request::{
        Append, AppendResponse, Client, ClientResponse, Event, Request, Timeout, Vote, VoteResponse,
    },
    volitile_state::VolitileState,
};
use crate::state::{
    handler::{EventHandler, Handler, TimeoutHandler},
    raft_state::RaftState,
};

#[derive(Default)]
pub struct Candidate {
    attempts: u32,
    votes: HashSet<u32>,
}
impl TimeoutHandler for Candidate {
    fn timeout_length(&self) -> Duration {
        Duration::from_millis(200)
    }
}

impl<T: CommandType> Handler<T> for Candidate {}
impl<T: CommandType> EventHandler<Vote, T> for Candidate {}
impl<T: CommandType> EventHandler<Client<T>, T> for Candidate {}
impl<T: CommandType> EventHandler<ClientResponse<T>, T> for Candidate {}
impl<T: CommandType> EventHandler<AppendResponse, T> for Candidate {}
impl<T: CommandType> EventHandler<Timeout, T> for Candidate {
    fn handle_event(
        &mut self,
        _volitile_state: &mut VolitileState,
        persistent_state: &mut PersistentState<T>,
        _sender: u32,
        _term: u32,
        _event: Timeout,
    ) -> (Vec<Request<T>>, Option<RaftState>) {
        self.attempts += 1;
        if self.attempts > 10 {
            Candidate::call_election(persistent_state)
        } else {
            (Candidate::request_votes(persistent_state), None)
        }
    }
}

impl<T: CommandType> EventHandler<Append<T>, T> for Candidate {
    fn handle_event(
        &mut self,
        _volitile_state: &mut VolitileState,
        persistent_state: &mut PersistentState<T>,
        sender: u32,
        term: u32,
        _event: Append<T>,
    ) -> (Vec<Request<T>>, Option<RaftState>) {
        if term >= persistent_state.current_term {
            persistent_state.keep_alive += 1;
            persistent_state.voted_for = Some(sender);
            return (Vec::new(), Some(Follower::default().into()));
        }
        (Vec::default(), None)
    }
}
impl<T: CommandType> EventHandler<VoteResponse, T> for Candidate {
    fn handle_event(
        &mut self,
        volitile_state: &mut VolitileState,
        persistent_state: &mut PersistentState<T>,
        sender: u32,
        _term: u32,
        event: VoteResponse,
    ) -> (Vec<Request<T>>, Option<RaftState>) {
        if event.success {
            println!("{} voted for {}", sender, persistent_state.id);
            self.votes.insert(sender);
        }
        if self.votes.len() + 1 >= persistent_state.quorum() {
            return Leader::from_candidate(&self, volitile_state, persistent_state);
        }
        (Vec::default(), None)
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
        persistent_state: &mut PersistentState<T>,
    ) -> (Vec<Request<T>>, Option<RaftState>) {
        println!("{} running for office!", persistent_state.id);
        persistent_state.current_term += 1;
        persistent_state.voted_for = Some(persistent_state.id);
        persistent_state.keep_alive += 1;
        (
            Candidate::request_votes(persistent_state),
            Some(Candidate::default().into()),
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
    fn test_timeout_few_iterations() {
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
        let mut candidate = Candidate {
            attempts: 0,
            ..Default::default()
        };
        let request: Request<u32> = Request {
            sender: 0,
            reciever: persistent_state.id,
            term: 0,
            event: Event::Timeout(request::Timeout),
        };

        let (requests, next) =
            candidate.handle_request(&mut volitile_state, &mut persistent_state, request);

        assert!(next.is_none());
        assert!(requests.len() == 2);
        assert_eq!(persistent_state.keep_alive, 0);
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
        assert!(candidate.attempts == 1);
    }

    #[test]
    fn test_timeout_many_iterations() {
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
        let mut candidate = Candidate {
            attempts: 10000,
            ..Default::default()
        };
        let request: Request<u32> = Request {
            sender: 0,
            reciever: persistent_state.id,
            term: 0,
            event: Event::Timeout(request::Timeout),
        };

        let (requests, next) =
            candidate.handle_request(&mut volitile_state, &mut persistent_state, request);

        assert!(next.is_some());
        assert_eq!(persistent_state.keep_alive, 1);
        if let Some(RaftState::Candidate(Candidate { attempts, votes })) = next {
            assert!(attempts == 0);
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
        let mut candidate = Candidate {
            attempts: 0,
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

        assert!(next.is_none());

        assert_eq!(persistent_state.keep_alive, 0);
        assert!(requests.len() == 0);
        assert!(candidate.attempts == 0);
        assert!(candidate.votes.len() == 0);
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
        let mut candidate = Candidate {
            attempts: 0,
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

        assert!(next.is_none());

        assert_eq!(persistent_state.keep_alive, 0);
        assert!(requests.len() == 0);
        assert!(candidate.attempts == 0);
        assert!(candidate.votes.eq(&HashSet::from([0])));
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
        let mut candidate = Candidate {
            attempts: 0,
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

        assert!(next.is_none());

        assert_eq!(persistent_state.keep_alive, 0);
        assert!(requests.len() == 0);
        assert!(candidate.attempts == 0);
        assert!(candidate.votes.eq(&HashSet::from([0])));
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
        let mut candidate = Candidate {
            attempts: 0,
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

        assert!(next.is_some());
        // We can verify the elected Leader values in the leader tests.
        if let Some(RaftState::Leader(Leader { .. })) = next {
        } else {
            panic!("Didn't become a leader!");
        }
        assert_eq!(persistent_state.keep_alive, 1);
        assert!(candidate.attempts == 0);
        assert!(candidate.votes.eq(&HashSet::from([0, 2])));
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
        let mut candidate = Candidate {
            attempts: 0,
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
            event: Event::Append(request::Append {
                prev_log_state: persistent_state.log_state(),
                entries: entries.clone(),
                leader_commit: 12,
            }),
        };

        let (_, next) =
            candidate.handle_request(&mut volitile_state, &mut persistent_state, request);
        assert_eq!(persistent_state.keep_alive, 0);
        assert!(next.is_none());
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
        let mut candidate = Candidate {
            attempts: 0,
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
            event: Event::Append(request::Append {
                prev_log_state: persistent_state.log_state(),
                entries: entries.clone(),
                leader_commit: 12,
            }),
        };

        let (_, next) =
            candidate.handle_request(&mut volitile_state, &mut persistent_state, request);

        assert!(next.is_some());
        if let Some(RaftState::Follower(_)) = next {
        } else {
            panic!("Failed to transition to follower");
        }
        assert_eq!(persistent_state.keep_alive, 1);
        assert!(persistent_state.voted_for == Some(0));
        assert!(persistent_state.log[0..2].iter().eq(log[0..2].iter()));
    }
}
