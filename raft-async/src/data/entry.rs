use super::data_type::DataType;

#[derive(Clone, Copy, PartialEq)]
pub struct Entry<T: DataType> {
    pub term: u32,
    pub data: T,
}
