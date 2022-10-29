use std::collections::HashSet;

use super::{follower::Follower, leader::Leader};
use crate::state::{
    handler::{EventHandler, Handler},
    raft_state::RaftState,
};
use crate::{
    data::{
        data_type::CommandType,
        persistent_state::PersistentState,
        request::{Event, Request, Tick, Vote, VoteResponse},
        volitile_state::VolitileState,
    },
    state::state::StateMachine,
};

#[derive(Default, Debug, PartialEq, Clone)]
pub struct Candidate {
    votes: HashSet<u32>,
}
const TICK_TILL_NEW_ELECTION: u32 = 10;

impl Handler for Candidate {}
impl EventHandler for Candidate {
    fn handle<T: CommandType, Output, SM>(
        mut self,
        volitile_state: &mut VolitileState,
        persistent_state: &mut PersistentState<T>,
        _state_machine: &mut SM,
        sender: u32,
        term: u32,
        request: Request<T, Output>,
    ) -> (Vec<Request<T, Output>>, RaftState)
    where
        SM: StateMachine<T, Output>,
    {
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
                if persistent_state.has_quorum(&self.votes) {
                    return Leader::from_candidate(self, volitile_state, persistent_state);
                }
                (Vec::default(), self.into())
            }
            _ => (Vec::default(), self.into()),
        }
    }
}

impl Candidate {
    fn request_votes<T: CommandType, Output>(
        persistent_state: &mut PersistentState<T>,
    ) -> Vec<Request<T, Output>> {
        persistent_state
            .other_servers()
            .iter()
            .map(|id| Request::<T, Output> {
                sender: persistent_state.id,
                reciever: *id,
                term: persistent_state.current_term,
                event: Event::Vote(Vote {
                    log_state: persistent_state.log_state(),
                }),
            })
            .collect()
    }
    pub fn call_election<T: CommandType, Output>(
        volitile_state: &mut VolitileState,
        persistent_state: &mut PersistentState<T>,
    ) -> (Vec<Request<T, Output>>, RaftState) {
        println!("{} running for office!", persistent_state.id);
        persistent_state.current_term += 1;
        persistent_state.voted_for = Some(persistent_state.id);
        volitile_state.tick_since_start = 0;
        (
            Candidate::request_votes(persistent_state),
            Candidate {
                votes: [persistent_state.id].into(),
            }
            .into(),
        )
    }
}
#[cfg(test)]
pub mod test_util {
    use super::Candidate;
    use crate::state::raft_state::RaftState;

    pub fn BASE_CANDIDATE() -> RaftState {
        RaftState::Candidate(Candidate { votes: [1].into() })
    }

    pub fn CANDIDATE(votes: &[u32]) -> RaftState {
        RaftState::Candidate(Candidate {
            votes: votes.iter().map(|id| *id).collect(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::test_util::BASE_CANDIDATE;
    use crate::data::request::test_util::{
        INSERT, MASS_HEARTBEAT, REQUEST_VOTES, TICK, VOTE_NO_RESPONSE, VOTE_YES_RESPONSE,
    };
    use crate::data::volitile_state::test_util::{VOLITILE_STATE, VOLITILE_STATE_TIMEOUT};
    use crate::state::concrete::candidate::test_util::CANDIDATE;
    use crate::state::concrete::follower::test_util::FOLLOWER;
    use crate::state::concrete::leader::test_util::BASE_LEADER;
    use crate::state::state::test_util::TestCase;
    use crate::state::state::State;

    #[test]
    fn test_tick() {
        let state = State::create_state(BASE_CANDIDATE()).set_voted(1);
        let mut test_case = TestCase::new(state, TICK)
            .set_vs(VOLITILE_STATE.increment_tick())
            .responses(&REQUEST_VOTES(4));
        test_case.run();
    }

    #[test]
    fn test_timeout() {
        let state = State::create_state(BASE_CANDIDATE())
            .set_vs(VOLITILE_STATE_TIMEOUT)
            .set_voted(1);
        let mut test_case = TestCase::new(state, TICK)
            .set_vs(VOLITILE_STATE)
            .set_term(5)
            .responses(&REQUEST_VOTES(5));
        test_case.run();
    }

    #[test]
    fn test_request_vote_rejection() {
        let state = State::create_state(BASE_CANDIDATE()).set_voted(1);
        let mut test_case = TestCase::new(state, VOTE_NO_RESPONSE);
        test_case.run();
    }

    #[test]
    fn test_request_vote_successful() {
        let state = State::create_state(BASE_CANDIDATE()).set_voted(1);
        let mut test_case = TestCase::new(state, VOTE_YES_RESPONSE).set_rs(CANDIDATE(&[0, 1]));
        test_case.run();
    }

    #[test]
    fn test_request_vote_successful_redundant() {
        let state = State::create_state(CANDIDATE(&[0, 1])).set_voted(1);
        let mut test_case = TestCase::new(state, VOTE_YES_RESPONSE);
        test_case.run();
    }

    #[test]
    fn test_request_vote_successful_elected() {
        let state = State::create_state(CANDIDATE(&[1, 2])).set_voted(1);
        let mut test_case = TestCase::new(state, VOTE_YES_RESPONSE)
            .set_rs(BASE_LEADER(3, 0))
            .set_term(5)
            .responses(&MASS_HEARTBEAT(5));
        test_case.run();
    }

    #[test]
    fn test_append_old_leader() {
        let state = State::create_state(CANDIDATE(&[1])).set_voted(1);
        let mut test_case = TestCase::new(state, INSERT(4).set_term(2));
        test_case.run();
    }

    #[test]
    fn test_append_current_leader() {
        let state = State::create_state(CANDIDATE(&[1])).set_voted(1);
        let mut test_case = TestCase::new(state, INSERT(4).set_term(4))
            .set_rs(FOLLOWER)
            .set_voted(0);
        test_case.run();
    }
}
