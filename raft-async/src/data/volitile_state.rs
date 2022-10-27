#[derive(Default, Clone, Copy)]
pub struct VolitileState {
    pub commit_index: usize,
    pub tick_since_start: u32,
}
