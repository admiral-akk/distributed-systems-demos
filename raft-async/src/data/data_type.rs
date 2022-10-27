use std::fmt::Debug;

pub trait CommandType: Clone + Debug + Send + Default + 'static {}
impl<T> CommandType for T where T: Clone + Debug + Default + Send + 'static {}

pub trait OutputType: Clone + Debug + Send + Default + 'static {}
impl<T> OutputType for T where T: Clone + Debug + Default + Send + 'static {}
