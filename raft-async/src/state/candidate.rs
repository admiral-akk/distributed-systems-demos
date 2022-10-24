use std::{collections::HashSet, time::SystemTime};

use super::{
    follower::Follower,
    raft_state::{Handler, RaftStateGeneric, RaftStateWrapper},
};
use crate::data::{
    data_type::DataType,
    persistent_state::PersistentState,
    request::{Request, RequestType},
    volitile_state::VolitileState,
};

#[derive(Default)]
pub struct Candidate {
    votes: HashSet<u32>,
}

impl Default for RaftStateGeneric<Candidate> {
    fn default() -> Self {
        Self {
            state: Candidate::default(),
        }
    }
}

const CANDIDATE_TIMEOUT: u128 = 5000;

impl<T: DataType> Handler<T> for RaftStateGeneric<Candidate> {
    fn handle(
        &mut self,
        request: Request<T>,
        volitile_state: &mut VolitileState,
        persistent_state: &mut PersistentState<T>,
    ) -> (Vec<Request<T>>, Option<RaftStateWrapper>) {
        let (sender, term) = (request.sender, request.term);
        if term > persistent_state.current_term {
            persistent_state.current_term = term;
            persistent_state.last_heartbeat = Some(SystemTime::now());
            persistent_state.voted_for = None;
            return (
                Vec::default(),
                Some(
                    RaftStateGeneric::<Follower> {
                        state: Default::default(),
                    }
                    .into(),
                ),
            );
        }
        match request.data {
            RequestType::Append { .. } => {
                if request.term >= persistent_state.current_term {
                    return (
                        Vec::default(),
                        Some(
                            RaftStateGeneric::<Follower> {
                                state: Default::default(),
                            }
                            .into(),
                        ),
                    );
                }
                (Vec::default(), None)
            }
            RequestType::VoteResponse { success } => {
                if success {
                    println!("{} voted for {}", request.sender, persistent_state.id);
                    self.state.votes.insert(request.sender);
                }
                if self.state.votes.len() > persistent_state.quorum() {
                    return RaftStateGeneric::from_candidate(
                        &self,
                        volitile_state,
                        persistent_state,
                    );
                }
                (Vec::default(), None)
            }
            _ => (Vec::default(), None),
        }
    }

    fn check_timeout(
        &mut self,
        volitile_state: &mut VolitileState,
        persistent_state: &mut PersistentState<T>,
    ) -> (Vec<Request<T>>, Option<RaftStateWrapper>) {
        if let Some(last_heartbeat) = persistent_state.last_heartbeat {
            let now = SystemTime::now();
            if now.duration_since(last_heartbeat).unwrap().as_millis() < CANDIDATE_TIMEOUT {
                return (Vec::default(), None);
            }
        }
        RaftStateGeneric::<Candidate>::call_election(persistent_state)
    }
}

impl RaftStateGeneric<Candidate> {
    pub fn call_election<T: DataType>(
        persistent_state: &mut PersistentState<T>,
    ) -> (Vec<Request<T>>, Option<RaftStateWrapper>) {
        println!("{} running for office!", persistent_state.id);
        persistent_state.current_term += 1;
        persistent_state.voted_for = Some(persistent_state.id);
        persistent_state.last_heartbeat = Some(SystemTime::now());
        (
            persistent_state.request_votes(),
            Some(RaftStateGeneric::<Candidate>::default().into()),
        )
    }
}
