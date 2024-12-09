use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};

#[derive(Debug)]
pub struct Stat {
    /// Number of workers
    pub(crate) number: Arc<AtomicU64>,
    /// Last worker ID
    pub(crate) worker: Arc<AtomicU64>,
    /// Number of online requests
    pub(crate) online: Arc<AtomicU64>,
    ///  Number of total requests
    pub(crate) total: Arc<AtomicU64>,
}

impl Stat {
    pub(crate) fn new() -> Stat {
        Stat {
            number: Arc::new(AtomicU64::new(0)),
            worker: Arc::new(AtomicU64::new(0)),
            online: Arc::new(AtomicU64::new(0)),
            total: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Number of workers
    pub fn get_number(&self) -> u64 {
        self.number.load(Ordering::Relaxed)
    }

    /// Last worker ID
    pub fn get_last(&self) -> u64 {
        self.worker.load(Ordering::Relaxed)
    }

    /// Number of online requests
    pub fn get_online(&self) -> u64 {
        self.online.load(Ordering::Relaxed)
    }

    /// Number of total requests
    pub fn get_total(&self) -> u64 {
        self.total.load(Ordering::Relaxed)
    }
}
