use std::fmt::Debug;

pub trait CommandType: Clone + Debug + Send + Default + PartialEq + 'static {}
impl<T> CommandType for T where T: Clone + Debug + Default + PartialEq + Send + 'static {}

pub trait OutputType: Clone + Debug + Send + PartialEq + Default + 'static {}
impl<T> OutputType for T where T: Clone + Debug + PartialEq + Default + Send + 'static {}
