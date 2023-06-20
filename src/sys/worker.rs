use std::{collections::HashMap, sync::Arc};

use chrono::Utc;
use tokio::{io::AsyncReadExt, net::TcpStream, sync::Mutex, time::timeout};

use crate::TINY_KEY;

use super::{
    action::{ActMap, Action, ActionData, Answer, WebFile},
    cache::Cache,
    file::TempFile,
    html::Html,
    lang::Lang,
    log::Log,
    pool::DBPool,
    workers::{fastcgi, grpc, http, scgi, uwsgi},
};

/// Buffer for read data from TcpStream
pub const BUFFER_SIZE: usize = 8192;

/// One year in seconds
const ONE_YEAR: i64 = 31622400;

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
#[derive(Debug)]
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
        let mut buf: [u8; BUFFER_SIZE] = [0; BUFFER_SIZE];
        let mut len = 0;
        if let Err(e) = timeout(std::time::Duration::from_secs(1), async {
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

        match Worker::detect(&buf) {
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
    /// * `slice: &[u8; BUFFER_SIZE]` - Slice of some first data from stream
    ///
    /// # Return
    /// * `WorkerType` - Type of the protocol
    ///
    /// # Notice
    ///
    /// This is a very easy and fast way to determine the protocol, but you shouldn't rely on it.
    fn detect(slice: &[u8; BUFFER_SIZE]) -> WorkerType {
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
        } else if slice[0..7].iter().enumerate().any(|(idx, byte)| {
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
    pub async fn call_action(data: ActionData) -> Vec<u8> {
        let mut action = match Action::new(data).await {
            Ok(action) => action,
            Err((redirect, files)) => {
                // Write status
                let mut answer: Vec<u8> = Vec::with_capacity(512);
                if redirect.permanently {
                    answer.extend_from_slice(
                        format!(
                            "Status: 301 {}\r\nLocation: {}\r\n\r\n",
                            Worker::http_code_get(301),
                            redirect.url
                        )
                        .as_bytes(),
                    );
                } else {
                    answer.extend_from_slice(
                        format!(
                            "Status: 302 {}\r\nLocation: {}\r\n\r\n",
                            Worker::http_code_get(302),
                            redirect.url
                        )
                        .as_bytes(),
                    );
                }
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
                return answer;
            }
        };

        let result = match Action::run(&mut action).await {
            Answer::Raw(answer) => answer,
            Answer::String(answer) => answer.into_bytes(),
            Answer::None => Vec::new(),
        };

        // + Status + Cookie + Keep-alive + Content-Type + Content-Length + headers
        // max length
        let capacity = result.len() + 4096;

        // Write status
        let mut answer: Vec<u8> = Vec::with_capacity(capacity);
        if let Some(redirect) = action.response.redirect.as_ref() {
            if redirect.permanently {
                answer.extend_from_slice(
                    format!(
                        "Status: 301 {}\r\nLocation: {}\r\n",
                        Worker::http_code_get(301),
                        redirect.url
                    )
                    .as_bytes(),
                );
            } else {
                answer.extend_from_slice(
                    format!(
                        "Status: 302 {}\r\nLocation: {}\r\n",
                        Worker::http_code_get(302),
                        redirect.url
                    )
                    .as_bytes(),
                );
            }
        } else if let Some(code) = action.response.http_code {
            answer.extend_from_slice(
                format!("Status: {} {}\r\n", code, Worker::http_code_get(code)).as_bytes(),
            );
        } else {
            answer.extend_from_slice(
                format!("Status: 200 {}\r\n", Worker::http_code_get(200)).as_bytes(),
            );
        }

        // Write Cookie
        let time = Utc::now() + chrono::Duration::seconds(ONE_YEAR);
        let date = time.format("%a, %d-%b-%Y %H:%M:%S GMT").to_string();
        let secure = if action.request.scheme == "https" {
            "Secure; "
        } else {
            ""
        };

        answer.extend_from_slice(
            format!(
                "Set-Cookie: {}={}; Expires={}; Max-Age={}; path=/; domain={}; {}SameSite=none\r\n",
                TINY_KEY, &action.session.key, date, ONE_YEAR, action.request.host, secure
            )
            .as_bytes(),
        );
        // Write Content-Type
        match &action.response.content_type {
            Some(content_type) => {
                answer.extend_from_slice(format!("Content-Type: {}\r\n", content_type).as_bytes())
            }
            None => answer.extend_from_slice(b"Content-Type: text/html; charset=utf-8\r\n"),
        }
        answer.extend_from_slice(b"Connection: Keep-Alive\r\n");
        // Write headers
        for head in &action.response.headers {
            answer.extend_from_slice(format!("{}\r\n", head).as_bytes());
        }
        // Write Content-Length
        answer.extend_from_slice(format!("Content-Length: {}\r\n\r\n", result.len()).as_bytes());
        answer.extend_from_slice(&result);

        // Stopping a call in a parallel thread
        tokio::spawn(async move {
            Action::stop(action).await;
        });
        answer
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
    ) -> (HashMap<String, String>, HashMap<String, Vec<WebFile>>) {
        let mut post = HashMap::new();
        let mut file = HashMap::new();

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
                                                Worker::get_post_file(h, d, &mut post, &mut file)
                                                    .await;
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
                }
            }
        }
        (post, file)
    }

    /// Gets post and file records from multipart/form-data
    pub async fn get_post_file(
        header: &str,
        data: &[u8],
        post: &mut HashMap<String, String>,
        file: &mut HashMap<String, Vec<WebFile>>,
    ) {
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
