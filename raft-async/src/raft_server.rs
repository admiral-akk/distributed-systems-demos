pub enum RaftState {
    Offline,
    Follower,
    Candidate,
    Leader,
}

#[derive(Default)]
pub struct PersistentState<T: Default> {
    current_term: u32,
    voted_for: Option<u32>,
    log: Vec<T>,
}

#[derive(Default)]
pub struct VolitileState {
    commit_index: usize,
    last_applied: usize,
}

#[derive(Default)]
pub struct VolitileLeaderState {
    next_index: Vec<usize>,
    match_index: Vec<usize>,
}

pub struct ServerState<T: Default> {
    state: RaftState,
    id: u32,
    persistent_state: PersistentState<T>,
    volitile_state: VolitileState,
    leader_state: Option<VolitileLeaderState>,
}

impl<T: Default> ServerState<T> {
    pub fn new(id: u32) -> Self {
        Self {
            state: RaftState::Offline,
            id,
            persistent_state: PersistentState::<T>::default(),
            volitile_state: VolitileState::default(),
            leader_state: None,
        }
    }
}
