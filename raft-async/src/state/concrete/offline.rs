use crate::{
    data::{
        data_type::CommandType,
        persistent_state::PersistentState,
        request::{Event, Request, Tick},
        volitile_state::VolitileState,
    },
    state::{
        handler::{EventHandler, Handler},
        raft_state::RaftState,
        state::StateMachine,
    },
};

use super::follower::Follower;

pub struct Offline;

impl Handler for Offline {}

const TICK_TO_REBOOT: u32 = 100;

impl EventHandler for Offline {
    fn handle<T: CommandType, Output, SM>(
        self,
        volitile_state: &mut VolitileState,
        persistent_state: &mut PersistentState<T>,
        _state_machine: &mut SM,
        _sender: u32,
        _term: u32,
        request: Request<T, Output>,
    ) -> (Vec<Request<T, Output>>, RaftState)
    where
        SM: StateMachine<T, Output>,
    {
        match request.event {
            Event::Tick(Tick) => {
                if volitile_state.tick_since_start < TICK_TO_REBOOT {
                    return Default::default();
                }
                *volitile_state = VolitileState::default();
                persistent_state.voted_for = None;
                (Vec::new(), Follower::default().into())
            }
            _ => (Vec::default(), self.into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use crate::data::persistent_state::{Config, Entry};
    use crate::data::request::{self, Event};
    use crate::Sum;
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
        let follower = Offline {};
        let request: Request<u32, u32> = Request {
            sender: 10,
            reciever: persistent_state.id,
            term: 0,
            event: Event::Tick(request::Tick),
        };
        let mut state_machine = Sum::default();

        let (requests, next) = follower.handle_request(
            &mut volitile_state,
            &mut persistent_state,
            &mut state_machine,
            request,
        );

        if let RaftState::Offline(_) = next {
        } else {
            panic!("Didn't transition to offline!");
        }
        assert!(requests.is_empty());
        assert_eq!(persistent_state.voted_for, None);
        assert_eq!(volitile_state.tick_since_start, 1);
        assert_eq!(volitile_state.get_commit_index(), 0);
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
        let follower = Offline {};
        let request: Request<u32, u32> = Request {
            sender: 10,
            reciever: persistent_state.id,
            term: 0,
            event: Event::Tick(request::Tick),
        };
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
        assert!(requests.is_empty());
        assert_eq!(persistent_state.voted_for, None);
        assert_eq!(volitile_state.tick_since_start, 0);
        assert_eq!(volitile_state.get_commit_index(), 0);
    }
}
