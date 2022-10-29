use crate::{
    data::{
        data_type::CommandType,
        persistent_state::PersistentState,
        request::{ClientResponse, Event, InsertResponse, Request, Tick, VoteResponse},
        volitile_state::VolitileState,
    },
    state::{
        handler::{EventHandler, Handler},
        raft_state::RaftState,
        state::StateMachine,
    },
};

use super::candidate::Candidate;

#[derive(Default, Debug, PartialEq, Clone)]
pub struct Follower;
const TICK_TILL_ELECTION: u32 = 25;
impl Handler for Follower {}
impl EventHandler for Follower {
    fn handle<T: CommandType, Output, SM>(
        self,
        volitile_state: &mut VolitileState,
        persistent_state: &mut PersistentState<T>,
        state_machine: &mut SM,
        sender: u32,
        term: u32,
        request: Request<T, Output>,
    ) -> (Vec<Request<T, Output>>, RaftState)
    where
        SM: StateMachine<T, Output>,
    {
        match request.event {
            Event::Vote(event) => {
                println!("{} requested vote from {}", sender, persistent_state.id);
                let mut success = persistent_state.current_term <= term;
                if success {
                    success &= persistent_state.try_vote_for(event, sender);
                }
                (
                    Vec::from([Request {
                        sender: persistent_state.id,
                        reciever: sender,
                        term: persistent_state.current_term,
                        event: Event::VoteResponse(VoteResponse { success }),
                    }]),
                    self.into(),
                )
            }
            Event::Tick(Tick) => {
                if volitile_state.tick_since_start < TICK_TILL_ELECTION {
                    (Vec::new(), self.into())
                } else {
                    Candidate::call_election(volitile_state, persistent_state)
                }
            }
            Event::Insert(event) => {
                let mut success = persistent_state.current_term <= term;
                if success {
                    // We have a valid leader.
                    volitile_state.tick_since_start = 0;
                    persistent_state.voted_for = Some(sender);
                }
                let max_commit_index = event.max_commit_index();
                if success {
                    success = persistent_state.try_insert(event);
                }

                if success {
                    volitile_state.try_update_commit_index(
                        state_machine,
                        persistent_state,
                        max_commit_index,
                    );
                }

                (
                    Vec::from([Request {
                        sender: persistent_state.id,
                        reciever: sender,
                        term: persistent_state.current_term,
                        event: Event::InsertResponse(InsertResponse { success }),
                    }]),
                    self.into(),
                )
            }
            Event::Client(event) => (
                Vec::from([Request {
                    sender: persistent_state.id,
                    reciever: sender,
                    term: 0,
                    event: Event::ClientResponse(ClientResponse::Failed {
                        leader_id: persistent_state.voted_for,
                        data: event.data,
                    }),
                }]),
                self.into(),
            ),
            _ => (Vec::default(), self.into()),
        }
    }
}
#[cfg(test)]
pub mod test_util {
    use crate::state::raft_state::RaftState;

    use super::Follower;

    pub const FOLLOWER: RaftState = RaftState::Follower(Follower);
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;
    use std::collections::HashSet;

    use crate::data::persistent_state::test_util::{
        LOG_LEADER, MISMATCH_LOG, PERSISTENT_STATE, PERSISTENT_STATE_LOG,
    };
    use crate::data::persistent_state::{Config, Entry, LogState};
    use crate::data::request::test_util::{
        INSERT, INSERT_FAILED_RESPONSE, INSERT_SUCCESS_RESPONSE, TICK,
    };
    use crate::data::request::{self, Data};
    use crate::data::volitile_state::test_util::{VOLITILE_STATE, VOLITILE_STATE_TIMEOUT};
    use crate::state::concrete::follower::test_util::FOLLOWER;
    use crate::state::concrete::follower::Follower;
    use crate::test_util::STATE_MACHINE;
    use crate::Sum;
    // Note this useful idiom: importing names from outer (for mod tests) scope.
    use super::*;

    #[test]
    fn test_tick() {
        let (mut state, mut volitile_state, mut persistent_state, mut state_machine, request) = (
            FOLLOWER,
            VOLITILE_STATE,
            PERSISTENT_STATE(),
            STATE_MACHINE(),
            TICK,
        );

        let (requests, state) = state.handle_request(
            request,
            &mut volitile_state,
            &mut persistent_state,
            &mut state_machine,
        );

        if let RaftState::Follower(_) = state {
        } else {
            panic!("Didn't transition to follower!");
        }
        assert!(requests.len() == 0);
        assert_eq!(volitile_state.tick_since_start, 1);
    }

    #[test]
    fn test_timeout() {
        let (mut state, mut volitile_state, mut persistent_state, mut state_machine, request) = (
            FOLLOWER,
            VOLITILE_STATE_TIMEOUT,
            PERSISTENT_STATE(),
            STATE_MACHINE(),
            TICK,
        );

        let (_, state) = state.handle_request(
            request,
            &mut volitile_state,
            &mut persistent_state,
            &mut state_machine,
        );

        if let RaftState::Candidate(_) = state {
        } else {
            panic!("Didn't start election!");
        }
    }

    #[test]
    fn test_append_old_leader() {
        let (mut state, mut volitile_state, mut persistent_state, mut state_machine, mut request) = (
            FOLLOWER,
            VOLITILE_STATE,
            PERSISTENT_STATE(),
            STATE_MACHINE(),
            INSERT(3).set_term(2),
        );

        let (requests, state) = state.handle_request(
            request,
            &mut volitile_state,
            &mut persistent_state,
            &mut state_machine,
        );

        if let RaftState::Follower(_) = state {
        } else {
            panic!("Didn't remain a follower!");
        }
        assert_eq!(persistent_state, PERSISTENT_STATE());
        assert_eq!(volitile_state, VOLITILE_STATE);
        assert_eq!(requests, [INSERT_FAILED_RESPONSE.reverse_sender()]);
    }

    #[test]
    fn test_append_log_too_short() {
        let (mut state, mut volitile_state, mut persistent_state, mut state_machine, mut request) = (
            FOLLOWER,
            VOLITILE_STATE,
            PERSISTENT_STATE(),
            STATE_MACHINE(),
            INSERT(13),
        );

        let (requests, state) = state.handle_request(
            request,
            &mut volitile_state,
            &mut persistent_state,
            &mut state_machine,
        );

        if let RaftState::Follower(_) = state {
        } else {
            panic!("Didn't remain a follower!");
        }
        assert_eq!(persistent_state, PERSISTENT_STATE().set_voted(0));
        assert_eq!(volitile_state, VOLITILE_STATE);
        assert_eq!(requests, [INSERT_FAILED_RESPONSE.reverse_sender()]);
    }

    #[test]
    fn test_append_log_last_term_mismatch() {
        let (mut state, mut volitile_state, mut persistent_state, mut state_machine, mut request) = (
            FOLLOWER,
            VOLITILE_STATE,
            PERSISTENT_STATE_LOG(MISMATCH_LOG()),
            STATE_MACHINE(),
            INSERT(3),
        );

        let (requests, state) = state.handle_request(
            request,
            &mut volitile_state,
            &mut persistent_state,
            &mut state_machine,
        );
        if let RaftState::Follower(_) = state {
        } else {
            panic!("Didn't remain a follower!");
        }
        assert_eq!(
            persistent_state,
            PERSISTENT_STATE_LOG(MISMATCH_LOG()).set_voted(0)
        );
        assert_eq!(volitile_state, VOLITILE_STATE);
        assert_eq!(requests, [INSERT_FAILED_RESPONSE.reverse_sender()]);
    }

    #[test]
    fn test_append_log_basic() {
        let (mut state, mut volitile_state, mut persistent_state, mut state_machine, mut request) = (
            FOLLOWER,
            VOLITILE_STATE,
            PERSISTENT_STATE(),
            STATE_MACHINE(),
            INSERT(3),
        );

        let (requests, state) = state.handle_request(
            request,
            &mut volitile_state,
            &mut persistent_state,
            &mut state_machine,
        );
        if let RaftState::Follower(_) = state {
        } else {
            panic!("Didn't remain a follower!");
        }
        assert_eq!(
            persistent_state,
            PERSISTENT_STATE_LOG(LOG_LEADER()).set_voted(0)
        );
        assert_eq!(volitile_state, VOLITILE_STATE.set_commit(4));
        assert_eq!(requests, [INSERT_SUCCESS_RESPONSE.reverse_sender()]);
    }

    #[test]
    fn test_append_log_overwrite() {
        let config = Config {
            servers: HashSet::from([0, 1, 2]),
        };
        let log = Vec::from([
            Entry::config(0, config),
            Entry::command(1, 10),
            Entry::command(2, 4),
            Entry::command(3, 5),
            Entry::command(3, 6),
        ]);

        let mut persistent_state: PersistentState<u32> = PersistentState {
            id: 1,
            current_term: 4,
            log: log.clone(),
            ..Default::default()
        };
        let mut volitile_state = VolitileState::default();
        let follower = Follower::default();
        let entries = Vec::from([Entry::command(4, 5)]);
        let original_request: Request<u32, u32> = Request {
            sender: 0,
            reciever: persistent_state.id,
            term: 4,
            event: Event::Insert(request::Insert {
                prev_log_state: LogState { term: 2, length: 3 },
                entries: entries.clone(),
                leader_commit: 12,
            }),
        };

        let mut state_machine = Sum::default();

        let (requests, next) = follower.handle_request(
            &mut volitile_state,
            &mut persistent_state,
            &mut state_machine,
            original_request,
        );

        if let RaftState::Follower(_) = next {
        } else {
            panic!("Didn't transition to follower!");
        }
        assert!(requests.len() == 1);
        assert!(persistent_state.current_term == 4);
        assert!(persistent_state.voted_for == Some(0));
        for request in requests {
            assert!(request.sender == persistent_state.id);
            assert!(request.reciever == 0);
            assert!(request.term == persistent_state.current_term);
            match request.event {
                Event::InsertResponse(event) => {
                    assert!(event.success);
                }
                _ => {
                    panic!("Non-append response!");
                }
            }
        }

        assert!(log[0..3].iter().eq(persistent_state.log[0..3].iter()));
        assert!(entries[0..1].iter().eq(persistent_state.log[3..4].iter()));
        assert!(persistent_state.log.len() == 4);
        assert!(volitile_state.get_commit_index() == 4);
    }

    #[test]
    fn test_vote_old_term() {
        let config = Config {
            servers: HashSet::from([0, 1, 2]),
        };
        let log = Vec::from([
            Entry::config(0, config),
            Entry::command(1, 10),
            Entry::command(2, 4),
            Entry::command(3, 5),
            Entry::command(3, 6),
        ]);
        let mut persistent_state: PersistentState<u32> = PersistentState {
            id: 1,
            current_term: 4,
            log: log.clone(),
            ..Default::default()
        };
        let mut volitile_state = VolitileState::default();
        let follower = Follower::default();
        let original_request: Request<u32, u32> = Request {
            sender: 2,
            reciever: persistent_state.id,
            term: 3,
            event: Event::Vote(request::Vote {
                log_state: LogState { term: 3, length: 5 },
            }),
        };

        let mut state_machine = Sum::default();

        let (requests, next) = follower.handle_request(
            &mut volitile_state,
            &mut persistent_state,
            &mut state_machine,
            original_request,
        );

        if let RaftState::Follower(_) = next {
        } else {
            panic!("Didn't transition to follower!");
        }
        assert!(requests.len() == 1);
        assert!(persistent_state.current_term == 4);
        assert!(persistent_state.voted_for == None);
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
            Entry::config(0, config),
            Entry::command(1, 10),
            Entry::command(2, 4),
            Entry::command(3, 5),
            Entry::command(3, 6),
        ]);
        let mut persistent_state: PersistentState<u32> = PersistentState {
            id: 1,
            current_term: 4,
            log: log.clone(),
            ..Default::default()
        };
        let mut volitile_state = VolitileState::default();
        let follower = Follower::default();
        let original_request: Request<u32, u32> = Request {
            sender: 2,
            reciever: persistent_state.id,
            term: 4,
            event: Event::Vote(request::Vote {
                log_state: LogState { term: 4, length: 3 },
            }),
        };

        let mut state_machine = Sum::default();

        let (requests, next) = follower.handle_request(
            &mut volitile_state,
            &mut persistent_state,
            &mut state_machine,
            original_request,
        );

        if let RaftState::Follower(_) = next {
        } else {
            panic!("Didn't transition to follower!");
        }
        assert!(requests.len() == 1);
        assert!(persistent_state.current_term == 4);
        assert!(persistent_state.voted_for == None);
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
            Entry::config(0, config),
            Entry::command(1, 10),
            Entry::command(2, 4),
            Entry::command(3, 5),
            Entry::command(3, 6),
        ]);
        let mut persistent_state: PersistentState<u32> = PersistentState {
            id: 1,
            current_term: 4,
            log: log.clone(),
            ..Default::default()
        };
        let mut volitile_state = VolitileState::default();
        let follower = Follower::default();
        let original_request: Request<u32, u32> = Request {
            sender: 2,
            reciever: persistent_state.id,
            term: 4,
            event: Event::Vote(request::Vote {
                log_state: LogState { term: 2, length: 4 },
            }),
        };

        let mut state_machine = Sum::default();

        let (requests, next) = follower.handle_request(
            &mut volitile_state,
            &mut persistent_state,
            &mut state_machine,
            original_request,
        );

        if let RaftState::Follower(_) = next {
        } else {
            panic!("Didn't transition to follower!");
        }
        assert!(requests.len() == 1);
        assert!(persistent_state.current_term == 4);
        assert!(persistent_state.voted_for == None);
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
            Entry::config(0, config),
            Entry::command(1, 10),
            Entry::command(2, 4),
            Entry::command(3, 5),
            Entry::command(3, 6),
        ]);
        let mut persistent_state: PersistentState<u32> = PersistentState {
            id: 1,
            current_term: 4,
            log: log.clone(),
            ..Default::default()
        };
        let mut volitile_state = VolitileState::default();
        let follower = Follower::default();
        let original_request: Request<u32, u32> = Request {
            sender: 2,
            reciever: persistent_state.id,
            term: 4,
            event: Event::Vote(request::Vote {
                log_state: LogState { term: 3, length: 5 },
            }),
        };

        let mut state_machine = Sum::default();

        let (requests, next) = follower.handle_request(
            &mut volitile_state,
            &mut persistent_state,
            &mut state_machine,
            original_request,
        );

        if let RaftState::Follower(_) = next {
        } else {
            panic!("Didn't transition to follower!");
        }
        assert!(requests.len() == 1);
        assert!(persistent_state.current_term == 4);
        assert!(persistent_state.voted_for == Some(2));
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
            Entry::config(0, config),
            Entry::command(1, 10),
            Entry::command(2, 4),
            Entry::command(3, 5),
            Entry::command(3, 6),
        ]);
        let mut persistent_state: PersistentState<u32> = PersistentState {
            id: 1,
            current_term: 4,
            log: log.clone(),
            ..Default::default()
        };
        let mut volitile_state = VolitileState::default();
        let follower = Follower::default();
        let original_request: Request<u32, u32> = Request {
            sender: 2,
            reciever: persistent_state.id,
            term: 4,
            event: Event::Vote(request::Vote {
                log_state: LogState { term: 2, length: 6 },
            }),
        };

        let mut state_machine = Sum::default();

        let (requests, next) = follower.handle_request(
            &mut volitile_state,
            &mut persistent_state,
            &mut state_machine,
            original_request,
        );

        if let RaftState::Follower(_) = next {
        } else {
            panic!("Didn't transition to follower!");
        }
        assert!(requests.len() == 1);
        assert!(persistent_state.current_term == 4);
        assert!(persistent_state.voted_for == Some(2));
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
            Entry::config(0, config),
            Entry::command(1, 10),
            Entry::command(3, 4),
        ]);
        let mut persistent_state: PersistentState<u32> = PersistentState {
            id: 1,
            current_term: 3,
            log: log.clone(),
            voted_for: Some(0),
            ..Default::default()
        };
        let mut volitile_state = VolitileState {
            commit_index: 1,
            ..Default::default()
        };
        let request: Request<u32, u32> = Request {
            sender: 10,
            reciever: persistent_state.id,
            term: 0,
            event: Event::Client(request::Client {
                data: Data::Command(2),
            }),
        };

        let follower = Follower {};

        let mut state_machine = Sum::default();

        let (requests, next) = follower.handle_request(
            &mut volitile_state,
            &mut persistent_state,
            &mut state_machine,
            request,
        );

        if let RaftState::Follower(_) = next {
        } else {
            panic!("Didn't transition to follower!");
        }
        assert_eq!(requests.len(), 1);
        for request in requests {
            assert_eq!(request.sender, persistent_state.id);
            assert_eq!(request.reciever, 10);
            assert_eq!(request.term, 0);
            match request.event {
                Event::ClientResponse(ClientResponse::Failed {
                    leader_id: Some(leader_id),
                    data: Data::Command(data),
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
            Entry::config(0, config),
            Entry::command(1, 10),
            Entry::command(3, 4),
        ]);

        let mut persistent_state: PersistentState<u32> = PersistentState {
            id: 1,
            current_term: 3,
            log: log.clone(),
            voted_for: None,
            ..Default::default()
        };
        let mut volitile_state = VolitileState {
            commit_index: 1,
            ..Default::default()
        };
        let request: Request<u32, u32> = Request {
            sender: 10,
            reciever: persistent_state.id,
            term: 0,
            event: Event::Client(request::Client {
                data: Data::Command(2),
            }),
        };

        let follower = Follower {};

        let mut state_machine = Sum::default();

        let (requests, next) = follower.handle_request(
            &mut volitile_state,
            &mut persistent_state,
            &mut state_machine,
            request,
        );

        if let RaftState::Follower(_) = next {
        } else {
            panic!("Didn't transition to follower!");
        }
        assert_eq!(requests.len(), 1);
        for request in requests {
            assert_eq!(request.sender, persistent_state.id);
            assert_eq!(request.reciever, 10);
            assert_eq!(request.term, 0);
            match request.event {
                Event::ClientResponse(ClientResponse::Failed {
                    leader_id: None,
                    data: Data::Command(data),
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
