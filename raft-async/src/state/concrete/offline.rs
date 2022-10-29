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
pub mod test_util {
    use crate::state::raft_state::RaftState;

    use super::Offline;

    pub const OFFLINE: RaftState = RaftState::Offline(Offline);
}
#[cfg(test)]
mod tests {

    use crate::data::persistent_state::test_util::PERSISTENT_STATE;
    use crate::data::request::test_util::TICK;
    use crate::data::volitile_state::test_util::{VOLITILE_STATE, VOLITILE_STATE_TIMEOUT};
    use crate::state::concrete::offline::test_util::OFFLINE;
    use crate::test_util::STATE_MACHINE;
    // Note this useful idiom: importing names from outer (for mod tests) scope.
    use super::*;

    #[test]
    fn test_tick() {
        let (mut state, mut volitile_state, mut persistent_state, mut state_machine, request) = (
            OFFLINE,
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

        if let RaftState::Offline(_) = state {
        } else {
            panic!("Didn't transition to offline!");
        }
        assert!(requests.is_empty());
        assert_eq!(persistent_state.voted_for, None);
        assert_eq!(volitile_state.tick_since_start, 1);
        assert_eq!(
            volitile_state.get_commit_index(),
            VOLITILE_STATE.get_commit_index()
        );
    }

    #[test]
    fn test_reboot() {
        let (mut state, mut volitile_state, mut persistent_state, mut state_machine, request) = (
            OFFLINE,
            VOLITILE_STATE_TIMEOUT,
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
        assert!(requests.is_empty());
        assert_eq!(persistent_state.voted_for, None);
        assert_eq!(volitile_state.tick_since_start, 0);
        assert_eq!(volitile_state.get_commit_index(), 0);
    }
}
