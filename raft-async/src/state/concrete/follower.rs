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
        LOG, LOG_LEADER, MISMATCH_LOG, PERSISTENT_STATE,
    };
    use crate::data::persistent_state::{Config, Entry, LogState};
    use crate::data::request::test_util::{
        INSERT, INSERT_FAILED_RESPONSE, INSERT_SUCCESS_RESPONSE, REQUEST_VOTES, TICK, VOTE,
        VOTE_NEW_SHORT, VOTE_NO_RESPONSE, VOTE_OLD_EQUAL, VOTE_OLD_LONG, VOTE_YES_RESPONSE,
    };
    use crate::data::request::{self, Data};
    use crate::data::volitile_state::test_util::{VOLITILE_STATE, VOLITILE_STATE_TIMEOUT};
    use crate::state::concrete::candidate::test_util::CANDIDATE;
    use crate::state::concrete::follower::test_util::FOLLOWER;
    use crate::state::concrete::follower::Follower;
    use crate::state::state::test_util::TestCase;
    use crate::state::state::State;
    use crate::Sum;
    // Note this useful idiom: importing names from outer (for mod tests) scope.
    use super::*;

    #[test]
    fn test_tick() {
        let state = State::create_state(FOLLOWER);

        let mut test_case = TestCase::new(state, TICK).set_vs(VOLITILE_STATE.increment_tick());
        test_case.run();
    }

    #[test]
    fn test_timeout() {
        let state = State::create_state(FOLLOWER).set_vs(VOLITILE_STATE_TIMEOUT);

        let mut test_case = TestCase::new(state, TICK)
            .set_rs(CANDIDATE())
            .set_vs(VOLITILE_STATE)
            .set_ps(PERSISTENT_STATE().increment_term().set_voted(1))
            .responses(&REQUEST_VOTES(5));
        test_case.run();
    }

    #[test]
    fn test_append_old_leader() {
        let state = State::create_state(FOLLOWER);
        let mut test_case = TestCase::new(state, INSERT(2).set_term(2))
            .responses(&[INSERT_FAILED_RESPONSE.reverse_sender()]);
        test_case.run();
    }

    #[test]
    fn test_append_log_too_short() {
        let state = State::create_state(FOLLOWER);
        let mut test_case = TestCase::new(state, INSERT(4))
            .responses(&[INSERT_FAILED_RESPONSE.reverse_sender()])
            .set_voted(0);
        test_case.run();
    }

    #[test]
    fn test_append_log_last_term_mismatch() {
        let state = State::create_state(FOLLOWER).set_log(MISMATCH_LOG());
        let mut test_case = TestCase::new(state, INSERT(3))
            .responses(&[INSERT_FAILED_RESPONSE.reverse_sender()])
            .set_voted(0);
        test_case.run();
    }

    #[test]
    fn test_append_log_basic() {
        let state = State::create_state(FOLLOWER);
        let mut test_case = TestCase::new(state, INSERT(3))
            .responses(&[INSERT_SUCCESS_RESPONSE.reverse_sender()])
            .set_log(LOG_LEADER()[0..4].into())
            .set_voted(0)
            .set_commit(4)
            .set_sum(19);
        test_case.run();
    }

    #[test]
    fn test_append_log_overwrite() {
        let state = State::create_state(FOLLOWER).set_log(MISMATCH_LOG());
        let mut test_case = TestCase::new(state, INSERT(2))
            .responses(&[INSERT_SUCCESS_RESPONSE.reverse_sender()])
            .set_log(LOG())
            .set_voted(0)
            .set_commit(3)
            .set_sum(14);
        test_case.run();
    }

    #[test]
    fn test_vote_old_term() {
        let state = State::create_state(FOLLOWER);
        let mut test_case =
            TestCase::new(state, VOTE.set_term(2)).responses(&[VOTE_NO_RESPONSE.reverse_sender()]);
        test_case.run();
    }

    #[test]
    fn test_vote_shorter_log_larger_last_term() {
        let state = State::create_state(FOLLOWER);
        let mut test_case =
            TestCase::new(state, VOTE_NEW_SHORT).responses(&[VOTE_NO_RESPONSE.reverse_sender()]);
        test_case.run();
    }

    #[test]
    fn test_vote_same_log_length_older_term() {
        let state = State::create_state(FOLLOWER);
        let mut test_case =
            TestCase::new(state, VOTE_OLD_EQUAL).responses(&[VOTE_NO_RESPONSE.reverse_sender()]);
        test_case.run();
    }

    #[test]
    fn test_vote_same_log_length_same_term() {
        let state = State::create_state(FOLLOWER);
        let mut test_case = TestCase::new(state, VOTE)
            .responses(&[VOTE_YES_RESPONSE.reverse_sender()])
            .set_voted(0);
        test_case.run();
    }

    #[test]
    fn test_vote_longer_log_length_older_term() {
        let state = State::create_state(FOLLOWER);
        let mut test_case = TestCase::new(state, VOTE_OLD_LONG)
            .responses(&[VOTE_YES_RESPONSE.reverse_sender()])
            .set_voted(0);
        test_case.run();
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
