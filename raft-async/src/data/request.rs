pub enum Request<T> {
    Append {
        sender: u32,
        term: u32,
        prev_log_index: usize,
        prev_log_term: u32,
        entries: Vec<T>,
        leader_commit: usize,
    },
    AppendResponse {
        sender: u32,
        term: u32,
        success: bool,
    },
    Vote {
        sender: u32,
        term: u32,
        prev_log_index: usize,
        prev_log_term: u32,
    },
    VoteResponse {
        sender: u32,
        term: u32,
        success: bool,
    },
}
