use std::sync::Arc;

use crate::sys::{
    log::Log,
    worker::{StreamRead, StreamWrite, WorkerData},
};

/// HTTP protocol
pub(crate) struct Net;

impl Net {
    pub async fn run(mut _stream_read: StreamRead, _stream_write: Arc<StreamWrite>, _data: WorkerData) {
        Log::warning(3, Some("HTTP".to_owned()));
    }

    pub fn write(_answer: Vec<u8>, _end: bool) -> Vec<u8> {
        Log::warning(3, Some("HTTP".to_owned()));
        Vec::new()
    }
}
