pub struct Request<T> {
    pub sender: u32,
    pub term: u32,
    pub data: RequestType<T>,
}

pub enum RequestType<T> {
    Append {
        prev_log_index: usize,
        prev_log_term: u32,
        entries: Vec<T>,
        leader_commit: usize,
    },
    AppendResponse {
        success: bool,
    },
    Vote {
        prev_log_index: usize,
        prev_log_term: u32,
    },
    VoteResponse {
        success: bool,
    },
}
