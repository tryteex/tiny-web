use tokio::net::TcpStream;

use crate::sys::{
    log::Log,
    worker::{WorkerData, BUFFER_SIZE},
};

/// HTTP protocol
pub struct Net;

impl Net {
    pub async fn run(
        mut _stream: TcpStream,
        _data: WorkerData,
        mut _buf: [u8; BUFFER_SIZE],
        _len: usize,
    ) {
        Log::warning(3, Some("HTTP".to_owned()));
    }
}
