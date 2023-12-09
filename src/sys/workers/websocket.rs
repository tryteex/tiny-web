use crate::sys::{
    log::Log,
    worker::{StreamRead, StreamWrite, WorkerData},
};

/// WebSocket protocol
pub struct Net;

impl Net {
    pub async fn run(mut _stream_read: StreamRead, mut _stream_write: StreamWrite, _data: WorkerData) {
        Log::warning(3, Some("WebSocket".to_owned()));
    }
}
