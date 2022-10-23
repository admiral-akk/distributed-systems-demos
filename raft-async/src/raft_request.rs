#[derive(Copy, Clone, Debug)]
pub struct RaftRequest {
    pub term: u32,
    pub sender: u32,
    pub request: RequestType,
}

#[derive(Copy, Clone, Debug)]
pub enum RequestType {
    Append {},
    AppendResponse { success: bool },
    Vote { log_length: usize, log_term: u32 },
    VoteResponse { vote: bool },
}
