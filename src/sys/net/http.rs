use core::str;

use std::{
    collections::HashMap,
    net::IpAddr,
    path::PathBuf,
    sync::{atomic::Ordering, Arc},
};

use std::{
    fmt::{Display, Formatter},
    str::Utf8Error,
};

use percent_encoding::percent_decode_str;

use crate::{
    log,
    sys::web::{
        action::ActionData,
        request::{HttpMethod, HttpVersion, Input, RawData, Request},
    },
};

use super::{
    stream::{StreamError, StreamRead, StreamWrite},
    worker::{Worker, WorkerData},
};

const HTTP_MIN_HEADER_LEN: usize = 18;

#[derive(Debug, Clone)]
struct Header {
    version: HttpVersion,
    method: HttpMethod,
    header: HashMap<String, String>,
    size: Option<usize>,
}

#[derive(Debug)]
enum StreamCloseError {
    Stream(StreamError),
    Utf8(Utf8Error),
    HttpProtocol(String),
    ContentLength(String),
    Header(String),
}

impl Display for StreamCloseError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            StreamCloseError::Stream(err) => write!(f, "Stream error: {}", err),
            StreamCloseError::Utf8(err) => write!(f, "UTF-8 error: {}", err),
            StreamCloseError::HttpProtocol(msg) => write!(f, "HTTP protocol error: {}", msg),
            StreamCloseError::ContentLength(msg) => write!(f, "Content length error: {}", msg),
            StreamCloseError::Header(msg) => write!(f, "Header error: {}", msg),
        }
    }
}

enum RecordType {
    Some(Header),
    StreamClose(StreamCloseError),
}

struct HttpParam {
    request: Request,
    #[cfg(any(feature = "session-memory", feature = "session-file", feature = "session-db"))]
    session: Option<String>,
}

struct HttpArg {
    remote_ip: Option<IpAddr>,
    root: Arc<PathBuf>,
    #[cfg(any(feature = "session-memory", feature = "session-file", feature = "session-db"))]
    session_key: Arc<String>,
}

pub(super) struct Http;

impl Http {
    pub(super) async fn run(mut stream_read: StreamRead, stream_write: Arc<StreamWrite>, data: WorkerData) {
        loop {
            if let Err(e) = stream_read.read(0).await {
                match e {
                    StreamError::Closed => {}
                    _e => {
                        log!(warning, 0, "{}", _e);
                    }
                }
                break;
            }

            let id = data.mon.total.fetch_add(1, Ordering::Relaxed);
            let online = Arc::clone(&data.mon.online);
            online.fetch_add(1, Ordering::Relaxed);

            let mut header = match Http::get_header(&mut stream_read).await {
                RecordType::Some(header) => header,
                RecordType::StreamClose(_e) => {
                    log!(warning, 0, "{}", _e);
                    online.fetch_sub(1, Ordering::Relaxed);
                    break;
                }
            };

            let body = match Http::get_body(&header, &mut stream_read).await {
                Ok(body) => body,
                Err(_e) => {
                    log!(warning, 0, "{}", _e);
                    online.fetch_sub(1, Ordering::Relaxed);
                    break;
                }
            };

            let arg = HttpArg {
                remote_ip: data.ip,
                root: Arc::clone(&data.root),
                #[cfg(any(feature = "session-memory", feature = "session-file", feature = "session-db"))]
                session_key: Arc::clone(&data.session.session_key),
            };
            let param = Http::read_param(&mut header, arg);

            // Reads POST data
            let (post, file, raw) = Worker::read_input(body, param.request.content_type.as_deref()).await;
            let mut request = param.request;

            request.input.file = Arc::new(file);
            request.input.post = Arc::new(post);
            request.input.raw = Arc::new(raw);

            let data = ActionData {
                id,
                mon: Arc::clone(&data.mon),
                engine: Arc::clone(&data.engine),
                salt: Arc::clone(&data.salt),
                request,
                tx: Arc::clone(&stream_write.tx),
                index: Arc::clone(&data.index),
                not_found: data.not_found.clone(),
                #[cfg(any(feature = "pgsql", feature = "mssql"))]
                db: Arc::clone(&data.db),
                #[cfg(any(feature = "html-static", feature = "html-reload"))]
                html: Arc::clone(&data.html),
                #[cfg(any(feature = "session-memory", feature = "session-file", feature = "session-db"))]
                session_loader: Arc::clone(&data.session),
                #[cfg(any(feature = "session-memory", feature = "session-file", feature = "session-db"))]
                session: param.session,
                #[cfg(any(feature = "lang-static", feature = "lang-reload"))]
                lang: Arc::clone(&data.lang),
                #[cfg(any(feature = "mail-sendmail", feature = "mail-smtp", feature = "mail-file"))]
                mail: Arc::clone(&data.mail),
                #[cfg(feature = "cache")]
                cache: Arc::clone(&data.cache),
            };
            let answer = Worker::call_action(data).await;
            // Run main controller
            stream_write.write(answer).await;

            #[cfg(any(feature = "session-memory", feature = "session-file", feature = "session-db"))]
            online.fetch_sub(1, Ordering::Relaxed);
            if header.version == HttpVersion::HTTP1_0 {
                break;
            }
        }
    }

    async fn get_header(stream: &mut StreamRead) -> RecordType {
        let mut buf = stream.get(stream.available());
        let mut found = false;
        let mut shift = 0;
        let mut b;

        let mut vec = Vec::with_capacity(32);
        let mut last = 0;
        let mut dotdot = 0;

        if buf.len() < HTTP_MIN_HEADER_LEN {
            if let Err(e) = stream.read(300).await {
                return RecordType::StreamClose(StreamCloseError::Stream(e));
            }
            buf = stream.get(stream.available());
        }
        while !found {
            while buf.len() - 4 < shift {
                if let Err(e) = stream.read(300).await {
                    return RecordType::StreamClose(StreamCloseError::Stream(e));
                }
                buf = stream.get(stream.available());
            }
            b = *unsafe { buf.get_unchecked(shift) };
            if b == b'\r' {
                if *unsafe { buf.get_unchecked(shift + 1) } == b'\n' {
                    if *unsafe { buf.get_unchecked(shift + 2) } == b'\r' {
                        if *unsafe { buf.get_unchecked(shift + 3) } == b'\n' {
                            found = true;
                            vec.push((last, shift, dotdot));
                        } else {
                            shift += 1;
                        }
                    } else {
                        vec.push((last, shift, dotdot));
                        dotdot = 0;
                        shift += 1;
                        last = shift + 1;
                    }
                } else {
                    shift += 1;
                }
            } else {
                shift += 1;
                if b == b':' && dotdot == 0 {
                    dotdot = shift;
                }
            }
        }

        let str: &str = match str::from_utf8(&buf[..shift]) {
            Ok(str) => str,
            Err(e) => return RecordType::StreamClose(StreamCloseError::Utf8(e)),
        };

        let mut head = Header {
            version: HttpVersion::None,
            method: HttpMethod::Get,
            header: HashMap::with_capacity(vec.len()),
            size: None,
        };
        let mut first = true;
        for (start, finish, dot) in vec {
            if dot == 0 && first {
                let mut parts = str[start..finish].split(' ');
                match parts.next() {
                    Some(method) => head.method = method.parse().unwrap_or(HttpMethod::Get),
                    None => return RecordType::StreamClose(StreamCloseError::HttpProtocol(str[start..finish].to_owned())),
                };
                match parts.next() {
                    Some(url) => head.header.insert("ORIGIN_URL".to_owned(), url.to_owned()),
                    None => return RecordType::StreamClose(StreamCloseError::HttpProtocol(str[start..finish].to_owned())),
                };
                match parts.next() {
                    Some(version) => match version {
                        "HTTP/1.0" => head.version = HttpVersion::HTTP1_0,
                        "HTTP/1.1" => head.version = HttpVersion::HTTP1_1,
                        _ => {
                            log!(warning, 0, "{}", version);
                        }
                    },
                    None => return RecordType::StreamClose(StreamCloseError::HttpProtocol(str[start..finish].to_owned())),
                };
                first = false;
            } else if dot > 0 {
                let key = str[start..dot - 1].to_uppercase();
                match key.as_str() {
                    "CONTENT-LENGTH" => match &str[dot + 1..finish].parse::<usize>() {
                        Ok(len) => {
                            if *len > 0 {
                                head.size = Some(*len);
                            }
                        }
                        Err(_) => return RecordType::StreamClose(StreamCloseError::ContentLength(str[dot + 1..finish].to_owned())),
                    },
                    _ => {
                        head.header.insert(key, str[dot + 1..finish].to_owned());
                    }
                }
            } else {
                return RecordType::StreamClose(StreamCloseError::Header(str[start..finish].to_owned()));
            }
        }
        stream.shift(shift + 4);
        RecordType::Some(head)
    }

    async fn get_body(header: &Header, stream: &mut StreamRead) -> Result<Vec<u8>, StreamError> {
        let body = match header.size {
            Some(mut size) => {
                let mut vec = Vec::with_capacity(size);
                let mut buf = stream.get(stream.available());
                while buf.len() < size {
                    if !buf.is_empty() {
                        vec.extend_from_slice(buf);
                        size -= buf.len();
                        stream.shift(buf.len());
                    }
                    stream.read(300).await?;
                    buf = stream.get(stream.available());
                }
                if !buf.is_empty() {
                    vec.extend_from_slice(&buf[..size]);
                    stream.shift(buf.len());
                }
                vec
            }
            None => Vec::new(),
        };
        Ok(body)
    }

    fn read_param(header: &mut Header, mut arg: HttpArg) -> HttpParam {
        let mut params = HashMap::with_capacity(16);

        let mut ajax = false;
        let mut host = String::new();
        let mut scheme = "http".to_owned();
        let mut agent = String::new();
        let mut referer = String::new();
        let mut ip = None;
        let mut url = String::new();
        let mut orig_url = String::new();

        let mut get = HashMap::new();
        let mut cookie = HashMap::new();
        let mut content_type = None;
        #[cfg(any(feature = "session-memory", feature = "session-file", feature = "session-db"))]
        let mut session = None;

        for (key, value) in header.header.drain() {
            match key.as_str() {
                "X-REQUESTED-WITH" => ajax = value.to_lowercase().eq("xmlhttprequest"),
                "HOST" => host = value,
                "X-FORWARDED-PROTO" => scheme = value,
                "USER-AGENT" => agent = value,
                "REFERER" => referer = value,
                "X-REAL-IP" => {
                    if let Ok(addr) = value.parse::<IpAddr>() {
                        ip = Some(addr);
                    }
                }
                "X-REQUEST-URI" => {
                    let mut list = value.split('?');
                    if let Some(u) = list.next() {
                        if let Ok(u) = percent_decode_str(u).decode_utf8() {
                            url = u.to_string();
                        }
                    }
                    if let Some(value) = list.next() {
                        if !value.is_empty() {
                            let gets: Vec<&str> = value.split('&').collect();
                            get.reserve(gets.len());
                            for v in gets {
                                let key: Vec<&str> = v.splitn(2, '=').collect();
                                match key.len() {
                                    1 => {
                                        if let Ok(u) = percent_decode_str(v).decode_utf8() {
                                            get.insert(u.to_string(), String::new());
                                        }
                                    }
                                    _ => {
                                        if let Ok(u) = percent_decode_str(unsafe { key.get_unchecked(0) }).decode_utf8() {
                                            if let Ok(v) = percent_decode_str(unsafe { key.get_unchecked(1) }).decode_utf8() {
                                                get.insert(u.to_string(), v.to_string());
                                            }
                                        }
                                    }
                                };
                            }
                        }
                    }
                }
                "ORIGIN_URL" => orig_url = value,
                "CONTENT-TYPE" => content_type = Some(value),
                "COOKIE" => {
                    let cooks: Vec<&str> = value.split("; ").collect();
                    cookie.reserve(cooks.len());
                    for v in cooks {
                        let key: Vec<&str> = v.splitn(2, '=').collect();
                        #[cfg(not(any(feature = "session-memory", feature = "session-file", feature = "session-db")))]
                        if key.len() == 2 {
                            cookie.insert((*unsafe { key.get_unchecked(0) }).to_owned(), (*unsafe { key.get_unchecked(1) }).to_owned());
                        }
                        #[cfg(any(feature = "session-memory", feature = "session-file", feature = "session-db"))]
                        if key.len() == 2 && *unsafe { key.get_unchecked(0) } == arg.session_key.as_str() {
                            let val = *unsafe { key.get_unchecked(1) };
                            if val.len() == 128 {
                                for b in val.as_bytes() {
                                    if !((*b > 47 && *b < 58) || (*b > 96 && *b < 103)) {
                                        continue;
                                    }
                                }
                                session = Some((*unsafe { key.get_unchecked(1) }).to_owned());
                            } else {
                                cookie.insert((*unsafe { key.get_unchecked(0) }).to_owned(), (*unsafe { key.get_unchecked(1) }).to_owned());
                            }
                        }
                    }
                }
                _ => {
                    params.insert(key, value);
                }
            }
        }

        if ip.is_none() {
            ip = arg.remote_ip.take();
        }
        if url.is_empty() {
            let mut list = orig_url.split('?');
            if let Some(u) = list.next() {
                if let Ok(u) = percent_decode_str(u).decode_utf8() {
                    url = u.to_string();
                }
            }
            if let Some(value) = list.next() {
                if !value.is_empty() {
                    let gets: Vec<&str> = value.split('&').collect();
                    get.reserve(gets.len());
                    for v in gets {
                        let key: Vec<&str> = v.splitn(2, '=').collect();
                        match key.len() {
                            1 => {
                                if let Ok(u) = percent_decode_str(v).decode_utf8() {
                                    get.insert(u.to_string(), String::new());
                                }
                            }
                            _ => {
                                if let Ok(u) = percent_decode_str(unsafe { key.get_unchecked(0) }).decode_utf8() {
                                    if let Ok(v) = percent_decode_str(unsafe { key.get_unchecked(1) }).decode_utf8() {
                                        get.insert(u.to_string(), v.to_string());
                                    }
                                }
                            }
                        };
                    }
                }
            }
        }
        let site = format!("{}://{}", scheme, host);
        let request = Request {
            ajax,
            host,
            scheme,
            agent,
            referer,
            ip,
            method: header.method.clone(),
            root: arg.root,
            url,
            input: Input {
                get: Arc::new(get),
                post: Arc::new(HashMap::new()),
                file: Arc::new(Vec::new()),
                cookie: Arc::new(cookie),
                params: Arc::new(params),
                raw: Arc::new(RawData::None),
            },
            site,
            version: header.version.clone(),
            content_type,
        };
        HttpParam {
            request,
            #[cfg(any(feature = "session-memory", feature = "session-file", feature = "session-db"))]
            session,
        }
    }
}
