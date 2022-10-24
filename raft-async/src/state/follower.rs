use std::time::{self, SystemTime};

use crate::data::{
    data_type::DataType,
    persistent_state::PersistentState,
    request::{Request, RequestType},
    volitile_state::VolitileState,
};

use super::{
    candidate::Candidate,
    raft_state::{Handler, RaftStateGeneric, RaftStateWrapper},
};

#[derive(Default)]
pub struct Follower {}

impl Default for RaftStateGeneric<Follower> {
    fn default() -> Self {
        Self {
            state: Follower::default(),
        }
    }
}

const FOLLOWER_TIMEOUT: u128 = 2000;

impl<T: DataType> Handler<T> for RaftStateGeneric<Follower> {
    fn handle(
        &mut self,
        request: Request<T>,
        volitile_state: &mut VolitileState,
        persistent_state: &mut PersistentState<T>,
    ) -> (Vec<Request<T>>, Option<RaftStateWrapper>) {
        let (sender, term) = (request.sender, request.term);
        match request.data {
            RequestType::Vote {
                log_length: prev_log_index,
                last_log_term: prev_log_term,
            } => {
                println!("{} requested vote from {}", sender, persistent_state.id);
                let mut success = true;
                success &= persistent_state.current_term <= term;
                if let Some(voted_for) = persistent_state.voted_for {
                    success &= voted_for != sender;
                }
                if persistent_state.log.len() > 0 {
                    success &= prev_log_index + 1 < persistent_state.log.len();
                    success &= prev_log_index == persistent_state.log.len()
                        && persistent_state.log[prev_log_index].term <= prev_log_term;
                }
                (
                    Vec::from([Request {
                        sender: persistent_state.id,
                        reciever: sender,
                        term: persistent_state.current_term,
                        data: RequestType::VoteResponse { success },
                    }]),
                    None,
                )
            }
            RequestType::Append {
                prev_log_length,
                prev_log_term,
                entries,
                leader_commit,
            } => {
                let mut success = true;
                success &= persistent_state.current_term <= term;
                persistent_state.last_heartbeat = Some(SystemTime::now());
                // If we don't have the previous entry, then the append fails.
                success &= persistent_state.log.len() >= prev_log_length;
                if success && prev_log_length > 0 {
                    // If we have a previous entry, then the term needs to match.
                    success &= persistent_state.log[prev_log_length - 1].term == prev_log_term;
                }

                if success {
                    for (index, &entry) in entries.iter().enumerate() {
                        let log_index = prev_log_length + index;
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

                if leader_commit > volitile_state.commit_index {
                    if success {
                        volitile_state.commit_index =
                            leader_commit.min(prev_log_length + entries.len());
                    } else {
                        volitile_state.commit_index = leader_commit;
                    }
                }

                (
                    Vec::from([Request {
                        sender: persistent_state.id,
                        reciever: sender,
                        term: persistent_state.current_term,
                        data: RequestType::AppendResponse { success },
                    }]),
                    None,
                )
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
            if now.duration_since(last_heartbeat).unwrap().as_millis() < FOLLOWER_TIMEOUT {
                return (Vec::default(), None);
            }
        }
        RaftStateGeneric::<Candidate>::call_election(persistent_state)
    }
}
