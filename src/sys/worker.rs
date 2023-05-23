use std::{sync::Arc, time::Duration};

use tokio::{io::AsyncReadExt, net::TcpStream, sync::Mutex, time::timeout};

use super::{
    action::ActMap,
    cache::Cache,
    html::Html,
    lang::Lang,
    log::Log,
    pool::DBPool,
    workers::{fastcgi, grpc, http, scgi, uwsgi},
};

/// Connection processing workflow
pub struct Worker;

/// Worker type
///
/// # Values
///
/// * `FastCGI` - FastCGI protocol.
/// * `Uwsgi` - UWSGI protocol.
/// * `Grpc` - GRPC protocol.
/// * `Scgi` - SCGI protocol.
/// * `Http` - HTTP or WebSocket protocol.
enum WorkerType {
    /// FastCGI protocol.
    FastCGI,
    /// UWSGI protocol.
    Uwsgi,
    /// GRPC protocol.
    Grpc,
    /// SCGI protocol.
    Scgi,
    /// HTTP or WebSocket protocol.
    Http,
}

/// General data
///
/// # Values
///
/// * `engine: Arc<ActMap>` - Engine - binary tree of controller functions.
/// * `lang: Arc<Lang>` - I18n system.
/// * `html: Arc<Html>` - Template maker.
/// * `cache: Arc<Mutex<Cache>>` - Cache system.
/// * `db: Arc<DBPool>` - Database connections pool.
/// * `salt: Arc<String>` - Salt for a crypto functions.
pub struct WorkerData {
    /// Engine - binary tree of controller functions.
    pub engine: Arc<ActMap>,
    /// I18n system.
    pub lang: Arc<Lang>,
    /// Template maker.
    pub html: Arc<Html>,
    /// Cache system.
    pub cache: Arc<Mutex<Cache>>,
    /// Database connections pool.
    pub db: Arc<DBPool>,
    /// Salt for a crypto functions.
    pub salt: Arc<String>,
}

impl Worker {
    /// Run main worker
    ///
    /// # Params
    ///
    /// * `mut stream: TcpStream` - Tokio tcp stream.
    /// * `data: WorkerData` - General data for the web engine.
    pub async fn run(mut stream: TcpStream, data: WorkerData) {
        // Read first data from stream to detect protocol
        let mut buf: [u8; 8192] = [0; 8192];
        let mut len = 0;
        if let Err(e) = timeout(Duration::from_secs(1), async {
            len = match stream.read(&mut buf).await {
                Ok(len) => len,
                Err(e) => {
                    Log::warning(2000, Some(e.to_string()));
                    return;
                }
            };
        })
        .await
        {
            Log::warning(2001, Some(e.to_string()));
            return;
        };

        match Worker::detect(&buf).await {
            WorkerType::FastCGI => fastcgi::Net::run(stream, data, buf, len).await,
            WorkerType::Uwsgi => uwsgi::Net::run(stream, data, buf, len).await,
            WorkerType::Grpc => grpc::Net::run(stream, data, buf, len).await,
            WorkerType::Scgi => scgi::Net::run(stream, data, buf, len).await,
            WorkerType::Http => http::Net::run(stream, data, buf, len).await,
        }
    }

    /// Autodetect protocol
    ///
    /// # Params
    ///
    /// * `slice: &[u8; 8192]` - Slice of some first data from stream
    ///
    /// # Return
    /// * `WorkerType` - Type of the protocol
    ///
    /// # Notice
    ///
    /// This is a very easy and fast way to determine the protocol, but you shouldn't rely on it.
    async fn detect(slice: &[u8; 8192]) -> WorkerType {
        if slice[0..1] == [1] {
            // FastCGI starts with a byte equal to 1 (version)
            WorkerType::FastCGI
        } else if slice[0..1] == [0] {
            // UWSGI starts with a byte equal to 0 (modifier1)
            WorkerType::Uwsgi
        } else if slice[0..14]
            == [
                0x50, 0x52, 0x49, 0x20, 0x2a, 0x20, 0x48, 0x54, 0x54, 0x50, 0x2f, 0x32, 0x2e, 0x30,
            ]
        {
            // gRPC starts with a bytes equal to "PRI * HTTP/2.0"
            WorkerType::Grpc
        } else if slice[0..6].iter().enumerate().any(|(idx, byte)| {
            if *byte == 0x3a {
                let num = String::from_utf8_lossy(&slice[..idx]);
                num.parse::<u16>().is_ok()
            } else {
                false
            }
        }) {
            // SCGI starts with bytes equal to a number (string format) and the character ":"
            WorkerType::Scgi
        } else {
            // Everything else to be HTTP or WebSocket
            WorkerType::Http
        }
    }
}
