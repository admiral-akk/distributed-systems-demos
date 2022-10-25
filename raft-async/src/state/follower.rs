use std::time::Duration;

use crate::data::{
    data_type::DataType,
    persistent_state::PersistentState,
    request::{Append, AppendResponse, Event, Request, Timeout, Vote, VoteResponse},
    volitile_state::VolitileState,
};

use super::raft_state::{EventHandler, Handler, RaftState, TimeoutHandler};

#[derive(Default)]
pub struct Follower {}

const FOLLOWER_TIMEOUT: u128 = 2000;
impl TimeoutHandler for Follower {
    fn timeout_length(&self) -> Duration {
        Duration::from_millis(1000)
    }
}

impl<T: DataType> EventHandler<AppendResponse, T> for Follower {}
impl<T: DataType> EventHandler<Timeout, T> for Follower {}
impl<T: DataType> EventHandler<VoteResponse, T> for Follower {}
impl<T: DataType> EventHandler<Vote, T> for Follower {
    fn handle_event(
        &mut self,
        volitile_state: &mut VolitileState,
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
        if persistent_state.log.len() > 0 {
            success &= event.log_length + 1 < persistent_state.log.len();
            success &= event.log_length == persistent_state.log.len()
                && persistent_state.log[event.log_length].term <= event.last_log_term;
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

impl<T: DataType> EventHandler<Append<T>, T> for Follower {
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
        }
        // If we don't have the previous entry, then the append fails.
        success &= persistent_state.log.len() >= event.prev_log_length;
        if success && event.prev_log_length > 0 {
            // If we have a previous entry, then the term needs to match.
            success &= persistent_state.log[event.prev_log_length - 1].term == event.prev_log_term;
        }

        if success {
            for (index, &entry) in event.entries.iter().enumerate() {
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
                volitile_state.commit_index = event
                    .leader_commit
                    .min(event.prev_log_length + event.entries.len());
            } else {
                volitile_state.commit_index = event.leader_commit;
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

impl<T: DataType> Handler<T> for Follower {}
