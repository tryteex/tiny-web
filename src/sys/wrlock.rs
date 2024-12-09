use std::sync::{atomic::AtomicBool, Arc};

use tokio::sync::Notify;

#[derive(Debug)]
pub(crate) struct WrLock {
    pub lock: AtomicBool,
    pub notify: Arc<Notify>,
}

impl Default for WrLock {
    fn default() -> WrLock {
        WrLock {
            lock: AtomicBool::new(false),
            notify: Arc::new(Notify::new()),
        }
    }
}
