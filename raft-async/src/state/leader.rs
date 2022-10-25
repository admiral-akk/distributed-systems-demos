use std::{collections::HashMap, time::SystemTime};

use async_std::channel::Sender;

use crate::data::{
    data_type::DataType,
    persistent_state::PersistentState,
    request::{Request, RequestType},
    volitile_state::VolitileState,
};

use super::{
    candidate::Candidate,
    follower::Follower,
    raft_state::{Handler, RaftState},
};

pub struct Leader {
    pub next_index: HashMap<u32, usize>,
    pub match_index: HashMap<u32, usize>,
}

impl Leader {}

impl Leader {
    pub fn send_heartbeat<T: DataType>(
        &self,
        volitile_state: &mut VolitileState,
        persistent_state: &mut PersistentState<T>,
    ) -> Vec<Request<T>> {
        persistent_state
            .other_servers()
            .iter()
            .map(|server| self.append_update(volitile_state, persistent_state, *server))
            .collect()
    }

    fn append_update<T: DataType>(
        &self,
        volitile_state: &mut VolitileState,
        persistent_state: &mut PersistentState<T>,
        server: u32,
    ) -> Request<T> {
        let next_index = self.match_index[&server];
        // Append at most 10 elements
        let entries = match persistent_state.log.is_empty() {
            false => Vec::from(
                &persistent_state.log
                    [(next_index - 1)..(next_index).min(persistent_state.log.len())],
            ),
            true => Vec::new(),
        };
        let data = RequestType::Append {
            prev_log_length: next_index,
            prev_log_term: match next_index > 0 {
                true => persistent_state.log[next_index - 1].term,
                false => 0,
            },
            entries,
            leader_commit: volitile_state.commit_index,
        };
        Request {
            sender: persistent_state.id,
            reciever: server,
            term: persistent_state.current_term,
            data,
        }
    }

    pub fn from_candidate<T: DataType>(
        candidate: &Candidate,
        volitile_state: &mut VolitileState,
        persistent_state: &mut PersistentState<T>,
    ) -> (Vec<Request<T>>, Option<RaftState>) {
        persistent_state.current_term += 1;
        println!("{} elected leader!", persistent_state.id);
        let leader = Leader {
            next_index: persistent_state
                .other_servers()
                .iter()
                .map(|id| (*id, persistent_state.log.len()))
                .collect(),
            match_index: persistent_state
                .other_servers()
                .iter()
                .map(|id| (*id, 0))
                .collect(),
        };
        let heartbeat = leader.send_heartbeat(volitile_state, persistent_state);
        (heartbeat, Some(leader.into()))
    }
}

impl<T: DataType> Handler<T> for Leader {
    fn append_response(
        &mut self,
        volitile_state: &mut VolitileState,
        persistent_state: &mut PersistentState<T>,
        sender: u32,
        term: u32,
        success: bool,
    ) -> (Vec<Request<T>>, Option<RaftState>) {
        let next_index = self.next_index[&sender];
        if success {
            self.match_index.insert(sender, next_index);
            self.next_index.insert(sender, next_index + 1);
        } else if next_index > 0 {
            self.next_index.insert(sender, next_index - 1);
        }
        let matching_servers = self
            .match_index
            .iter()
            .filter(|(_, v)| **v >= next_index)
            .count();

        if matching_servers + 1 > persistent_state.quorum() {
            volitile_state.commit_index = next_index.max(volitile_state.commit_index);
        }
        (Vec::default(), None)
    }
}
