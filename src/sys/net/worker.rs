use std::{collections::HashMap, sync::Arc};

#[cfg(feature = "https")]
use std::io::Error;

#[cfg(any(feature = "http", feature = "https"))]
use std::net::IpAddr;

#[cfg(any(feature = "http", feature = "https"))]
use std::path::PathBuf;

#[cfg(any(feature = "html-reload", feature = "lang-reload"))]
use tokio::sync::RwLock;

#[cfg(feature = "https")]
use tokio_rustls::TlsAcceptor;

use crate::{
    log, log_vv,
    sys::{
        stat::stat::Stat,
        web::{
            action::{Action, ActionData, ActionRedirect, ModuleMap},
            request::{HttpVersion, RawData, WebFile},
        },
    },
};

#[cfg(any(feature = "mail-sendmail", feature = "mail-smtp", feature = "mail-file"))]
use crate::sys::app::init::MailConfig;

#[cfg(any(feature = "pgsql", feature = "mssql"))]
use crate::sys::db::adapter::DB;

#[cfg(feature = "cache")]
use crate::sys::web::cache::Cache;

#[cfg(feature = "file-disk")]
use crate::sys::web::file::TempFile;

#[cfg(any(feature = "html-static", feature = "html-reload"))]
use crate::sys::web::html::Html;

#[cfg(any(feature = "lang-static", feature = "lang-reload"))]
use crate::sys::web::lang::Lang;

#[cfg(any(feature = "session-memory", feature = "session-file", feature = "session-db"))]
use crate::sys::web::session::SessionLoader;

use super::stream::{MessageWrite, Stream, StreamRead, StreamWrite, BUFFER_SIZE};

#[cfg(feature = "fastcgi")]
use super::fastcgi::FastCGI;

#[cfg(any(feature = "http", feature = "https"))]
use super::http::Http;

#[cfg(feature = "scgi")]
use super::scgi::Scgi;

#[cfg(feature = "uwsgi")]
use super::uwsgi::Uwsgi;

pub(crate) struct WorkerData {
    #[cfg(feature = "debug-vvv")]
    pub id: u64,
    pub mon: Arc<Stat>,
    pub engine: Arc<ModuleMap>,
    #[cfg(any(feature = "http", feature = "https"))]
    pub root: Arc<PathBuf>,
    pub salt: Arc<String>,
    #[cfg(any(feature = "http", feature = "https"))]
    pub ip: Option<IpAddr>,
    pub index: Arc<[i64; 3]>,
    pub not_found: Option<Arc<[i64; 3]>>,
    #[cfg(any(feature = "pgsql", feature = "mssql"))]
    pub db: Arc<DB>,
    #[cfg(feature = "https")]
    pub acceptor: Arc<TlsAcceptor>,
    #[cfg(feature = "html-static")]
    pub html: Arc<Html>,
    #[cfg(feature = "html-reload")]
    pub html: Arc<RwLock<Html>>,
    #[cfg(any(feature = "session-memory", feature = "session-file", feature = "session-db"))]
    pub session: Arc<SessionLoader>,
    #[cfg(feature = "lang-static")]
    pub lang: Arc<Lang>,
    #[cfg(feature = "lang-reload")]
    pub lang: Arc<RwLock<Lang>>,
    #[cfg(any(feature = "mail-sendmail", feature = "mail-smtp", feature = "mail-file"))]
    pub mail: Arc<MailConfig>,
    #[cfg(feature = "cache")]
    pub cache: Arc<Cache>,
}

pub(crate) struct Worker;

impl Worker {
    pub(crate) async fn run(stream: Stream, data: WorkerData) {
        #[cfg(not(feature = "https"))]
        let (read, write) = stream.into_split();
        #[cfg(feature = "https")]
        let (read, write) = match stream.into_split(Arc::clone(&data.acceptor)).await {
            Ok(read_write) => read_write,
            Err(_e) => {
                log!(warning, 0, "{}", _e);
                return;
            }
        };
        let stream_read = StreamRead {
            stream: read,
            buf: [0; BUFFER_SIZE],
            len: 0,
            shift: 0,
        };
        let (stream_write, handle) = StreamWrite::new(write).await;
        let tx = Arc::clone(&stream_write.tx);

        #[cfg(any(feature = "http", feature = "https"))]
        Http::run(stream_read, stream_write, data).await;
        #[cfg(feature = "fastcgi")]
        FastCGI::run(stream_read, stream_write, data).await;
        #[cfg(feature = "scgi")]
        Scgi::run(stream_read, stream_write, data).await;
        #[cfg(feature = "uwsgi")]
        Uwsgi::run(stream_read, stream_write, data).await;

        StreamWrite::end(handle, tx).await;
    }

    pub(crate) async fn write(action: &Action, src: Vec<u8>) {
        let src = if !action.header_send {
            let mut vec = Worker::get_header(src.len() + 4096, action, None);
            vec.extend_from_slice(&src);
            vec
        } else {
            src
        };

        #[cfg(not(feature = "fastcgi"))]
        if let Err(_e) = action.tx.send(MessageWrite::Message(src)).await {
            log!(warning, 0, "{}", _e);
        }
        #[cfg(feature = "fastcgi")]
        if let Err(_e) = action.tx.send(MessageWrite::Message(src, false)).await {
            log!(warning, 0, "{}", _e);
        }
    }

    pub(crate) async fn read_input(data: Vec<u8>, content_type: Option<&str>) -> (HashMap<String, String>, Vec<WebFile>, RawData) {
        let mut post = HashMap::new();
        let mut file = Vec::new();
        let mut raw = RawData::None;

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
                                        Worker::get_post_file(h, d.to_vec(), &mut post, &mut file).await;
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
                                                Worker::get_post_file(h, d.to_vec(), &mut post, &mut file).await;
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
                    raw = RawData::Raw(data);
                }
            } else {
                raw = RawData::Raw(data);
            }
        } else if !data.is_empty() {
            raw = RawData::Raw(data)
        }
        (post, file, raw)
    }

    async fn get_post_file(header: &str, data: Vec<u8>, post: &mut HashMap<String, String>, file: &mut Vec<WebFile>) {
        let parts: Vec<&str> = header.splitn(3, "; ").collect();
        let len = parts.len();
        // Post data found
        if len == 2 {
            if let Ok(value) = std::str::from_utf8(&data) {
                let name = unsafe { *parts.get_unchecked(1) };
                let name = &name[6..name.len() - 1];
                post.insert(name.to_owned(), value.to_owned());
            }
        } else if len == 3 {
            // File data found
            let form = unsafe { *parts.get_unchecked(1) };
            let form = form[6..form.len() - 1].to_owned();
            let name = unsafe { *parts.get_unchecked(2) };
            let name: Vec<&str> = name.splitn(2, "\r\n").collect();
            let name = &name[0][10..name[0].len() - 1];

            #[cfg(feature = "file-disk")]
            let path = TempFile::new_name();
            #[cfg(feature = "file-disk")]
            if TempFile::write(&path, &data).await.is_err() {
                return;
            }
            file.push(WebFile {
                name: form.to_owned(),
                file: name.to_owned(),
                size: data.len(),
                #[cfg(feature = "file-disk")]
                tmp: path,
                #[cfg(feature = "file-memory")]
                data,
            });
        }
    }

    #[cfg(feature = "https")]
    pub(crate) fn load_cert(root: Arc<PathBuf>) -> Result<Arc<TlsAcceptor>, Error> {
        use std::{
            fs::File,
            io::{BufReader, ErrorKind},
        };

        use rustls::{
            pki_types::{CertificateDer, PrivateKeyDer},
            ServerConfig,
        };
        use rustls_pemfile::{certs, read_all, Item};

        let mut cert_file = root.as_ref().to_owned();
        cert_file.push("ssl");
        let mut key_file = cert_file.clone();
        cert_file.push("certificate.crt");
        key_file.push("privateKey.key");

        let mut cert_file = BufReader::new(File::open(cert_file)?);
        let mut key_file = BufReader::new(File::open(key_file)?);

        let certs = certs(&mut cert_file).collect::<Result<Vec<CertificateDer<'static>>, Error>>()?;

        let pem_files = match read_all(&mut key_file).next() {
            Some(file) => file?,
            None => return Err(Error::new(ErrorKind::Other, "Private key not found in file ./ssl/privateKey.key")),
        };
        let key = match pem_files {
            Item::Pkcs1Key(key) => PrivateKeyDer::Pkcs1(key),
            Item::Pkcs8Key(key) => PrivateKeyDer::Pkcs8(key),
            Item::Sec1Key(key) => PrivateKeyDer::Sec1(key),
            e => return Err(Error::new(ErrorKind::Other, format!("Private key not support {:?} in file ./ssl/privateKey.key", e))),
        };

        let tls_config = match ServerConfig::builder().with_no_client_auth().with_single_cert(certs, key) {
            Ok(config) => Arc::new(config),
            Err(e) => return Err(Error::new(ErrorKind::Other, e)),
        };

        Ok(Arc::new(TlsAcceptor::from(tls_config)))
    }

    pub(super) async fn call_action(data: ActionData) -> Vec<u8> {
        #[cfg(any(feature = "debug-vv", feature = "debug-vvv"))]
        let id = data.id;
        log_vv!(info, 0, "Async thread: {}. {:?} {:?} {}{}", id, data.request.ip, data.request.method, data.request.site, data.request.url,);
        #[cfg(any(feature = "debug-vv", feature = "debug-vvv"))]
        let time_start = chrono::Local::now();

        #[cfg(any(feature = "html-reload", feature = "lang-reload"))]
        Worker::reload(&data).await;

        let status = data.request.version.get_status();
        #[cfg(any(feature = "session-memory", feature = "session-file", feature = "session-db"))]
        let session = Arc::clone(&data.session_loader);
        let answer = match Action::init(data).await {
            Ok(ActionRedirect::Action(mut action)) => {
                let result = Action::run(&mut action).await;

                let result = if !action.header_send {
                    // + Status + Cookie + Keep-alive + Content-Type + Content-Length + headers
                    // max length
                    let capacity = result.len() + 4096;
                    let mut answer = Worker::get_header(capacity, &action, Some(result.len()));
                    answer.extend_from_slice(&result);
                    answer
                } else {
                    Vec::new()
                };
                #[cfg(any(
                    feature = "file-disk",
                    feature = "session-memory",
                    feature = "session-file",
                    feature = "session-db"
                ))]
                tokio::spawn(async move {
                    #[cfg(feature = "file-disk")]
                    Action::clean_file(&action.request.input.file).await;
                    #[cfg(any(feature = "session-memory", feature = "session-file", feature = "session-db"))]
                    let _ = session.save(action.session).await;
                });
                result
            }
            #[cfg(feature = "redirect-db")]
            Ok(ActionRedirect::Redirect(redirect)) => {
                // Write status
                let mut answer = Vec::with_capacity(512);
                if redirect.permanently {
                    answer.extend_from_slice(
                        format!("{status} 301 {}\r\nLocation: {}\r\n\r\n", Worker::http_code_get(301), redirect.url).as_bytes(),
                    );
                } else {
                    answer.extend_from_slice(
                        format!("{status} 302 {}\r\nLocation: {}\r\n\r\n", Worker::http_code_get(302), redirect.url).as_bytes(),
                    );
                }
                answer
            }
            Err(_) => Worker::get_500(status),
        };

        #[cfg(any(feature = "debug-vv", feature = "debug-vvv"))]
        let delta = chrono::Local::now() - time_start;
        log_vv!(info, 0, "Async thread: {}. Time execute={}", id, delta);

        answer
    }

    #[inline]
    pub(crate) fn get_500(status: &str) -> Vec<u8> {
        format!("{status} 500 {}\r\n\r\n", Worker::http_code_get(500)).as_bytes().to_vec()
    }

    /// Reload lang and template
    #[cfg(any(feature = "html-reload", feature = "lang-reload"))]
    async fn reload(data: &ActionData) {
        #[cfg(feature = "html-reload")]
        Html::reload(Arc::clone(&data.html)).await;
        #[cfg(feature = "lang-reload")]
        Lang::reload(Arc::clone(&data.lang)).await;
    }

    fn get_header(capacity: usize, action: &Action, content_length: Option<usize>) -> Vec<u8> {
        let status = match action.request.version {
            HttpVersion::None => "Status:",
            HttpVersion::HTTP1_0 => "HTTP/1.0",
            HttpVersion::HTTP1_1 => "HTTP/1.1",
        };

        let mut answer: Vec<u8> = Vec::with_capacity(capacity);
        if let Some(redirect) = action.response.redirect.as_ref() {
            if redirect.permanently {
                answer
                    .extend_from_slice(format!("{status} 301 {}\r\nLocation: {}\r\n", Worker::http_code_get(301), redirect.url).as_bytes());
            } else {
                answer
                    .extend_from_slice(format!("{status} 302 {}\r\nLocation: {}\r\n", Worker::http_code_get(302), redirect.url).as_bytes());
            }
        } else if let Some(code) = action.response.http_code {
            answer.extend_from_slice(format!("{status} {} {}\r\n", code, Worker::http_code_get(code)).as_bytes());
        } else {
            answer.extend_from_slice(format!("{status} 200 {}\r\n", Worker::http_code_get(200)).as_bytes());
        }

        match &action.response.content_type {
            Some(content_type) => answer.extend_from_slice(format!("Content-Type: {}\r\n", content_type).as_bytes()),
            None => answer.extend_from_slice(b"Content-Type: text/html; charset=utf-8\r\n"),
        }
        answer.extend_from_slice(b"Connection: Keep-Alive\r\n");
        for (name, val) in &action.response.headers {
            answer.extend_from_slice(format!("{}: {}\r\n", name, val).as_bytes());
        }
        if let Some(len) = content_length {
            answer.extend_from_slice(format!("Content-Length: {}\r\n", len).as_bytes());
        }
        answer.extend_from_slice(b"\r\n");

        answer
    }

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
}
