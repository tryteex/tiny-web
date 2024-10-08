use core::str;
use std::{collections::HashMap, net::IpAddr, sync::Arc};

use percent_encoding::percent_decode_str;

use crate::sys::{
    action::ActionData,
    log::Log,
    request::{HttpMethod, HttpVersion, Input, RawData, Request},
    worker::{StreamRead, StreamWrite, Worker, WorkerData},
};

/// HTTP header
#[derive(Debug)]
struct Header {
    /// Http version
    version: HttpVersion,
    /// Http method
    method: HttpMethod,
    /// Headers
    header: HashMap<String, String>,
    /// Content length
    size: Option<usize>,
    /// Transfer-Encoding: chunked
    chunked: bool,
}

/// The record type when reading it from the stream
///
/// # Values
///
/// * `Some(Header)` - Some HTTP Header .
/// * `StreamClose` - The stream was closed.
#[derive(Debug)]
enum RecordType {
    /// Some FastCGI value.
    Some(Header),
    /// The stream was closed.
    StreamClose,
}

/// HTTP minimum header length
pub const HTTP_MIN_HEADER_LEN: usize = 18;

/// HTTP protocol
pub(crate) struct Net;

/// Alias for FastCGI protocol
type Http = Net;

impl Net {
    pub async fn run(mut stream_read: StreamRead, stream_write: Arc<StreamWrite>, data: WorkerData) {
        loop {
            let mut header = match Http::get_header(&mut stream_read, 0).await {
                RecordType::Some(header) => header,
                RecordType::StreamClose => break,
            };

            let body = match Http::get_body(&header, &mut stream_read).await {
                Ok(body) => body,
                Err(_) => break,
            };

            let (mut request, content_type, session) =
                Http::read_param(&mut header, Arc::clone(&data.session_key), &data.ip, Arc::clone(&data.root));

            // Reads POST data
            let (post, file, raw) = Worker::read_input(body, content_type).await;
            request.input.file = file;
            request.input.post = post;
            request.input.raw = raw;

            let stop = match data.stop {
                Some((ref rpc, stop)) => Some((Arc::clone(rpc), stop)),
                None => None,
            };

            let data = ActionData {
                engine: Arc::clone(&data.engine),
                lang: Arc::clone(&data.lang),
                html: Arc::clone(&data.html),
                cache: Arc::clone(&data.cache),
                db: Arc::clone(&data.db),
                session_key: Arc::clone(&data.session_key),
                salt: Arc::clone(&data.salt),
                mail: Arc::clone(&data.mail),
                request,
                session,
                tx: Arc::clone(&stream_write.tx),
                action_index: Arc::clone(&data.action_index),
                action_not_found: Arc::clone(&data.action_not_found),
                action_err: Arc::clone(&data.action_err),
                stop,
                root: Arc::clone(&data.root),
            };

            // Run main controller
            let answer = Worker::call_action(data).await;
            stream_write.write(answer).await;

            if header.version == HttpVersion::HTTP1_0 {
                break;
            }
        }
    }

    fn read_param(
        header: &mut Header,
        session: Arc<String>,
        remote_ip: &IpAddr,
        root: Arc<String>,
    ) -> (Request, Option<String>, Option<String>) {
        let mut params = HashMap::with_capacity(16);

        let mut ajax = false;
        let mut host = String::new();
        let mut scheme = "http".to_owned();
        let mut agent = String::new();
        let mut referer = String::new();
        let mut ip = String::new();
        let mut path = String::new();
        let mut url = String::new();
        let mut orig_url = String::new();

        let mut get = HashMap::new();
        let mut cookie = HashMap::new();
        let mut content_type = None;
        let mut session_key = None;

        for (key, value) in header.header.drain() {
            match key.as_str() {
                "X-REQUESTED-WITH" => ajax = value.to_lowercase().eq("xmlhttprequest"),
                "HOST" => host = value,
                "X-FORWARDED-PROTO" => scheme = value,
                "USER-AGENT" => agent = value,
                "REFERER" => referer = value,
                "X-REAL-IP" => ip = value,
                "X-DOCUMENT-ROOT" => path = value,
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
                        if key.len() == 2 && *unsafe { key.get_unchecked(0) } == session.as_str() {
                            let val = *unsafe { key.get_unchecked(1) };
                            if val.len() == 128 {
                                for b in val.as_bytes() {
                                    if !((*b > 47 && *b < 58) || (*b > 96 && *b < 103)) {
                                        continue;
                                    }
                                }
                                session_key = Some((*unsafe { key.get_unchecked(1) }).to_owned());
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

        if ip.is_empty() {
            ip = remote_ip.to_string();
        }
        if path.is_empty() {
            path = (*root).clone();
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
        (
            Request {
                ajax,
                host,
                scheme,
                agent,
                referer,
                ip,
                method: header.method.clone(),
                path,
                url,
                input: Input {
                    get,
                    post: HashMap::new(),
                    file: HashMap::new(),
                    cookie,
                    params,
                    raw: RawData::None,
                },
                site,
                version: header.version.clone(),
            },
            content_type,
            session_key,
        )
    }

    async fn get_body(header: &Header, stream: &mut StreamRead) -> Result<Vec<u8>, ()> {
        let body = match header.size {
            Some(mut size) => {
                let mut vec = Vec::with_capacity(size);
                let mut buf = stream.get(stream.available());
                if buf.len() < size {
                    if !buf.is_empty() {
                        vec.extend_from_slice(buf);
                        size -= buf.len();
                        stream.shift(buf.len());
                    }
                    if stream.read(300).await.is_err() {
                        return Err(());
                    }
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

    pub fn write(answer: Vec<u8>, _end: bool) -> Vec<u8> {
        answer
    }

    /// Read one request header
    async fn get_header(stream: &mut StreamRead, timeout: u64) -> RecordType {
        let mut buf = stream.get(stream.available());
        let mut found = false;
        let mut shift = 0;
        let mut b;

        let mut vec = Vec::with_capacity(32);
        let mut last = 0;
        let mut dotdot = 0;

        if buf.len() < HTTP_MIN_HEADER_LEN {
            if stream.read(timeout).await.is_err() {
                return RecordType::StreamClose;
            }
            buf = stream.get(stream.available());
        }
        while !found {
            while buf.len() - 4 < shift {
                if stream.read(300).await.is_err() {
                    return RecordType::StreamClose;
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
            Err(_) => return RecordType::StreamClose,
        };

        let mut head = Header {
            version: HttpVersion::None,
            method: HttpMethod::Get,
            header: HashMap::with_capacity(vec.len()),
            size: None,
            chunked: false,
        };
        let mut first = true;
        for (start, finish, dot) in vec {
            if dot == 0 && first {
                let mut parts = str[start..finish].split(' ');
                match parts.next() {
                    Some(method) => head.method = method.parse().unwrap_or(HttpMethod::Get),
                    None => return RecordType::StreamClose,
                };
                match parts.next() {
                    Some(url) => head.header.insert("ORIGIN_URL".to_owned(), url.to_owned()),
                    None => return RecordType::StreamClose,
                };
                match parts.next() {
                    Some(version) => match version {
                        "HTTP/1.0" => head.version = HttpVersion::HTTP1_0,
                        "HTTP/1.1" => head.version = HttpVersion::HTTP1_1,
                        //"HTTP/2" => head.version = HttpVersion::HTTP2,
                        _ => {
                            Log::warning(400, Some(version.to_owned()));
                        }
                    },
                    None => return RecordType::StreamClose,
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
                        Err(_) => return RecordType::StreamClose,
                    },
                    "TRANSFER-ENCODING" => head.chunked = &str[dot + 1..finish] == "chunked",
                    _ => {
                        head.header.insert(key, str[dot + 1..finish].to_owned());
                    }
                }
            } else {
                return RecordType::StreamClose;
            }
        }
        stream.shift(shift + 4);
        RecordType::Some(head)
    }
}
