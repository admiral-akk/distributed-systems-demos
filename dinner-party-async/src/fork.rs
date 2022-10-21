use async_std::sync::Arc;
use futures::lock::Mutex;
pub struct Fork {}

impl Fork {
    pub fn new() -> Arc<Mutex<Self>> {
        Arc::new(Mutex::new(Fork {}))
    }
}
