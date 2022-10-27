use crate::data::{
    data_type::{CommandType, OutputType},
    persistent_state::PersistentState,
    request::{Event, Request},
    volitile_state::VolitileState,
};

use super::{concrete::offline::Offline, raft_state::RaftState, state::StateMachine};

pub trait EventHandler
where
    Self: Into<RaftState>,
{
    fn handle<T: CommandType, Output: OutputType, SM>(
        self,
        _volitile_state: &mut VolitileState,
        _persistent_state: &mut PersistentState<T>,
        _state_machine: &mut SM,
        _sender: u32,
        _term: u32,
        _request: Request<T, Output>,
    ) -> (Vec<Request<T, Output>>, RaftState)
    where
        SM: StateMachine<T, Output>,
    {
        (Vec::default(), self.into())
    }
}

pub trait Handler: EventHandler {
    fn handle_request<T: CommandType, Output: OutputType, SM>(
        self,
        volitile_state: &mut VolitileState,
        persistent_state: &mut PersistentState<T>,
        state_machine: &mut SM,
        request: Request<T, Output>,
    ) -> (Vec<Request<T, Output>>, RaftState)
    where
        SM: StateMachine<T, Output>,
    {
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

        self.handle(
            volitile_state,
            persistent_state,
            state_machine,
            sender,
            term,
            request,
        )
    }
}
