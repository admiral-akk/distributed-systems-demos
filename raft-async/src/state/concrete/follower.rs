use std::time::Duration;

use crate::{
    data::{
        data_type::CommandType,
        persistent_state::PersistentState,
        request::{
            Append, AppendResponse, Client, ClientResponse, Event, Request, Timeout, Vote,
            VoteResponse,
        },
        volitile_state::VolitileState,
    },
    state::{
        handler::{EventHandler, Handler, TimeoutHandler},
        raft_state::RaftState,
    },
};

use super::candidate::Candidate;

#[derive(Default)]
pub struct Follower {}

impl TimeoutHandler for Follower {
    fn timeout_length(&self) -> Duration {
        Duration::from_millis(1000)
    }
}

impl<T: CommandType> EventHandler<AppendResponse, T> for Follower {}
impl<T: CommandType> EventHandler<Timeout, T> for Follower {
    fn handle_event(
        &mut self,
        _volitile_state: &mut VolitileState,
        persistent_state: &mut PersistentState<T>,
        _sender: u32,
        _term: u32,
        _event: Timeout,
    ) -> (Vec<Request<T>>, Option<RaftState>) {
        Candidate::call_election(persistent_state)
    }
}
impl<T: CommandType> EventHandler<VoteResponse, T> for Follower {}
impl<T: CommandType> EventHandler<Vote, T> for Follower {
    fn handle_event(
        &mut self,
        _volitile_state: &mut VolitileState,
        persistent_state: &mut PersistentState<T>,
        sender: u32,
        term: u32,
        event: Vote,
    ) -> (Vec<Request<T>>, Option<RaftState>) {
        println!("{} requested vote from {}", sender, persistent_state.id);
        let mut success = true;
        success &= persistent_state.current_term <= term;
        if let Some(voted_for) = persistent_state.voted_for {
            success &= voted_for != sender;
        }
        // Candidate log is at least as long as follower log
        success &= event.log_length >= persistent_state.log.len();
        if event.log_length == persistent_state.log.len() && event.log_length > 0 {
            // If they match length, then term of last log entry is at least as large
            success &= persistent_state.log[event.log_length - 1].term <= event.last_log_term;
        }

        if success {
            persistent_state.voted_for = Some(sender);
        }

        (
            Vec::from([Request {
                sender: persistent_state.id,
                reciever: sender,
                term: persistent_state.current_term,
                event: Event::VoteResponse(VoteResponse { success }),
            }]),
            None,
        )
    }
}

impl<T: CommandType> EventHandler<Append<T>, T> for Follower {
    fn handle_event(
        &mut self,
        volitile_state: &mut VolitileState,
        persistent_state: &mut PersistentState<T>,
        sender: u32,
        term: u32,
        event: Append<T>,
    ) -> (Vec<Request<T>>, Option<RaftState>) {
        let mut success = true;
        success &= persistent_state.current_term <= term;
        if success {
            // We have a valid leader.
            persistent_state.keep_alive += 1;
            persistent_state.voted_for = Some(sender);
        }
        // If we don't have the previous entry, then the append fails.
        success &= persistent_state.log.len() >= event.prev_log_length;
        if success && event.prev_log_length > 0 {
            // If we have a previous entry, then the term needs to match.
            success &= persistent_state.log[event.prev_log_length - 1].term == event.prev_log_term;
        }

        let entry_len = event.entries.len();
        if success {
            for (index, entry) in event.entries.into_iter().enumerate() {
                let log_index = event.prev_log_length + index;
                if persistent_state.log.len() > log_index {
                    if persistent_state.log[log_index].term != entry.term {
                        persistent_state
                            .log
                            .drain(log_index..persistent_state.log.len());
                    }
                }
                if persistent_state.log.len() > log_index {
                    persistent_state.log[log_index] = entry;
                } else {
                    persistent_state.log.push(entry);
                }
            }
        }

        if event.leader_commit > volitile_state.commit_index {
            if success {
                volitile_state.commit_index =
                    event.leader_commit.min(event.prev_log_length + entry_len);
            }
        }

        (
            Vec::from([Request {
                sender: persistent_state.id,
                reciever: sender,
                term: persistent_state.current_term,
                event: Event::AppendResponse(AppendResponse { success }),
            }]),
            None,
        )
    }
}

impl<T: CommandType> Handler<T> for Follower {}

impl<T: CommandType> EventHandler<Client<T>, T> for Follower {
    fn handle_event(
        &mut self,
        _volitile_state: &mut VolitileState,
        persistent_state: &mut PersistentState<T>,
        sender: u32,
        _term: u32,
        event: Client<T>,
    ) -> (Vec<Request<T>>, Option<RaftState>) {
        (
            Vec::from([Request {
                sender: persistent_state.id,
                reciever: sender,
                term: 0,
                event: Event::ClientResponse(ClientResponse::Failed {
                    leader_id: persistent_state.voted_for,
                    data: event.data,
                }),
            }]),
            None,
        )
    }
}
impl<T: CommandType> EventHandler<ClientResponse<T>, T> for Follower {}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use crate::data::persistent_state::{Config, Entry};
    use crate::data::request;
    use crate::state::concrete::follower::Follower;
    // Note this useful idiom: importing names from outer (for mod tests) scope.
    use super::*;

    #[test]
    fn test_timeout() {
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
                    term: 2,
                    command: 4,
                },
            ]),
            ..Default::default()
        };
        let mut volitile_state = VolitileState::default();
        let mut follower = Follower::default();
        let term = persistent_state.current_term;
        let request: Request<u32> = Request {
            sender: 10,
            reciever: persistent_state.id,
            term: 0,
            event: Event::Timeout(request::Timeout),
        };

        let (requests, next) =
            follower.handle_request(&mut volitile_state, &mut persistent_state, request);

        assert!(next.is_some());
        if let Some(RaftState::Candidate(_)) = next {
            assert!(persistent_state.current_term == term + 1);
        } else {
            panic!("Didn't transition to candidate!");
        }
        assert!(requests.len() == 2);
        assert_eq!(persistent_state.keep_alive, 1);
        for request in requests {
            assert!(request.sender == persistent_state.id);
            assert!(request.term == persistent_state.current_term);
            match request.event {
                Event::Vote(event) => {
                    assert!(event.last_log_term == 2);
                    assert!(event.log_length == 2);
                }
                _ => {
                    panic!("Non-vote event!")
                }
            }
        }
    }

    #[test]
    fn test_append_old_leader() {
        let config = Config {
            servers: HashSet::from([0, 1, 2]),
        };
        let log = Vec::from([
            Entry {
                term: 1,
                command: 10,
            },
            Entry {
                term: 2,
                command: 4,
            },
        ]);
        let mut persistent_state: PersistentState<u32> = PersistentState {
            config,
            id: 1,
            current_term: 6,
            log: log.clone(),
            ..Default::default()
        };
        let mut volitile_state = VolitileState::default();
        let mut follower = Follower::default();
        let original_request: Request<u32> = Request {
            sender: 0,
            reciever: persistent_state.id,
            term: 4,
            event: Event::Append(request::Append {
                prev_log_length: 2,
                prev_log_term: 2,
                entries: Vec::from([Entry {
                    term: 3,
                    command: 5,
                }]),
                leader_commit: 2,
            }),
        };

        let (requests, next) =
            follower.handle_request(&mut volitile_state, &mut persistent_state, original_request);

        assert!(next.is_none());
        assert!(requests.len() == 1);
        assert!(persistent_state.current_term == 6);
        assert_eq!(persistent_state.keep_alive, 0);
        assert!(
            persistent_state.voted_for == None,
            "Follower should not redirect to this leader."
        );
        for request in requests {
            assert!(request.sender == persistent_state.id);
            assert!(request.reciever == 0);
            assert!(request.term == persistent_state.current_term);
            match request.event {
                Event::AppendResponse(event) => {
                    assert!(!event.success);
                }
                _ => {
                    panic!("Non-append response!");
                }
            }
        }
        assert!(log.iter().eq(persistent_state.log.iter()));
        assert!(volitile_state.commit_index == 0);
    }

    #[test]
    fn test_append_log_too_short() {
        let config = Config {
            servers: HashSet::from([0, 1, 2]),
        };
        let log = Vec::from([
            Entry {
                term: 1,
                command: 10,
            },
            Entry {
                term: 2,
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
        let mut follower = Follower::default();
        let original_request: Request<u32> = Request {
            sender: 0,
            reciever: persistent_state.id,
            term: 4,
            event: Event::Append(request::Append {
                prev_log_length: 10,
                prev_log_term: 2,
                entries: Vec::from([Entry {
                    term: 3,
                    command: 5,
                }]),
                leader_commit: 2,
            }),
        };

        let (requests, next) =
            follower.handle_request(&mut volitile_state, &mut persistent_state, original_request);

        assert!(next.is_none());
        assert!(requests.len() == 1);
        assert!(persistent_state.current_term == 4);
        assert!(persistent_state.voted_for == Some(0));
        assert_eq!(persistent_state.keep_alive, 1);
        for request in requests {
            assert!(request.sender == persistent_state.id);
            assert!(request.reciever == 0);
            assert!(request.term == persistent_state.current_term);
            match request.event {
                Event::AppendResponse(event) => {
                    assert!(!event.success);
                }
                _ => {
                    panic!("Non-append response!");
                }
            }
        }
        assert!(log.iter().eq(persistent_state.log.iter()));
        assert!(volitile_state.commit_index == 0);
    }

    #[test]
    fn test_append_log_last_term_mismatch() {
        let config = Config {
            servers: HashSet::from([0, 1, 2]),
        };
        let log = Vec::from([
            Entry {
                term: 1,
                command: 10,
            },
            Entry {
                term: 2,
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
        let mut follower = Follower::default();
        let original_request: Request<u32> = Request {
            sender: 0,
            reciever: persistent_state.id,
            term: 4,
            event: Event::Append(request::Append {
                prev_log_length: 2,
                prev_log_term: 3,
                entries: Vec::from([Entry {
                    term: 3,
                    command: 5,
                }]),
                leader_commit: 2,
            }),
        };

        let (requests, next) =
            follower.handle_request(&mut volitile_state, &mut persistent_state, original_request);

        assert!(next.is_none());
        assert!(requests.len() == 1);
        assert!(persistent_state.current_term == 4);
        assert!(persistent_state.voted_for == Some(0));
        assert_eq!(persistent_state.keep_alive, 1);
        for request in requests {
            assert!(request.sender == persistent_state.id);
            assert!(request.reciever == 0);
            assert!(request.term == persistent_state.current_term);
            match request.event {
                Event::AppendResponse(event) => {
                    assert!(!event.success);
                }
                _ => {
                    panic!("Non-append response!");
                }
            }
        }
        assert!(log.iter().eq(persistent_state.log.iter()));
        assert!(volitile_state.commit_index == 0);
    }

    #[test]
    fn test_append_log_basic() {
        let config = Config {
            servers: HashSet::from([0, 1, 2]),
        };
        let log = Vec::from([
            Entry {
                term: 1,
                command: 10,
            },
            Entry {
                term: 2,
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
        let mut follower = Follower::default();
        let entries = Vec::from([Entry {
            term: 3,
            command: 5,
        }]);
        let original_request: Request<u32> = Request {
            sender: 0,
            reciever: persistent_state.id,
            term: 4,
            event: Event::Append(request::Append {
                prev_log_length: 2,
                prev_log_term: 2,
                entries: entries.clone(),
                leader_commit: 2,
            }),
        };

        let (requests, next) =
            follower.handle_request(&mut volitile_state, &mut persistent_state, original_request);

        assert!(next.is_none());
        assert!(requests.len() == 1);
        assert!(persistent_state.current_term == 4);
        assert!(persistent_state.voted_for == Some(0));
        assert_eq!(persistent_state.keep_alive, 1);
        for request in requests {
            assert!(request.sender == persistent_state.id);
            assert!(request.reciever == 0);
            assert!(request.term == persistent_state.current_term);
            match request.event {
                Event::AppendResponse(event) => {
                    assert!(event.success);
                }
                _ => {
                    panic!("Non-append response!");
                }
            }
        }

        assert!(log[0..2].iter().eq(persistent_state.log[0..2].iter()));
        assert!(entries[0..1].iter().eq(persistent_state.log[2..3].iter()));
        assert!(volitile_state.commit_index == 2);
    }

    #[test]
    fn test_append_log_overwrite() {
        let config = Config {
            servers: HashSet::from([0, 1, 2]),
        };
        let log = Vec::from([
            Entry {
                term: 1,
                command: 10,
            },
            Entry {
                term: 2,
                command: 4,
            },
            Entry {
                term: 3,
                command: 5,
            },
            Entry {
                term: 3,
                command: 6,
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
        let mut follower = Follower::default();
        let entries = Vec::from([Entry {
            term: 4,
            command: 5,
        }]);
        let original_request: Request<u32> = Request {
            sender: 0,
            reciever: persistent_state.id,
            term: 4,
            event: Event::Append(request::Append {
                prev_log_length: 2,
                prev_log_term: 2,
                entries: entries.clone(),
                leader_commit: 12,
            }),
        };

        let (requests, next) =
            follower.handle_request(&mut volitile_state, &mut persistent_state, original_request);

        assert!(next.is_none());
        assert!(requests.len() == 1);
        assert!(persistent_state.current_term == 4);
        assert!(persistent_state.voted_for == Some(0));
        assert_eq!(persistent_state.keep_alive, 1);
        for request in requests {
            assert!(request.sender == persistent_state.id);
            assert!(request.reciever == 0);
            assert!(request.term == persistent_state.current_term);
            match request.event {
                Event::AppendResponse(event) => {
                    assert!(event.success);
                }
                _ => {
                    panic!("Non-append response!");
                }
            }
        }

        assert!(log[0..2].iter().eq(persistent_state.log[0..2].iter()));
        assert!(entries[0..1].iter().eq(persistent_state.log[2..3].iter()));
        assert!(persistent_state.log.len() == 3);
        assert!(volitile_state.commit_index == 3);
    }

    #[test]
    fn test_vote_old_term() {
        let config = Config {
            servers: HashSet::from([0, 1, 2]),
        };
        let log = Vec::from([
            Entry {
                term: 1,
                command: 10,
            },
            Entry {
                term: 2,
                command: 4,
            },
            Entry {
                term: 3,
                command: 5,
            },
            Entry {
                term: 3,
                command: 6,
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
        let mut follower = Follower::default();
        let original_request: Request<u32> = Request {
            sender: 2,
            reciever: persistent_state.id,
            term: 3,
            event: Event::Vote(request::Vote {
                log_length: 5,
                last_log_term: 3,
            }),
        };

        let (requests, next) =
            follower.handle_request(&mut volitile_state, &mut persistent_state, original_request);

        assert!(next.is_none());
        assert!(requests.len() == 1);
        assert!(persistent_state.current_term == 4);
        assert!(persistent_state.voted_for == None);
        assert_eq!(persistent_state.keep_alive, 0);
        for request in requests {
            assert!(request.sender == persistent_state.id);
            assert!(request.reciever == 2);
            assert!(request.term == persistent_state.current_term);
            match request.event {
                Event::VoteResponse(event) => {
                    assert!(!event.success);
                }
                _ => {
                    panic!("Non-vote response!");
                }
            }
        }
        assert!(persistent_state.log.iter().eq(log.iter()));
    }

    #[test]
    fn test_vote_shorter_log_larger_last_term() {
        let config = Config {
            servers: HashSet::from([0, 1, 2]),
        };
        let log = Vec::from([
            Entry {
                term: 1,
                command: 10,
            },
            Entry {
                term: 2,
                command: 4,
            },
            Entry {
                term: 3,
                command: 5,
            },
            Entry {
                term: 3,
                command: 6,
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
        let mut follower = Follower::default();
        let original_request: Request<u32> = Request {
            sender: 2,
            reciever: persistent_state.id,
            term: 4,
            event: Event::Vote(request::Vote {
                log_length: 3,
                last_log_term: 4,
            }),
        };

        let (requests, next) =
            follower.handle_request(&mut volitile_state, &mut persistent_state, original_request);

        assert!(next.is_none());
        assert!(requests.len() == 1);
        assert!(persistent_state.current_term == 4);
        assert!(persistent_state.voted_for == None);
        assert_eq!(persistent_state.keep_alive, 0);
        for request in requests {
            assert!(request.sender == persistent_state.id);
            assert!(request.reciever == 2);
            assert!(request.term == persistent_state.current_term);
            match request.event {
                Event::VoteResponse(event) => {
                    assert!(!event.success);
                }
                _ => {
                    panic!("Non-vote response!");
                }
            }
        }
        assert!(persistent_state.log.iter().eq(log.iter()));
    }

    #[test]
    fn test_vote_same_log_length_older_term() {
        let config = Config {
            servers: HashSet::from([0, 1, 2]),
        };
        let log = Vec::from([
            Entry {
                term: 1,
                command: 10,
            },
            Entry {
                term: 2,
                command: 4,
            },
            Entry {
                term: 3,
                command: 5,
            },
            Entry {
                term: 3,
                command: 6,
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
        let mut follower = Follower::default();
        let original_request: Request<u32> = Request {
            sender: 2,
            reciever: persistent_state.id,
            term: 4,
            event: Event::Vote(request::Vote {
                log_length: 4,
                last_log_term: 2,
            }),
        };

        let (requests, next) =
            follower.handle_request(&mut volitile_state, &mut persistent_state, original_request);

        assert!(next.is_none());
        assert!(requests.len() == 1);
        assert!(persistent_state.current_term == 4);
        assert!(persistent_state.voted_for == None);
        assert_eq!(persistent_state.keep_alive, 0);
        for request in requests {
            assert!(request.sender == persistent_state.id);
            assert!(request.reciever == 2);
            assert!(request.term == persistent_state.current_term);
            match request.event {
                Event::VoteResponse(event) => {
                    assert!(!event.success);
                }
                _ => {
                    panic!("Non-vote response!");
                }
            }
        }
        assert!(persistent_state.log.iter().eq(log.iter()));
    }

    #[test]
    fn test_vote_same_log_length_same_term() {
        let config = Config {
            servers: HashSet::from([0, 1, 2]),
        };
        let log = Vec::from([
            Entry {
                term: 1,
                command: 10,
            },
            Entry {
                term: 2,
                command: 4,
            },
            Entry {
                term: 3,
                command: 5,
            },
            Entry {
                term: 3,
                command: 6,
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
        let mut follower = Follower::default();
        let original_request: Request<u32> = Request {
            sender: 2,
            reciever: persistent_state.id,
            term: 4,
            event: Event::Vote(request::Vote {
                log_length: 4,
                last_log_term: 3,
            }),
        };

        let (requests, next) =
            follower.handle_request(&mut volitile_state, &mut persistent_state, original_request);

        assert!(next.is_none());
        assert!(requests.len() == 1);
        assert!(persistent_state.current_term == 4);
        assert!(persistent_state.voted_for == Some(2));
        assert_eq!(persistent_state.keep_alive, 0);
        for request in requests {
            assert!(request.sender == persistent_state.id);
            assert!(request.reciever == 2);
            assert!(request.term == persistent_state.current_term);
            match request.event {
                Event::VoteResponse(event) => {
                    assert!(event.success);
                }
                _ => {
                    panic!("Non-vote response!");
                }
            }
        }
        assert!(persistent_state.log.iter().eq(log.iter()));
    }

    #[test]
    fn test_vote_longer_log_length_older_term() {
        let config = Config {
            servers: HashSet::from([0, 1, 2]),
        };
        let log = Vec::from([
            Entry {
                term: 1,
                command: 10,
            },
            Entry {
                term: 2,
                command: 4,
            },
            Entry {
                term: 3,
                command: 5,
            },
            Entry {
                term: 3,
                command: 6,
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
        let mut follower = Follower::default();
        let original_request: Request<u32> = Request {
            sender: 2,
            reciever: persistent_state.id,
            term: 4,
            event: Event::Vote(request::Vote {
                log_length: 5,
                last_log_term: 2,
            }),
        };

        let (requests, next) =
            follower.handle_request(&mut volitile_state, &mut persistent_state, original_request);

        assert!(next.is_none());
        assert!(requests.len() == 1);
        assert!(persistent_state.current_term == 4);
        assert!(persistent_state.voted_for == Some(2));
        assert_eq!(persistent_state.keep_alive, 0);
        for request in requests {
            assert!(request.sender == persistent_state.id);
            assert!(request.reciever == 2);
            assert!(request.term == persistent_state.current_term);
            match request.event {
                Event::VoteResponse(event) => {
                    assert!(event.success);
                }
                _ => {
                    panic!("Non-vote response!");
                }
            }
        }
        assert!(persistent_state.log.iter().eq(log.iter()));
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
            voted_for: Some(0),
            ..Default::default()
        };
        let mut volitile_state = VolitileState { commit_index: 1 };
        let request: Request<u32> = Request {
            sender: 10,
            reciever: persistent_state.id,
            term: 0,
            event: Event::Client(request::Client { data: 2 }),
        };

        let mut follower = Follower {};

        let (requests, next) =
            follower.handle_request(&mut volitile_state, &mut persistent_state, request);

        assert!(next.is_none());
        assert_eq!(requests.len(), 1);
        assert_eq!(persistent_state.keep_alive, 0);
        for request in requests {
            assert_eq!(request.sender, persistent_state.id);
            assert_eq!(request.reciever, 10);
            assert_eq!(request.term, 0);
            match request.event {
                Event::ClientResponse(ClientResponse::Failed {
                    leader_id: Some(leader_id),
                    data,
                }) => {
                    assert_eq!(data, 2);
                    assert_eq!(leader_id, 0);
                }
                _ => {
                    panic!("Invalid client response!");
                }
            }
        }
    }
    #[test]
    fn test_client_request_no_leader() {
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
            voted_for: None,
            ..Default::default()
        };
        let mut volitile_state = VolitileState { commit_index: 1 };
        let request: Request<u32> = Request {
            sender: 10,
            reciever: persistent_state.id,
            term: 0,
            event: Event::Client(request::Client { data: 2 }),
        };

        let mut follower = Follower {};

        let (requests, next) =
            follower.handle_request(&mut volitile_state, &mut persistent_state, request);

        assert!(next.is_none());
        assert_eq!(requests.len(), 1);
        assert_eq!(persistent_state.keep_alive, 0);
        for request in requests {
            assert_eq!(request.sender, persistent_state.id);
            assert_eq!(request.reciever, 10);
            assert_eq!(request.term, 0);
            match request.event {
                Event::ClientResponse(ClientResponse::Failed {
                    leader_id: None,
                    data,
                }) => {
                    assert_eq!(data, 2);
                }
                _ => {
                    panic!("Invalid client response!");
                }
            }
        }
    }
}
