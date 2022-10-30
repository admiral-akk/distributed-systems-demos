use crate::{
    data::{
        data_type::CommandType,
        persistent_state::PersistentState,
        request::{Event, Request, Tick},
        volitile_state::VolitileState,
    },
    server::raft_cluster::Id,
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
        _sender: Id,
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
    use super::Offline;
    use crate::state::raft_state::RaftState;

    pub const OFFLINE: RaftState = RaftState::Offline(Offline);
}
#[cfg(test)]
mod tests {
    use crate::data::request::test_util::TICK;
    use crate::data::volitile_state::test_util::{VOLITILE_STATE, VOLITILE_STATE_TIMEOUT};
    use crate::state::concrete::follower::test_util::FOLLOWER;
    use crate::state::concrete::offline::test_util::OFFLINE;
    use crate::state::state::test_util::TestCase;
    use crate::state::state::State;

    #[test]
    fn test_tick() {
        let state = State::create_state(OFFLINE);
        let mut test_case = TestCase::new(state, TICK).set_vs(VOLITILE_STATE.increment_tick());
        test_case.run();
    }

    #[test]
    fn test_reboot() {
        let state = State::create_state(OFFLINE).set_vs(VOLITILE_STATE_TIMEOUT);

        let mut test_case = TestCase::new(state, TICK)
            .set_rs(FOLLOWER)
            .set_vs(crate::data::volitile_state::test_util::FRESH_VOLITILE_STATE);
        test_case.run();
    }
}
