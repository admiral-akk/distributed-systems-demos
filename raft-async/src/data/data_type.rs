use std::fmt::Debug;

pub trait DataType: Copy + Clone + Debug + Send + 'static {}
impl<T> DataType for T where T: Copy + Clone + Debug + Send + 'static {}
