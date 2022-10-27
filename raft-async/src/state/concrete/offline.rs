use crate::{
    data::{
        data_type::CommandType,
        persistent_state::PersistentState,
        request::{
            Client, ClientResponse, Crash, Insert, InsertResponse, Request, Tick, Vote,
            VoteResponse,
        },
        volitile_state::VolitileState,
    },
    state::{
        handler::{EventHandler, Handler},
        raft_state::RaftState,
    },
};

use super::follower::Follower;

pub struct Offline {}

impl<T: CommandType> Handler<T> for Offline {}
impl<T: CommandType> EventHandler<Crash, T> for Offline {
    fn handle_event(
        &mut self,
        _volitile_state: &mut VolitileState,
        _persistent_state: &mut PersistentState<T>,
        _sender: u32,
        _term: u32,
        _event: Crash,
    ) -> (Vec<Request<T>>, Option<RaftState>) {
        (Vec::default(), Some(RaftState::Offline(Offline {})))
    }
}
impl<T: CommandType> EventHandler<Insert<T>, T> for Offline {}
impl<T: CommandType> EventHandler<Client<T>, T> for Offline {}
impl<T: CommandType> EventHandler<ClientResponse<T>, T> for Offline {}
impl<T: CommandType> EventHandler<Vote, T> for Offline {}
impl<T: CommandType> EventHandler<VoteResponse, T> for Offline {}

const TICK_TO_REBOOT: u32 = 100;

impl<T: CommandType> EventHandler<Tick, T> for Offline {
    fn handle_event(
        &mut self,
        volitile_state: &mut VolitileState,
        persistent_state: &mut PersistentState<T>,
        _sender: u32,
        _term: u32,
        _event: Tick,
    ) -> (Vec<Request<T>>, Option<RaftState>) {
        volitile_state.tick_since_start += 1;
        if volitile_state.tick_since_start < TICK_TO_REBOOT {
            return Default::default();
        }
        *volitile_state = VolitileState::default();
        persistent_state.voted_for = None;
        (Vec::new(), Some(Follower::default().into()))
    }
}

impl<T: CommandType> EventHandler<InsertResponse, T> for Offline {}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use crate::data::persistent_state::{Config, Entry};
    use crate::data::request::{self, Event};
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
                    term: 2,
                    command: 4,
                },
            ]),
            ..Default::default()
        };
        let mut volitile_state = VolitileState {
            commit_index: 0,
            tick_since_start: 0,
        };
        let mut follower = Offline {};
        let request: Request<u32> = Request {
            sender: 10,
            reciever: persistent_state.id,
            term: 0,
            event: Event::Tick(request::Tick),
        };

        let (requests, next) =
            follower.handle_request(&mut volitile_state, &mut persistent_state, request);

        assert!(next.is_none());
        assert!(requests.is_empty());
        assert_eq!(persistent_state.voted_for, None);
        assert_eq!(volitile_state.tick_since_start, 1);
        assert_eq!(volitile_state.commit_index, 0);
    }

    #[test]
    fn test_reboot() {
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
        let mut volitile_state = VolitileState {
            commit_index: 1000,
            tick_since_start: 100000,
        };
        let mut follower = Offline {};
        let request: Request<u32> = Request {
            sender: 10,
            reciever: persistent_state.id,
            term: 0,
            event: Event::Tick(request::Tick),
        };

        let (requests, next) =
            follower.handle_request(&mut volitile_state, &mut persistent_state, request);

        assert!(next.is_some());
        if let Some(RaftState::Follower(_)) = next {
        } else {
            panic!("Didn't transition to follower!");
        }
        assert!(requests.is_empty());
        assert_eq!(persistent_state.voted_for, None);
        assert_eq!(volitile_state.tick_since_start, 0);
        assert_eq!(volitile_state.commit_index, 0);
    }
}
