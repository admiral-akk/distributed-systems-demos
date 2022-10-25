use std::fmt::Debug;

pub trait DataType: Copy + Clone + Debug + Send + Default + PartialEq + 'static {}
impl<T> DataType for T where T: Copy + Clone + Debug + Default + Send + PartialEq + 'static {}
