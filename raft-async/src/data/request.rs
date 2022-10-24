use super::data_type::DataType;
use super::entry::Entry;

#[derive(Clone)]
pub struct Request<T: DataType> {
    pub sender: u32,
    pub reciever: u32,
    pub term: u32,
    pub data: RequestType<T>,
}

#[derive(Clone)]
pub enum RequestType<T: DataType> {
    Append {
        prev_log_length: usize,
        prev_log_term: u32,
        entries: Vec<Entry<T>>,
        leader_commit: usize,
    },
    AppendResponse {
        success: bool,
    },
    Vote {
        log_length: usize,
        last_log_term: u32,
    },
    VoteResponse {
        success: bool,
    },
    Bootup {},
}
