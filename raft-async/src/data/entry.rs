use super::data_type::CommandType;

#[derive(Clone, Copy, PartialEq)]
pub struct Entry<T: CommandType> {
    pub term: u32,
    pub command: T,
}
