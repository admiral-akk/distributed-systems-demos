use std::time::Duration;

use crate::data::{
    data_type::CommandType,
    persistent_state::PersistentState,
    request::{
        Append, AppendResponse, Client, ClientResponse, Event, Request, Timeout, Vote, VoteResponse,
    },
    volitile_state::VolitileState,
};

use super::raft_state::RaftState;

pub trait TimeoutHandler {
    fn timeout_length(&self) -> Duration;
}

pub trait EventHandler<EventType, T: CommandType> {
    fn handle_event(
        &mut self,
        _volitile_state: &mut VolitileState,
        _persistent_state: &mut PersistentState<T>,
        _sender: u32,
        _term: u32,
        _event: EventType,
    ) -> (Vec<Request<T>>, Option<RaftState>) {
        (Vec::default(), None)
    }
}

pub trait Handler<T: CommandType>:
    EventHandler<Append<T>, T>
    + EventHandler<AppendResponse, T>
    + EventHandler<Timeout, T>
    + EventHandler<Vote, T>
    + EventHandler<VoteResponse, T>
    + EventHandler<Client<T>, T>
    + EventHandler<ClientResponse<T>, T>
{
    fn handle_request(
        &mut self,
        volitile_state: &mut VolitileState,
        persistent_state: &mut PersistentState<T>,
        request: Request<T>,
    ) -> (Vec<Request<T>>, Option<RaftState>) {
        let (sender, term) = (request.sender, request.term);
        match request.event {
            Event::Append(event) => {
                self.handle_event(volitile_state, persistent_state, sender, term, event)
            }
            Event::AppendResponse(event) => {
                self.handle_event(volitile_state, persistent_state, sender, term, event)
            }
            Event::Vote(event) => {
                self.handle_event(volitile_state, persistent_state, sender, term, event)
            }
            Event::VoteResponse(event) => {
                self.handle_event(volitile_state, persistent_state, sender, term, event)
            }
            Event::Timeout(event) => {
                self.handle_event(volitile_state, persistent_state, sender, term, event)
            }
            Event::Client(event) => {
                self.handle_event(volitile_state, persistent_state, sender, term, event)
            }
            Event::ClientResponse(event) => {
                self.handle_event(volitile_state, persistent_state, sender, term, event)
            }
        }
    }
}
