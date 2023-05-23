use tokio::net::TcpStream;

use crate::sys::{log::Log, worker::WorkerData};

/// SCGI protocol
pub struct Net;

impl Net {
    pub async fn run(mut _stream: TcpStream, _data: WorkerData, mut _buf: [u8; 8192], _len: usize) {
        Log::warning(3, Some("SCGI".to_owned()));
    }
}
