use crate::data::{
    data_type::CommandType,
    persistent_state::PersistentState,
    request::{Event, Request},
    volitile_state::VolitileState,
};

use super::{concrete::offline::Offline, raft_state::RaftState};

pub trait EventHandler
where
    Self: Into<RaftState>,
{
    fn handle<T: CommandType>(
        self,
        _volitile_state: &mut VolitileState,
        _persistent_state: &mut PersistentState<T>,
        _sender: u32,
        _term: u32,
        _request: Request<T>,
    ) -> (Vec<Request<T>>, RaftState) {
        (Vec::default(), self.into())
    }
}

pub trait Handler: EventHandler {
    fn handle_request<T: CommandType>(
        self,
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
