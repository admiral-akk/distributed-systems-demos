use crate::state::state::StateMachine;

use super::{data_type::CommandType, persistent_state::PersistentState};

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
            state_machine.apply(persistent_state.log[index].command.clone());
        }
        self.commit_index = new_commit_index;
        true
    }
}
