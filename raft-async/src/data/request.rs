use super::data_type::DataType;
use super::entry::Entry;

pub struct Request<T: DataType> {
    // Todo: figure out better framing for sender/reciever/term, since it's not relevant to all events.
    pub sender: u32,
    pub reciever: u32,
    pub term: u32,
    pub event: Event<T>,
}

pub enum Event<T: DataType> {
    Append(Append<T>),
    AppendResponse(AppendResponse),
    Vote(Vote),
    VoteResponse(VoteResponse),
    Timeout(Timeout),
    Client(Client<T>),
    ClientResponse(ClientResponse<T>),
}

pub struct Client<T: DataType> {
    pub data: T,
}

pub enum ClientResponse<T: DataType> {
    Failed { leader_id: u32, data: T },
    Success { data: T },
}

pub struct Append<T: DataType> {
    pub prev_log_length: usize,
    pub prev_log_term: u32,
    pub entries: Vec<Entry<T>>,
    pub leader_commit: usize,
}
pub struct AppendResponse {
    pub success: bool,
}
pub struct Vote {
    pub log_length: usize,
    pub last_log_term: u32,
}
pub struct VoteResponse {
    pub success: bool,
}
pub struct Timeout;
