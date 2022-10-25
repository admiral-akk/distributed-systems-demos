use std::time::Duration;

use crate::{
    data::{
        data_type::DataType,
        persistent_state::PersistentState,
        request::{
            Append, AppendResponse, Client, ClientResponse, Request, Timeout, Vote, VoteResponse,
        },
        volitile_state::VolitileState,
    },
    state::{
        handler::{EventHandler, Handler, TimeoutHandler},
        raft_state::RaftState,
    },
};

use super::follower::Follower;

pub struct Offline {}

// Does nothing. Only request it handles is timeout, which it assumes is a reboot request.
impl TimeoutHandler for Offline {
    fn timeout_length(&self) -> Duration {
        Duration::from_millis(1000)
    }
}

impl<T: DataType> Handler<T> for Offline {}

impl<T: DataType> EventHandler<Append<T>, T> for Offline {}
impl<T: DataType> EventHandler<Client<T>, T> for Offline {}
impl<T: DataType> EventHandler<ClientResponse<T>, T> for Offline {}

impl<T: DataType> EventHandler<Vote, T> for Offline {}

impl<T: DataType> EventHandler<VoteResponse, T> for Offline {}

impl<T: DataType> EventHandler<Timeout, T> for Offline {
    fn handle_event(
        &mut self,
        volitile_state: &mut VolitileState,
        persistent_state: &mut PersistentState<T>,
        _sender: u32,
        _term: u32,
        _event: Timeout,
    ) -> (Vec<Request<T>>, Option<RaftState>) {
        volitile_state.commit_index = 0;
        persistent_state.voted_for = None;
        persistent_state.keep_alive += 1;
        (Vec::new(), Some(Follower::default().into()))
    }
}

impl<T: DataType> EventHandler<AppendResponse, T> for Offline {}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use crate::data::entry::Entry;
    use crate::data::persistent_state::Config;
    use crate::data::request::{self, Event};
    // Note this useful idiom: importing names from outer (for mod tests) scope.
    use super::*;

    #[test]
    fn test_timeout() {
        let config = Config {
            servers: HashSet::from([0, 1, 2]),
        };
        let mut persistent_state: PersistentState<u32> = PersistentState {
            config,
            id: 1,
            current_term: 3,
            log: Vec::from([Entry { term: 1, data: 10 }, Entry { term: 2, data: 4 }]),
            ..Default::default()
        };
        let mut volitile_state = VolitileState::default();
        let mut follower = Offline {};
        let request: Request<u32> = Request {
            sender: 10,
            reciever: persistent_state.id,
            term: 0,
            event: Event::Timeout(request::Timeout),
        };

        let (requests, next) =
            follower.handle_request(&mut volitile_state, &mut persistent_state, request);

        assert!(next.is_some());
        if let Some(RaftState::Follower(_)) = next {
        } else {
            panic!("Didn't transition to follower!");
        }
        assert!(requests.is_empty());
        assert_eq!(persistent_state.voted_for, None);
        assert_eq!(persistent_state.keep_alive, 1);
        assert_eq!(volitile_state.commit_index, 0);
    }
}
