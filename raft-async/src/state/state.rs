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
            persistent_state::{test_util::PERSISTENT_STATE, Entry, PersistentState},
            request::Request,
            volitile_state::{test_util::VOLITILE_STATE, VolitileState},
        },
        state::raft_state::RaftState,
        Sum,
    };
    use pretty_assertions::assert_eq;

    use super::State;

    impl State<u32, Sum> {
        pub fn create_state(raft_state: RaftState) -> State<u32, Sum> {
            State {
                state_machine: Sum { total: 10 },
                persistent_state: PERSISTENT_STATE(),
                raft_state,
                volitile_state: VOLITILE_STATE,
            }
        }

        pub fn set_voted(mut self, voted_for: u32) -> Self {
            self.persistent_state = self.persistent_state.set_voted(voted_for);
            self
        }

        pub fn set_rs(mut self, raft_state: RaftState) -> Self {
            self.raft_state = raft_state;
            self
        }

        pub fn set_ps(mut self, persistent_state: PersistentState<u32>) -> Self {
            self.persistent_state = persistent_state;
            self
        }

        pub fn set_log(mut self, log: Vec<Entry<u32>>) -> Self {
            self.persistent_state.log = log;
            self
        }

        pub fn set_commit(mut self, commit_index: usize) -> Self {
            self.volitile_state.commit_index = commit_index;
            self
        }
        pub fn set_vs(mut self, volitile_state: VolitileState) -> Self {
            self.volitile_state = volitile_state;
            self
        }

        pub fn set_sum(mut self, total: u32) -> Self {
            self.state_machine.total = total;
            self
        }
    }

    pub struct TestCase {
        state: State<u32, Sum>,
        request: Request<u32, u32>,
        expected_state: State<u32, Sum>,
        expected_responses: Vec<Request<u32, u32>>,
    }

    impl TestCase {
        pub fn new(state: State<u32, Sum>, request: Request<u32, u32>) -> Self {
            Self {
                expected_state: state.clone(),
                expected_responses: Vec::new(),
                request,
                state,
            }
        }
        pub fn responses(mut self, responses: &[Request<u32, u32>]) -> Self {
            self.expected_responses = responses.into();
            self.expected_responses
                .sort_by(|r_1, r_2| r_1.reciever.cmp(&r_2.reciever));
            self
        }

        pub fn set_rs(mut self, raft_state: RaftState) -> Self {
            self.expected_state = self.expected_state.set_rs(raft_state);
            self
        }

        pub fn set_voted(mut self, voted_for: u32) -> Self {
            self.expected_state.persistent_state.voted_for = Some(voted_for);
            self
        }

        pub fn set_ps(mut self, persistent_state: PersistentState<u32>) -> Self {
            self.expected_state = self.expected_state.set_ps(persistent_state);
            self
        }

        pub fn set_log(mut self, log: Vec<Entry<u32>>) -> Self {
            self.expected_state = self.expected_state.set_log(log);
            self
        }
        pub fn set_vs(mut self, volitile_state: VolitileState) -> Self {
            self.expected_state = self.expected_state.set_vs(volitile_state);
            self
        }

        pub fn set_sum(mut self, total: u32) -> Self {
            self.expected_state = self.expected_state.set_sum(total);
            self
        }

        pub fn set_commit(mut self, commit_index: usize) -> Self {
            self.expected_state = self.expected_state.set_commit(commit_index);
            self
        }

        pub fn run(&mut self) {
            let mut responses;
            (responses, self.state) = self.state.clone().handle_request(self.request.clone());

            responses.sort_by(|r_1, r_2| r_1.reciever.cmp(&r_2.reciever));
            assert_eq!(self.state, self.expected_state);
            assert_eq!(responses, self.expected_responses);
        }
    }
}
