use std::time::Duration;

use crate::{
    data::{
        data_type::CommandType,
        persistent_state::{Config, PersistentState},
        request::Request,
        volitile_state::VolitileState,
    },
    state::concrete::follower::Follower,
};

use super::{concrete::offline::Offline, raft_state::RaftState};

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
                ..Default::default()
            },
            raft_state: RaftState::default(),
            volitile_state: VolitileState::default(),
        }
    }

    pub fn shutdown(&mut self) {
        println!("Server {} crashed", self.persistent_state.id);
        self.raft_state = RaftState::Offline(Offline {});
    }

    pub fn timeout_length(&self) -> Duration {
        self.raft_state.timeout_length()
    }

    pub fn handle_request(&mut self, request: Request<T>) -> Vec<Request<T>> {
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
        if let Some(next) = next {
            self.raft_state = next;
        }
        responses
    }
}
