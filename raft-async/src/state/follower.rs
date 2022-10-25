use std::time::Duration;

use crate::data::{
    data_type::DataType,
    persistent_state::PersistentState,
    request::{
        Append, AppendResponse, Client, ClientResponse, Event, Request, Timeout, Vote, VoteResponse,
    },
    volitile_state::VolitileState,
};

use super::{
    candidate::Candidate,
    raft_state::{EventHandler, Handler, RaftState, TimeoutHandler},
};

#[derive(Default)]
pub struct Follower {}

const FOLLOWER_TIMEOUT: u128 = 2000;
impl TimeoutHandler for Follower {
    fn timeout_length(&self) -> Duration {
        Duration::from_millis(1000)
    }
}

impl<T: DataType> EventHandler<AppendResponse, T> for Follower {}
impl<T: DataType> EventHandler<Timeout, T> for Follower {
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
impl<T: DataType> EventHandler<VoteResponse, T> for Follower {}
impl<T: DataType> EventHandler<Vote, T> for Follower {
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
            persistent_state.voted_for = Some(sender);
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

impl<T: DataType> EventHandler<Client<T>, T> for Follower {
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
impl<T: DataType> EventHandler<ClientResponse<T>, T> for Follower {}
