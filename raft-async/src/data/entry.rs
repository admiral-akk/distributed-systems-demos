use super::data_type::CommandType;

#[derive(Clone, PartialEq)]
pub struct Entry<T: Clone> {
    pub term: u32,
    pub command: T,
}
