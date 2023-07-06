use crate::sys::{
    log::Log,
    worker::{StreamRead, StreamWrite, WorkerData},
};

/// GRPC protocol
pub struct Net;

impl Net {
    pub async fn run(_stream_read: StreamRead, _stream_write: StreamWrite, _data: WorkerData) {
        Log::warning(3, Some("gRPC".to_owned()));
    }
}
