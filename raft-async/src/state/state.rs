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

pub trait StateMachine<In, Out>: Default + Send + 'static {
    fn apply(&mut self, command: In);
    fn get(&self) -> Out;
}

pub struct State<T: CommandType, SM> {
    pub state_machine: SM,
    pub persistent_state: PersistentState<T>,
    pub raft_state: RaftState,
    pub volitile_state: VolitileState,
}

impl<T: CommandType, SM: Default> State<T, SM> {
    pub fn new(id: u32, config: Config) -> Self {
        Self {
            state_machine: SM::default(),
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

    pub fn handle_request<Output>(
        mut self,
        request: Request<T, Output>,
    ) -> (Vec<Request<T, Output>>, Self)
    where
        SM: StateMachine<T, Output>,
    {
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

        let responses;
        (responses, self.raft_state) = self.raft_state.handle_request(
            request,
            &mut self.volitile_state,
            &mut self.persistent_state,
            &mut self.state_machine,
        );
        (responses, self)
    }
}
