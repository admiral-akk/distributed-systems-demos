use std::time::SystemTime;

#[derive(Default, Clone, Copy)]
pub struct VolitileState {
    pub commit_index: usize,
}
