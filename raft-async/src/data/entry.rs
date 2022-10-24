use super::data_type::DataType;

#[derive(Clone, Copy)]
pub struct Entry<T: DataType> {
    pub term: u32,
    pub data: T,
}
