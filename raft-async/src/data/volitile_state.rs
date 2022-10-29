use crate::state::state::StateMachine;

use super::{data_type::CommandType, persistent_state::PersistentState, request::Data};

#[derive(Default, Clone, Copy)]
pub struct VolitileState {
    pub commit_index: usize,
    pub tick_since_start: u32,
}

impl VolitileState {
    pub fn get_commit_index(&self) -> usize {
        self.commit_index
    }

    pub fn try_update_commit_index<T: CommandType, Output, SM: StateMachine<T, Output>>(
        &mut self,
        state_machine: &mut SM,
        persistent_state: &PersistentState<T>,
        new_commit_index: usize,
    ) -> bool {
        if self.commit_index >= new_commit_index {
            return false;
        }
        for index in self.commit_index..new_commit_index {
            match &persistent_state.log[index].data {
                Data::Command(command) => {
                    state_machine.apply(command.clone());
                }
                _ => {}
            }
        }
        self.commit_index = new_commit_index;
        true
    }
}

#[cfg(test)]
pub mod test_util {
    use super::VolitileState;

    pub const VOLITILE_STATE: VolitileState = VolitileState {
        commit_index: 2,
        tick_since_start: 0,
    };

    pub const VOLITILE_STATE_TIMEOUT: VolitileState = VolitileState {
        commit_index: 2,
        tick_since_start: 100000,
    };
}
