use crate::sys::{
    log::Log,
    worker::{StreamRead, StreamWrite, WorkerData},
};

/// HTTP protocol
pub struct Net;

impl Net {
    pub async fn run(mut _stream_read: StreamRead, mut _stream_write: StreamWrite, _data: WorkerData) {
        Log::warning(3, Some("HTTP".to_owned()));
    }
}
