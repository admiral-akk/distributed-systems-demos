use std::collections::HashMap;

use crate::{
    data::{
        data_type::CommandType,
        persistent_state::{Entry, LatestConfig, PersistentState},
        request::{
            ActiveConfig, Client, ClientData, ClientResponse, Event, InsertResponse, Request, Tick,
            Transaction,
        },
        volitile_state::VolitileState,
    },
    server::raft_cluster::Id,
    state::state::StateMachine,
};
use crate::{
    data::{data_type::OutputType, request::Data},
    state::{
        handler::{EventHandler, Handler},
        raft_state::RaftState,
    },
};

use super::{candidate::Candidate, follower::Follower};

#[derive(Debug, PartialEq, Clone)]
pub struct Leader {
    pub next_index: HashMap<Id, usize>,
    pub match_index: HashMap<Id, usize>,
}

impl Leader {
    pub fn send_heartbeat<T: CommandType, Output>(
        self,
        volitile_state: &mut VolitileState,
        persistent_state: &mut PersistentState<T>,
    ) -> (Vec<Request<T, Output>>, RaftState) {
        let latest_config = persistent_state
            .latest_config(volitile_state.commit_index)
            .1;

        // If we recently committed a configuration that removes this server, it demotes itself.
        if latest_config.committed {
            if !latest_config
                .config
                .servers()
                .contains(&persistent_state.id)
            {
                return (Vec::new(), Follower.into());
            }

            if let ActiveConfig::Transition { new, .. } = latest_config.config {
                persistent_state.log.push(Entry::stable_config(
                    persistent_state.current_term,
                    latest_config.transaction_id,
                    new.clone(),
                ));
            }
        }

        // If we recently committed a configuration that transitions between two configurations, append the new configuration on its own.

        (
            persistent_state
                .other_servers(volitile_state.commit_index)
                .iter()
                .map(|server| self.append_update(volitile_state, persistent_state, *server))
                .collect(),
            self.into(),
        )
    }

    fn append_update<T: CommandType, Output>(
        &self,
        volitile_state: &mut VolitileState,
        persistent_state: &mut PersistentState<T>,
        server: Id,
    ) -> Request<T, Output> {
        Request {
            sender: persistent_state.id,
            reciever: server,
            term: persistent_state.current_term,
            event: persistent_state.insert(
                self.next_index[&server],
                1,
                volitile_state.get_commit_index(),
            ),
        }
    }

    pub fn from_candidate<T: CommandType, Output>(
        _candidate: Candidate,
        volitile_state: &mut VolitileState,
        persistent_state: &mut PersistentState<T>,
    ) -> (Vec<Request<T, Output>>, RaftState) {
        persistent_state.current_term += 1;
        volitile_state.tick_since_start = 0;
        println!("{:?} elected leader!", persistent_state.id);

        let latest_config = persistent_state
            .latest_config(volitile_state.commit_index)
            .1;

        // If we recently committed a configuration that removes this server, it demotes itself.
        if latest_config.committed {
            if let ActiveConfig::Transition { new, .. } = latest_config.config {
                persistent_state.log.push(Entry::stable_config(
                    persistent_state.current_term,
                    latest_config.transaction_id,
                    new.clone(),
                ));
            }
        }
        let other_servers = persistent_state.other_servers(volitile_state.commit_index);
        let leader = Leader {
            next_index: persistent_state
                .other_servers(volitile_state.commit_index)
                .iter()
                .map(|id| (*id, persistent_state.log.len()))
                .collect(),
            match_index: other_servers.iter().map(|id| (*id, 0)).collect(),
        };
        leader.send_heartbeat(volitile_state, persistent_state)
    }
}

impl Handler for Leader {}
impl EventHandler for Leader {
    fn handle<T: CommandType, Output: OutputType, SM>(
        mut self,
        volitile_state: &mut VolitileState,
        persistent_state: &mut PersistentState<T>,
        state_machine: &mut SM,
        sender: Id,
        _term: u32,
        request: Request<T, Output>,
    ) -> (Vec<Request<T, Output>>, RaftState)
    where
        SM: StateMachine<T, Output>,
    {
        match request.event {
            Event::Client(Client { id, data }) => {
                // If we are not in a stable config state, we cannot transition.
                match &data {
                    ClientData::Command(command) => {
                        persistent_state.push(Transaction {
                            id,
                            data: Data::Command(command.clone()),
                        });
                        (Vec::new(), self.into())
                    }
                    ClientData::Config(config) => {
                        let latest_config = persistent_state
                            .latest_config(volitile_state.commit_index)
                            .1;
                        match latest_config {
                            LatestConfig {
                                config: ActiveConfig::Stable(old_config),
                                committed: true,
                                ..
                            } => {
                                persistent_state.push(Transaction {
                                    id,
                                    data: Data::Config(ActiveConfig::Transition {
                                        prev: old_config.clone(),
                                        new: config.clone(),
                                    }),
                                });
                                (Vec::new(), self.into())
                            }
                            _ => {
                                // fails, we're in the midst of a migration.
                                (
                                    [Request {
                                        sender: persistent_state.id,
                                        reciever: sender,
                                        term: 0,
                                        event: Event::ClientResponse(ClientResponse::Failed {
                                            id,
                                            leader_id: Some(persistent_state.id),
                                            data,
                                        }),
                                    }]
                                    .into(),
                                    self.into(),
                                )
                            }
                        }
                    }
                }
            }
            Event::Tick(Tick) => self.send_heartbeat(volitile_state, persistent_state),
            Event::InsertResponse(InsertResponse { success }) => {
                let next_index = self.next_index[&sender];
                if success {
                    if next_index < persistent_state.log.len() {
                        self.next_index.insert(sender, next_index + 1);
                    }
                    self.match_index.insert(sender, self.next_index[&sender]);
                } else if next_index > 0 {
                    self.next_index.insert(sender, next_index - 1);
                }
                let matching_servers = self
                    .match_index
                    .iter()
                    .filter(|(_, v)| **v >= next_index)
                    .map(|(id, _)| *id)
                    .collect();

                if persistent_state.has_quorum(volitile_state.commit_index, &matching_servers) {
                    if volitile_state.try_update_commit_index(
                        state_machine,
                        persistent_state,
                        next_index,
                    ) {
                        println!(
                            "Leader {} index committed, value: {:?}!",
                            next_index,
                            state_machine.get()
                        );
                    }
                }
                (Vec::default(), self.into())
            }
            _ => (Vec::default(), self.into()),
        }
    }
}

#[cfg(test)]
pub mod test_util {
    use std::collections::HashSet;

    use super::Leader;
    use crate::{
        data::persistent_state::test_util::CONFIG,
        server::raft_cluster::{test_util::SERVER_1, Id},
        state::raft_state::RaftState,
    };

    pub fn BASE_LEADER(log_length: usize, match_index: usize) -> RaftState {
        BASE_LEADER_WITH_RANGE(log_length, match_index, CONFIG().servers)
    }

    pub fn BASE_LEADER_WITH_RANGE(
        log_length: usize,
        match_index: usize,
        servers: HashSet<Id>,
    ) -> RaftState {
        RaftState::Leader(super::Leader {
            next_index: servers
                .iter()
                .filter(|id| !(*id).eq(&SERVER_1))
                .map(|id| (*id, log_length))
                .collect(),
            match_index: servers
                .iter()
                .filter(|id| !(*id).eq(&SERVER_1))
                .map(|id| (*id, match_index))
                .collect(),
        })
    }

    impl RaftState {
        pub fn set_next_index(mut self, id: Id, index: usize) -> Self {
            match &mut self {
                RaftState::Leader(Leader { next_index, .. }) => {
                    next_index.insert(id, index);
                }
                _ => {}
            }
            self
        }
        pub fn set_match_index(mut self, id: Id, index: usize) -> Self {
            match &mut self {
                RaftState::Leader(Leader { match_index, .. }) => {
                    match_index.insert(id, index);
                }
                _ => {}
            }
            self
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::data::persistent_state::test_util::{
        JOINT_CONFIG, LOG_TRANSITION_STABLE_CONFIG, LOG_WITH_CLIENT,
    };
    use crate::data::request::test_util::{
        CLIENT_COMMAND, INSERT_FAILED_RESPONSE, INSERT_SUCCESS_RESPONSE, MASS_HEARTBEAT, TICK,
    };
    use crate::server::raft_cluster::test_util::{SERVER_0, SERVER_4, SERVER_5, SERVER_6};
    use crate::state::concrete::follower::test_util::FOLLOWER;
    use crate::state::concrete::leader::test_util::BASE_LEADER;
    use crate::state::state::test_util::TestCase;
    use crate::state::state::State;

    use super::test_util::BASE_LEADER_WITH_RANGE;

    #[test]
    fn test_tick() {
        let state = State::create_state(BASE_LEADER(3, 2));
        let mut test_case = TestCase::new(state, TICK)
            .increment_tick()
            .responses(&MASS_HEARTBEAT(4));
        test_case.run();
    }

    #[test]
    fn test_append_response_success() {
        let state = State::create_state(BASE_LEADER(3, 2)).set_next_index(SERVER_0, 2);
        let mut test_case = TestCase::new(state, INSERT_SUCCESS_RESPONSE)
            .set_rs(BASE_LEADER(3, 2).set_match_index(SERVER_0, 3));
        test_case.run();
    }

    #[test]
    fn test_append_response_succeeds_up_to_date() {
        let state = State::create_state(BASE_LEADER(3, 2));
        let mut test_case =
            TestCase::new(state, INSERT_SUCCESS_RESPONSE).set_match_index(SERVER_0, 3);
        test_case.run();
    }

    #[test]
    fn test_append_response_fails() {
        let state = State::create_state(BASE_LEADER(3, 2));
        let mut test_case =
            TestCase::new(state, INSERT_FAILED_RESPONSE).set_next_index(SERVER_0, 2);
        test_case.run();
    }

    #[test]
    fn test_client_request() {
        let state = State::create_state(BASE_LEADER(3, 2));
        let mut test_case = TestCase::new(state, CLIENT_COMMAND).set_log(LOG_WITH_CLIENT());
        test_case.run();
    }

    #[test]
    fn test_new_config_commit_no_quorum() {
        let state = State::create_state(
            BASE_LEADER_WITH_RANGE(3, 3, JOINT_CONFIG().servers)
                .set_match_index(SERVER_0, 2)
                .set_match_index(SERVER_4, 2)
                .set_match_index(SERVER_5, 2)
                .set_match_index(SERVER_6, 2),
        )
        .set_log(LOG_TRANSITION_STABLE_CONFIG());
        let mut test_case =
            TestCase::new(state, INSERT_SUCCESS_RESPONSE).set_match_index(SERVER_0, 3);
        test_case.run();
    }

    #[test]
    fn test_new_config_commit_with_quorum() {
        let state = State::create_state(
            BASE_LEADER_WITH_RANGE(3, 3, JOINT_CONFIG().servers)
                .set_match_index(SERVER_0, 2)
                .set_match_index(SERVER_5, 2)
                .set_match_index(SERVER_6, 2),
        )
        .set_log(LOG_TRANSITION_STABLE_CONFIG());
        let mut test_case = TestCase::new(state, INSERT_SUCCESS_RESPONSE.set_sender(SERVER_6))
            .set_match_index(SERVER_6, 3)
            .set_commit(3);
        test_case.run();
    }

    #[test]
    fn test_new_config_demotion() {
        let state = State::create_state(BASE_LEADER_WITH_RANGE(3, 3, JOINT_CONFIG().servers))
            .set_commit(3)
            .set_log(LOG_TRANSITION_STABLE_CONFIG());
        let mut test_case = TestCase::new(state, TICK).set_rs(FOLLOWER).increment_tick();
        test_case.run();
    }
}
