use crate::log_entry::LogEntry;

pub enum RaftRequest<T: Default + Copy> {
    AppendEntries(AppendEntries<T>),
    RequestVote(RequestVote),
    Response(Response),
}

pub struct RequestVote {
    pub term: u32,
    pub candidateId: u32,
    pub lastLogIndex: u32,
    pub lastLogTerm: u32,
}

pub struct Response {
    pub term: u32,
    pub success: bool,
}

pub struct AppendEntries<T: Default + Copy> {
    pub term: u32,
    pub leaderId: u32,
    pub prevLogIndex: usize,
    pub prevLogTerm: u32,
    pub entries: Vec<LogEntry<T>>,
}
