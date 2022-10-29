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

#[derive(Debug, PartialEq, Clone)]
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
    use crate::data::volitile_state::test_util::{
        FRESH_VOLITILE_STATE, VOLITILE_STATE, VOLITILE_STATE_TIMEOUT,
    };
    use crate::state::concrete::follower::test_util::FOLLOWER;
    use crate::state::concrete::offline::test_util::OFFLINE;
    use crate::state::state::test_util::{create_state, TestCase};
    use crate::test_util::STATE_MACHINE;
    // Note this useful idiom: importing names from outer (for mod tests) scope.
    use super::*;

    #[test]
    fn test_tick() {
        let state = create_state(STATE_MACHINE(), PERSISTENT_STATE(), OFFLINE, VOLITILE_STATE);
        let request = TICK;
        let mut test_case =
            TestCase::new(state, request, "Offline - Tick").set_vs(VOLITILE_STATE.increment_tick());
        test_case.run();
    }

    #[test]
    fn test_reboot() {
        let state = create_state(
            STATE_MACHINE(),
            PERSISTENT_STATE(),
            OFFLINE,
            VOLITILE_STATE_TIMEOUT,
        );
        let request = TICK;
        let mut test_case = TestCase::new(state, request, "Offline - Tick")
            .set_vs(VOLITILE_STATE.set_commit(0))
            .set_rs(FOLLOWER);
        test_case.run();
    }
}
