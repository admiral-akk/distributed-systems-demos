use std::sync::{Arc, Mutex};

pub struct Fork {}

impl Fork {
    pub fn new() -> Arc<Mutex<Self>> {
        Arc::new(Mutex::new(Fork {}))
    }
}
