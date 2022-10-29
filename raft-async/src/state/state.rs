use std::fmt::Debug;

use crate::{
    data::{
        data_type::{CommandType, OutputType},
        persistent_state::{Config, Entry, PersistentState},
        request::Request,
        volitile_state::VolitileState,
    },
    state::concrete::follower::Follower,
};

use super::raft_state::RaftState;

pub trait StateMachine<In, Out>: Default + Send + Debug + PartialEq + Clone + Eq + 'static {
    fn apply(&mut self, command: In);
    fn get(&self) -> Out;
}

#[derive(Debug, PartialEq, Clone)]
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
            volitile_state: VolitileState {
                commit_index: 1,
                ..Default::default()
            },
            persistent_state: PersistentState {
                id,
                current_term: 0,
                voted_for: None,
                log: [Entry::config(0, config.clone())].into(),
            },
            raft_state: RaftState::default(),
        }
    }

    pub fn handle_request<Output: OutputType>(
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

#[cfg(test)]
pub mod test_util {
    use crate::{
        data::{
            data_type::{CommandType, OutputType},
            persistent_state::PersistentState,
            request::Request,
            volitile_state::VolitileState,
        },
        state::raft_state::RaftState,
    };
    use pretty_assertions::assert_eq;

    use super::{State, StateMachine};

    pub fn create_state<In: CommandType, SM>(
        state_machine: SM,
        persistent_state: PersistentState<In>,
        raft_state: RaftState,
        volitile_state: VolitileState,
    ) -> State<In, SM> {
        State {
            state_machine,
            persistent_state,
            raft_state,
            volitile_state,
        }
    }

    pub struct TestCase<In: CommandType, Out: OutputType, SM: StateMachine<In, Out>> {
        pub state: State<In, SM>,
        pub request: Request<In, Out>,
        pub expected_state: State<In, SM>,
        pub expected_responses: Vec<Request<In, Out>>,
        pub name: String,
    }

    impl<In: CommandType, Out: OutputType, SM: StateMachine<In, Out>> TestCase<In, Out, SM> {
        pub fn new(state: State<In, SM>, request: Request<In, Out>, name: &str) -> Self {
            Self {
                expected_state: state.clone(),
                expected_responses: Vec::new(),
                request,
                state,
                name: name.to_string(),
            }
        }
        pub fn responses(mut self, responses: &[Request<In, Out>]) -> Self {
            self.expected_responses = responses.into();
            self
        }

        pub fn set_rs(mut self, raft_state: RaftState) -> Self {
            self.expected_state.raft_state = raft_state;
            self
        }

        pub fn set_vs(mut self, volitile_state: VolitileState) -> Self {
            self.expected_state.volitile_state = volitile_state;
            self
        }

        pub fn run(&mut self) {
            let responses;
            (responses, self.state) = self.state.clone().handle_request(self.request.clone());
            assert_eq!(
                self.state, self.expected_state,
                "State mismatch for case: {}",
                self.name
            );
            assert_eq!(
                responses, self.expected_responses,
                "Response mismatch for case: {}",
                self.name
            );
        }
    }
}
