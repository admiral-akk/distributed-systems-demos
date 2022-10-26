use std::fmt::Debug;

pub trait CommandType: Copy + Clone + Debug + Send + Default + PartialEq + 'static {}
impl<T> CommandType for T where T: Copy + Clone + Debug + Default + Send + PartialEq + 'static {}
