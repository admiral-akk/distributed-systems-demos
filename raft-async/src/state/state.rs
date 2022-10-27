use crate::{
    data::{
        data_type::CommandType,
        persistent_state::{Config, PersistentState},
        request::Request,
        volitile_state::VolitileState,
    },
    state::concrete::follower::Follower,
};

use super::raft_state::RaftState;

pub struct State<T: CommandType> {
    pub persistent_state: PersistentState<T>,
    pub raft_state: RaftState,
    pub volitile_state: VolitileState,
}

impl<T: CommandType> State<T> {
    pub fn new(id: u32, config: Config) -> Self {
        Self {
            persistent_state: PersistentState {
                id,
                config,
                current_term: 0,
                voted_for: None,
                log: Vec::new(),
            },
            raft_state: RaftState::default(),
            volitile_state: VolitileState::default(),
        }
    }

    pub fn handle_request(mut self, request: Request<T>) -> (Self, Vec<Request<T>>) {
        // Todo: Maybe find better place to put this, since State shouldn't be aware of how each state updates.
        match self.raft_state {
            RaftState::Offline(_) => {}
            _ => {
                if request.term > self.persistent_state.current_term {
                    println!(
                        "New term, {} reverting to follower.",
                        self.persistent_state.id
                    );
                    self.persistent_state.current_term = request.term;
                    self.persistent_state.voted_for = None;
                    self.raft_state = RaftState::Follower(Follower::default());
                }
            }
        }

        let (responses, next) = self.raft_state.handle_request(
            request,
            &mut self.volitile_state,
            &mut self.persistent_state,
        );
        self.raft_state = next;
        (self, responses)
    }
}
