use super::data_type::DataType;
use super::entry::Entry;

pub struct Request<T: DataType> {
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
