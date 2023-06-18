use tokio::net::TcpStream;

use crate::sys::{
    log::Log,
    worker::{WorkerData, BUFFER_SIZE},
};

/// UWSGI protocol
pub struct Net;

impl Net {
    pub async fn run(
        mut _stream: TcpStream,
        _data: WorkerData,
        mut _buf: [u8; BUFFER_SIZE],
        _len: usize,
    ) {
        Log::warning(3, Some("UWSGI".to_owned()));
    }
}
