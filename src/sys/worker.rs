use core::str;
use std::{cmp::min, collections::HashMap, io::Error, sync::Arc};

use chrono::{TimeDelta, Utc};
use serde_json::Value;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{
        tcp::{OwnedReadHalf, OwnedWriteHalf},
        TcpStream,
    },
    sync::{
        mpsc::{self, Sender},
        Mutex,
    },
    task::JoinHandle,
};

#[cfg(debug_assertions)]
use tokio::sync::RwLock;

use super::{
    action::{ActMap, Action, ActionData, Answer},
    cache::CacheSys,
    dbs::adapter::DB,
    file::TempFile,
    html::Html,
    init::Addr,
    lang::Lang,
    log::Log,
    mail::Mail,
    request::{RawData, WebFile},
    route::Route,
    workers::{fastcgi, grpc, http, scgi, uwsgi, websocket},
};

/// Buffer for read data from TcpStream
pub const BUFFER_SIZE: usize = 8192;

/// One year in seconds
const ONE_YEAR: i64 = 31622400;

/// Connection processing workflow
#[derive(Debug)]
pub(crate) struct Worker;

/// Worker type
///
/// # Values
///
/// * `FastCGI` - FastCGI protocol.
/// * `Uwsgi` - UWSGI protocol.
/// * `Grpc` - GRPC protocol.
/// * `Scgi` - SCGI protocol.
/// * `Http` - HTTP protocol.
/// * `WebSocket` - WebSocket protocol.
#[derive(Debug, Clone)]
pub(crate) enum WorkerType {
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
    /// WebSocket
    WebSocket,
}

/// General data
#[derive(Debug)]
pub(crate) struct WorkerData {
    /// Engine - binary tree of controller functions.
    pub engine: Arc<ActMap>,
    /// I18n system.
    #[cfg(not(debug_assertions))]
    pub lang: Arc<Lang>,
    /// I18n system.
    #[cfg(debug_assertions)]
    pub lang: Arc<RwLock<Lang>>,
    /// Template maker.
    #[cfg(not(debug_assertions))]
    pub html: Arc<Html>,
    /// Template maker.
    #[cfg(debug_assertions)]
    pub html: Arc<RwLock<Html>>,
    /// Cache system.
    pub cache: Arc<Mutex<CacheSys>>,
    /// Database connections pool.
    pub db: Arc<DB>,
    /// Session key.
    pub session_key: Arc<String>,
    /// Salt for a crypto functions.
    pub salt: Arc<String>,
    /// Mail provider.
    pub mail: Arc<Mutex<Mail>>,
    /// Default controller for request "/" or default class or default action
    pub action_index: Arc<Route>,
    /// Default controller for 404 Not Found
    pub action_not_found: Arc<Route>,
    /// Default controller for error_route
    pub action_err: Arc<Route>,
    /// Stop signal
    pub(crate) stop: Option<(Arc<Addr>, i64, Arc<String>)>,
    /// The full path to the folder where the server was started.
    pub(crate) root: Arc<String>,
}

/// A network stream errors
pub(crate) enum StreamError {
    /// Stream are closed
    Closed,
    /// Error reading from stream
    Error(Error),
    /// Buffer is small
    Buffer,
    /// Read timeout
    Timeout,
}

/// Half of the network to read
pub(crate) struct StreamRead {
    /// A Tokio network stream
    tcp: OwnedReadHalf,
    /// Reading buffer
    buf: [u8; BUFFER_SIZE],
    /// The number of bytes in the buffer
    len: usize,
    // Reading shift
    shift: usize,
}

impl StreamRead {
    /// Read from stream to buffer
    ///
    /// # Params
    ///
    /// * `timeout: u64` - How long to wait for a reading? (in milliseconds)
    ///
    /// If timeout = 0, it will wait until data appears or an error occurs.
    /// After `StreamError::Timeout`, saving data in the buffer and correct operation of the protocol are not guaranteed, so it is advisable to close the stream.
    pub async fn read(&mut self, timeout: u64) -> Result<(), StreamError> {
        if self.shift == 0 && self.len == BUFFER_SIZE {
            return Err(StreamError::Buffer);
        } else if self.shift > 0 && self.len <= BUFFER_SIZE {
            self.buf.copy_within(self.shift.., 0);
            self.len -= self.shift;
            self.shift = 0;
        }

        if timeout > 0 {
            match tokio::time::timeout(std::time::Duration::from_millis(timeout), async {
                match self.tcp.read(unsafe { self.buf.get_unchecked_mut(self.len..) }).await {
                    Ok(len) => {
                        if len == 0 {
                            Err(StreamError::Closed)
                        } else {
                            Ok(len)
                        }
                    }
                    Err(e) => Err(StreamError::Error(e)),
                }
            })
            .await
            {
                Ok(res) => match res {
                    Ok(len) => {
                        self.len += len;
                        Ok(())
                    }
                    Err(e) => Err(e),
                },
                Err(_) => Err(StreamError::Timeout),
            }
        } else {
            match self.tcp.read(unsafe { self.buf.get_unchecked_mut(self.len..) }).await {
                Ok(len) => {
                    if len == 0 {
                        Err(StreamError::Closed)
                    } else {
                        self.len += len;
                        Ok(())
                    }
                }
                Err(e) => Err(StreamError::Error(e)),
            }
        }
    }

    /// Gets bytes in the local buffer
    pub fn get(&self, size: usize) -> &[u8] {
        let size = min(self.shift + size, self.len);
        unsafe { self.buf.get_unchecked(self.shift..size) }
    }

    /// Adds shift in the data
    pub fn shift(&mut self, shift: usize) {
        self.shift += shift;
        if self.shift > self.len {
            self.shift = self.len;
        }
    }

    /// Gets length of available data
    pub fn available(&self) -> usize {
        self.len - self.shift
    }
}

/// Half of the network to write
pub(crate) struct StreamWrite {
    /// Sender
    pub tx: Arc<Sender<MessageWrite>>,
}

#[derive(Debug)]
pub(crate) enum MessageWrite {
    Message(Vec<u8>, bool),
    End,
}

impl StreamWrite {
    pub async fn new(mut tcp: OwnedWriteHalf, protocol: Arc<WorkerType>) -> (Arc<StreamWrite>, JoinHandle<()>) {
        let (tx, mut rx) = mpsc::channel(32);
        let stream = Arc::new(StreamWrite { tx: Arc::new(tx) });

        let handle = tokio::spawn(async move {
            while let Some(message) = rx.recv().await {
                match message {
                    MessageWrite::Message(message, end) => {
                        let data = match protocol.as_ref() {
                            WorkerType::FastCGI => fastcgi::Net::write(message, end),
                            WorkerType::Uwsgi => uwsgi::Net::write(message, end),
                            WorkerType::Grpc => grpc::Net::write(message, end),
                            WorkerType::Scgi => scgi::Net::write(message, end),
                            WorkerType::Http => http::Net::write(message, end),
                            WorkerType::WebSocket => websocket::Net::write(message, end),
                        };
                        if let Err(e) = tcp.write_all(&data).await {
                            Log::warning(101, Some(e.to_string()));
                        }
                    }
                    MessageWrite::End => break,
                }
            }
        });
        (stream, handle)
    }

    /// Write unswer to the stream
    pub async fn write(&self, data: Vec<u8>) {
        if let Err(e) = self.tx.send(MessageWrite::Message(data, true)).await {
            Log::warning(100, Some(e.to_string()));
        }
    }

    async fn end(handle: JoinHandle<()>, tx: Arc<Sender<MessageWrite>>) {
        if let Err(e) = tx.send(MessageWrite::End).await {
            Log::warning(100, Some(e.to_string()));
        }
        if let Err(e) = handle.await {
            Log::warning(102, Some(e.to_string()));
        }
    }
}

impl Worker {
    /// Run main worker
    ///
    /// # Params
    ///
    /// * `stream: TcpStream` - Tokio tcp stream.
    /// * `data: WorkerData` - General data for the web engine.
    /// * `protocol: Arc<WorkerType>` - Used protocol.
    pub async fn run(stream: TcpStream, data: WorkerData, protocol: Arc<WorkerType>) {
        let (read, write) = stream.into_split();
        let mut stream_read = StreamRead {
            tcp: read,
            buf: [0; BUFFER_SIZE],
            len: 0,
            shift: 0,
        };
        let (stream_write, handle) = StreamWrite::new(write, Arc::clone(&protocol)).await;
        let tx = Arc::clone(&stream_write.tx);
        if let Err(e) = stream_read.read(1000).await {
            match e {
                StreamError::Closed => {}
                StreamError::Error(e) => {
                    Log::warning(2000, Some(e.to_string()));
                }
                StreamError::Buffer => {
                    Log::warning(2006, None);
                }
                StreamError::Timeout => {
                    Log::warning(2001, None);
                }
            }
            return;
        }
        match protocol.as_ref() {
            WorkerType::FastCGI => fastcgi::Net::run(stream_read, stream_write, data).await,
            WorkerType::Uwsgi => uwsgi::Net::run(stream_read, stream_write, data).await,
            WorkerType::Grpc => grpc::Net::run(stream_read, stream_write, data).await,
            WorkerType::Scgi => scgi::Net::run(stream_read, stream_write, data).await,
            WorkerType::Http => http::Net::run(stream_read, stream_write, data).await,
            WorkerType::WebSocket => websocket::Net::run(stream_read, stream_write, data).await,
        }
        StreamWrite::end(handle, tx).await;
    }

    /// Return a text description of the return code
    pub fn http_code_get(code: u16) -> &'static str {
        match code {
            100 => "Continue",
            101 => "Switching Protocols",
            102 => "Processing",
            103 => "Early Hints",
            200 => "OK",
            201 => "Created",
            202 => "Accepted",
            203 => "Non-Authoritative Information",
            204 => "No Content",
            205 => "Reset Content",
            206 => "Partial Content",
            207 => "Multi-Status",
            208 => "Already Reported",
            226 => "IM Used",
            300 => "Multiple Choices",
            301 => "Moved Permanently",
            302 => "Found",
            303 => "See Other",
            304 => "Not Modified",
            305 => "Use Proxy",
            306 => "(Unused)",
            307 => "Temporary Redirect",
            308 => "Permanent Redirect",
            400 => "Bad Request",
            401 => "Unauthorized",
            402 => "Payment Required",
            403 => "Forbidden",
            404 => "Not Found",
            405 => "Method Not Allowed",
            406 => "Not Acceptable",
            407 => "Proxy Authentication Required",
            408 => "Request Timeout",
            409 => "Conflict",
            410 => "Gone",
            411 => "Length Required",
            412 => "Precondition Failed",
            413 => "Content Too Large",
            414 => "URI Too Long",
            415 => "Unsupported Media Type",
            416 => "Range Not Satisfiable",
            417 => "Expectation Failed",
            418 => "(Unused)",
            421 => "Misdirected Request",
            422 => "Unprocessable Content",
            423 => "Locked",
            424 => "Failed Dependency",
            425 => "Too Early",
            426 => "Upgrade Required",
            428 => "Precondition Required",
            429 => "Too Many Requests",
            431 => "Request Header Fields Too Large",
            451 => "Unavailable For Legal Reasons",
            500 => "Internal Server Error",
            501 => "Not Implemented",
            502 => "Bad Gateway",
            503 => "Service Unavailable",
            504 => "Gateway Timeout",
            505 => "HTTP Version Not Supported",
            506 => "Variant Also Negotiates",
            507 => "Insufficient Storage",
            508 => "Loop Detected",
            510 => "Not Extended (OBSOLETED)",
            511 => "Network Authentication Required",
            _ => "Unassigned",
        }
    }

    /// Run web engine
    ///
    /// # Return
    ///
    /// Vector with a binary data
    pub(crate) async fn call_action(data: ActionData) -> Vec<u8> {
        #[cfg(debug_assertions)]
        Log::info(228, Some(format!("{} {:?} {}{}", data.request.ip, data.request.method, data.request.site, data.request.url)));

        // Check and reload langs and templates
        #[cfg(debug_assertions)]
        Worker::reload(Arc::clone(&data.lang), Arc::clone(&data.html)).await;

        let mut action = match Action::new(data).await {
            Ok(action) => action,
            Err((redirect, files)) => {
                // Stopping a call in a parallel thread
                tokio::spawn(async move {
                    let mut vec = Vec::with_capacity(32);
                    for files in files.into_values() {
                        for f in files {
                            vec.push(f.tmp)
                        }
                    }
                    Action::clean_file(vec).await;
                });
                // Write status
                let mut answer: Vec<u8> = Vec::with_capacity(512);
                if redirect.permanently {
                    answer.extend_from_slice(
                        format!("Status: 301 {}\r\nLocation: {}\r\n\r\n", Worker::http_code_get(301), redirect.url).as_bytes(),
                    );
                } else {
                    answer.extend_from_slice(
                        format!("Status: 302 {}\r\nLocation: {}\r\n\r\n", Worker::http_code_get(302), redirect.url).as_bytes(),
                    );
                }
                return answer;
            }
        };

        let result = match Action::run(&mut action).await {
            Answer::Raw(answer) => answer,
            Answer::String(answer) => answer.into_bytes(),
            Answer::None => Vec::new(),
        };

        let answer = if !action.header_send {
            // + Status + Cookie + Keep-alive + Content-Type + Content-Length + headers
            // max length
            let capacity = result.len() + 4096;
            let mut answer = Worker::get_header(capacity, &action, Some(result.len()));
            answer.extend_from_slice(&result);
            answer
        } else {
            result
        };

        // Stopping a call in a parallel thread
        tokio::spawn(async move {
            Action::end(action).await;
        });
        answer
    }

    /// Reload lang and template
    #[cfg(debug_assertions)]
    async fn reload(lang: Arc<RwLock<Lang>>, html: Arc<RwLock<Html>>) {
        let changed = lang.read().await.check_time().await;
        if changed {
            let mut lang = lang.write().await;
            let files = Lang::get_files(Arc::clone(&lang.root)).await;
            lang.load(files).await;
        }
        let changed = html.read().await.check_time().await;
        if changed {
            let mut html = html.write().await;
            html.load().await;
        }
    }

    fn get_header(capacity: usize, action: &Action, content_length: Option<usize>) -> Vec<u8> {
        // Write status
        let mut answer: Vec<u8> = Vec::with_capacity(capacity);
        if let Some(redirect) = action.response.redirect.as_ref() {
            if redirect.permanently {
                answer
                    .extend_from_slice(format!("Status: 301 {}\r\nLocation: {}\r\n", Worker::http_code_get(301), redirect.url).as_bytes());
            } else {
                answer
                    .extend_from_slice(format!("Status: 302 {}\r\nLocation: {}\r\n", Worker::http_code_get(302), redirect.url).as_bytes());
            }
        } else if let Some(code) = action.response.http_code {
            answer.extend_from_slice(format!("Status: {} {}\r\n", code, Worker::http_code_get(code)).as_bytes());
        } else {
            answer.extend_from_slice(format!("Status: 200 {}\r\n", Worker::http_code_get(200)).as_bytes());
        }

        // Write Cookie
        let sec = TimeDelta::new(ONE_YEAR, 0).unwrap_or(TimeDelta::zero());
        let time = Utc::now() + sec;
        let date = time.format("%a, %d-%b-%Y %H:%M:%S GMT").to_string();
        let secure = if action.request.scheme == "https" { "Secure; " } else { "" };

        answer.extend_from_slice(
            format!(
                "Set-Cookie: {}={}; Expires={}; Max-Age={}; path=/; domain={}; {}SameSite=none\r\n",
                action.session.session_key, &action.session.key, date, ONE_YEAR, action.request.host, secure
            )
            .as_bytes(),
        );
        // Write Content-Type
        match &action.response.content_type {
            Some(content_type) => answer.extend_from_slice(format!("Content-Type: {}\r\n", content_type).as_bytes()),
            None => answer.extend_from_slice(b"Content-Type: text/html; charset=utf-8\r\n"),
        }
        answer.extend_from_slice(b"Connection: Keep-Alive\r\n");
        // Write headers
        for (name, val) in &action.response.headers {
            answer.extend_from_slice(format!("{}: {}\r\n", name, val).as_bytes());
        }
        // Write Content-Length
        if let Some(len) = content_length {
            answer.extend_from_slice(format!("Content-Length: {}\r\n", len).as_bytes());
        }
        answer.extend_from_slice(b"\r\n");

        answer
    }

    pub async fn write(action: &Action, src: Vec<u8>) {
        let src = if !action.header_send {
            let mut vec = Worker::get_header(src.len() + 4096, action, None);
            vec.extend_from_slice(&src);
            vec
        } else {
            src
        };
        if let Err(e) = action.tx.send(MessageWrite::Message(src, false)).await {
            Log::warning(100, Some(e.to_string()));
        }
    }

    /// Read post and file datas.
    ///
    /// # Params
    ///
    /// * `data: Vec<u8>` - Data.
    /// * `content_type: Option<String>` - CONTENT_TYPE parameter
    ///
    /// # Return
    ///
    /// * `HashMap<String, String>` - Post data.
    /// * `HashMap<String, Vec<WebFile>>` - File data.
    pub async fn read_input(
        data: Vec<u8>,
        content_type: Option<String>,
    ) -> (HashMap<String, String>, HashMap<String, Vec<WebFile>>, RawData) {
        let mut post = HashMap::new();
        let mut file = HashMap::new();
        let mut raw = RawData::None;

        // Different types of CONTENT_TYPE need to be processed differently
        if let Some(c) = content_type {
            // Simple post
            if c == "application/x-www-form-urlencoded" {
                if !data.is_empty() {
                    if let Ok(s) = std::str::from_utf8(&data) {
                        let val: Vec<&str> = s.split('&').collect();
                        post.reserve(val.len());
                        for v in val {
                            let val: Vec<&str> = v.splitn(2, '=').collect();
                            match val.len() {
                                1 => post.insert(v.to_owned(), String::new()),
                                _ => post.insert(val[0].to_owned(), val[1].to_owned()),
                            };
                        }
                    }
                }
            } else if c == "application/json;charset=UTF-8" {
                raw = match serde_json::from_slice::<Value>(&data) {
                    Ok(v) => RawData::Json(v),
                    Err(_) => match String::from_utf8(data.clone()) {
                        Ok(s) => RawData::String(s),
                        Err(e) => RawData::Raw(e.into_bytes()),
                    },
                };
            } else if c.len() > 30 {
                // Multi post with files
                if let "multipart/form-data; boundary=" = &c[..30] {
                    let boundary = format!("--{}", &c[30..]);
                    let stop: [u8; 4] = [13, 10, 13, 10];
                    if !data.is_empty() {
                        let mut seek: usize = 0;
                        let mut start: usize;
                        let b_len = boundary.len();
                        let len = data.len() - 4;
                        let mut found: Option<(usize, &str)> = None;
                        while seek < len {
                            // Find a boundary
                            if boundary.as_bytes() == &data[seek..seek + b_len] {
                                if seek + b_len == len {
                                    if let Some((l, h)) = found {
                                        let d = &data[l..seek - 2];
                                        Worker::get_post_file(h, d, &mut post, &mut file).await;
                                    };
                                    break;
                                }
                                seek += b_len + 2;
                                start = seek;
                                while seek < len {
                                    if stop == data[seek..seek + 4] {
                                        if let Ok(s) = std::str::from_utf8(&data[start..seek]) {
                                            if let Some((l, h)) = found {
                                                let d = &data[l..start - b_len - 4];
                                                Worker::get_post_file(h, d, &mut post, &mut file).await;
                                            };
                                            found = Some((seek + 4, s));
                                        }
                                        seek += 4;
                                        break;
                                    } else {
                                        seek += 1;
                                    }
                                }
                            } else {
                                seek += 1;
                            }
                        }
                    }
                } else {
                    raw = match String::from_utf8(data.clone()) {
                        Ok(s) => RawData::String(s),
                        Err(e) => RawData::Raw(e.into_bytes()),
                    }
                }
            } else {
                raw = match String::from_utf8(data.clone()) {
                    Ok(s) => RawData::String(s),
                    Err(e) => RawData::Raw(e.into_bytes()),
                }
            }
        } else if !data.is_empty() {
            raw = match String::from_utf8(data.clone()) {
                Ok(s) => RawData::String(s),
                Err(e) => RawData::Raw(e.into_bytes()),
            }
        }
        (post, file, raw)
    }

    /// Gets post and file records from multipart/form-data
    async fn get_post_file(header: &str, data: &[u8], post: &mut HashMap<String, String>, file: &mut HashMap<String, Vec<WebFile>>) {
        let h: Vec<&str> = header.splitn(3, "; ").collect();
        let len = h.len();

        // Post data found
        if len == 2 {
            if let Ok(v) = std::str::from_utf8(data) {
                let k = &h[1][6..h[1].len() - 1];
                post.insert(k.to_owned(), v.to_owned());
            }
        } else if len == 3 {
            // File data found
            let k = h[1][6..h[1].len() - 1].to_owned();
            let n: Vec<&str> = h[2].splitn(2, "\r\n").collect();
            let n = &n[0][10..n[0].len() - 1];

            let path = TempFile::new_name();
            if TempFile::write(&path, data).await.is_ok() {
                if file.get(&k).is_none() {
                    file.insert(k.to_owned(), Vec::with_capacity(16));
                } else if let Some(d) = file.get_mut(&k) {
                    d.push(WebFile {
                        size: data.len(),
                        name: n.to_owned(),
                        tmp: path,
                    })
                };
            }
        }
    }
}
