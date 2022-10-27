use crate::data::{
    data_type::CommandType,
    persistent_state::PersistentState,
    request::{
        self, Client, ClientResponse, Crash, Event, Insert, InsertResponse, Request, Tick,
        VoteResponse,
    },
    volitile_state::VolitileState,
};

use super::{concrete::offline::Offline, raft_state::RaftState};

pub trait EventHandler
where
    Self: Into<RaftState>,
{
    fn handle<T: CommandType>(
        mut self,
        volitile_state: &mut VolitileState,
        persistent_state: &mut PersistentState<T>,
        sender: u32,
        term: u32,
        request: Request<T>,
    ) -> (Vec<Request<T>>, RaftState) {
        (Vec::default(), self.into())
    }
}

pub trait Handler: EventHandler {
    fn handle_request<T: CommandType>(
        mut self,
        volitile_state: &mut VolitileState,
        persistent_state: &mut PersistentState<T>,
        request: Request<T>,
    ) -> (Vec<Request<T>>, RaftState) {
        let (sender, term) = (request.sender, request.term);
        match request.event {
            Event::Tick(_) => {
                volitile_state.tick_since_start += 1;
            }
            Event::Crash(_) => {
                return (Vec::default(), Offline.into());
            }
            _ => {}
        }

        self.handle(volitile_state, persistent_state, sender, term, request)
    }
}
