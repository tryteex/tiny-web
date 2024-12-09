use std::{
    cmp::min,
    collections::HashMap,
    net::IpAddr,
    sync::{atomic::Ordering, Arc},
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

#[derive(Debug)]
struct Header {
    /// FastCGI header type.
    pub header_type: u8,
    /// Content length.
    pub content_length: u16,
    /// Padding length.
    pub padding_length: u8,
}

/// Describes one record in the FastCGI protocol
#[derive(Debug)]
struct Record {
    /// FastCGI header.
    pub header: Header,
    /// Data.
    pub data: Vec<u8>,
}

#[derive(Debug)]
enum RecordType {
    /// Some FastCGI value.
    Some(Record),
    /// The stream was closed.
    StreamClose,
}

struct FastCGIParam {
    request: Request,
    #[cfg(any(feature = "session-memory", feature = "session-file", feature = "session-db"))]
    session: Option<String>,
}

struct FastCGIArg {
    data: Vec<u8>,
    #[cfg(any(feature = "session-memory", feature = "session-file", feature = "session-db"))]
    session_key: Arc<String>,
}

/// FastCGI header length
pub const FASTCGI_HEADER_LEN: usize = 8;
/// FastCGI max content length in the record
pub const FASTCGI_MAX_CONTENT_LEN: usize = 65535;

/// FastCGI header type BEGIN_REQUEST
pub const FASTCGI_BEGIN_REQUEST: u8 = 1;
/// FastCGI header type END_REQUEST
pub const FASTCGI_END_REQUEST: u8 = 3;
/// FastCGI header type PARAMS
pub const FASTCGI_PARAMS: u8 = 4;
/// FastCGI header type STDIN
pub const FASTCGI_STDIN: u8 = 5;
/// FastCGI header type STDOUT
pub const FASTCGI_STDOUT: u8 = 6;

/// The value of 1 in the Big Endian u16 format
pub const U16_BE_1: [u8; 2] = u16::to_be_bytes(1);
/// The value of 8 in the Big Endian u16 format
pub const U16_BE_8: [u8; 2] = u16::to_be_bytes(8);

// FastCGI protocol
pub(super) struct FastCGI;

impl FastCGI {
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

            // Gets one Record
            let record = match FastCGI::read_record_raw(&mut stream_read, 0).await {
                RecordType::Some(r) => r,
                RecordType::StreamClose => {
                    online.fetch_sub(1, Ordering::Relaxed);
                    break;
                }
            };
            // Start parsing the protocol, only if it starts with BEGIN_REQUEST
            if FASTCGI_BEGIN_REQUEST == record.header.header_type {
                let mut is_param_done = false;
                let mut is_stdin_done = false;
                let mut params = Vec::new();
                let mut stdin = Vec::new();

                // Loop until empty records PARAMS and STDIN are received
                loop {
                    // Gets next Record
                    let record = match FastCGI::read_record_raw(&mut stream_read, 300).await {
                        RecordType::Some(r) => r,
                        RecordType::StreamClose => {
                            online.fetch_sub(1, Ordering::Relaxed);
                            break;
                        }
                    };
                    match record.header.header_type {
                        FASTCGI_PARAMS => {
                            if record.data.is_empty() {
                                is_param_done = true;
                            } else {
                                params.extend_from_slice(&record.data);
                            }
                        }
                        FASTCGI_STDIN => {
                            if record.data.is_empty() {
                                is_stdin_done = true;
                            } else {
                                stdin.extend_from_slice(&record.data);
                            }
                        }
                        _ => {
                            online.fetch_sub(1, Ordering::Relaxed);
                            return;
                        }
                    }
                    if is_stdin_done && is_param_done {
                        online.fetch_sub(1, Ordering::Relaxed);
                        break;
                    }
                }
                // Reads params
                let arg = FastCGIArg {
                    data: params,
                    #[cfg(any(feature = "session-memory", feature = "session-file", feature = "session-db"))]
                    session_key: Arc::clone(&data.session.session_key),
                };

                let param = FastCGI::read_param(arg);

                // Reads POST data
                let (post, file, raw) = Worker::read_input(stdin, param.request.content_type.as_deref()).await;
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

                // Run main controller
                let answer = Worker::call_action(data).await;
                stream_write.write(answer).await;
                online.fetch_sub(1, Ordering::Relaxed);
            } else {
                online.fetch_sub(1, Ordering::Relaxed);
                break;
            }
        }
    }

    /// Read params from FastCGI record
    fn read_param(mut arg: FastCGIArg) -> FastCGIParam {
        let mut params = HashMap::with_capacity(16);
        let len = arg.data.len();
        let mut size = 0;

        let mut ajax = false;
        let mut host = String::new();
        let mut scheme = "https".to_owned();
        let mut agent = String::new();
        let mut referer = String::new();
        let mut ip = None;
        let mut method = String::new();
        let mut path = String::new();
        let mut url = String::new();

        let mut get = HashMap::new();
        let mut cookie = HashMap::new();
        let mut content_type = None;
        #[cfg(any(feature = "session-memory", feature = "session-file", feature = "session-db"))]
        let mut session = None;

        while size < len {
            let key_len;
            let value_len;

            // FastCGI transmits a name-value pair as the length of the name,
            // followed by the length of the value, followed by the name, followed by the value.
            //
            // Lengths of 127 bytes and less can be encoded in one byte,
            // while longer lengths are always encoded in four bytes

            // We know that size < len, so tiny optimization
            unsafe {
                // Gets key length
                if (*arg.data.get_unchecked(size) >> 7) == 0 {
                    if size + 1 > len {
                        break;
                    }
                    key_len = usize::from(*arg.data.get_unchecked(size));
                    size += 1;
                } else {
                    if size + 4 > len {
                        break;
                    }
                    let elem = arg.data.get_unchecked_mut(size);
                    *elem &= 0x7F;
                    key_len = u32::from_be_bytes([
                        *arg.data.get_unchecked(size),
                        *arg.data.get_unchecked(size + 1),
                        *arg.data.get_unchecked(size + 2),
                        *arg.data.get_unchecked(size + 3),
                    ]) as usize;
                    size += 4;
                }
                if key_len == 0 {
                    break;
                }
                // Gets value length
                if (*arg.data.get_unchecked(size) >> 7) == 0 {
                    if size + 1 > len {
                        break;
                    }
                    value_len = usize::from(*arg.data.get_unchecked(size));
                    size += 1;
                } else {
                    if size + 4 > len {
                        break;
                    }
                    let elem = arg.data.get_unchecked_mut(size);
                    *elem &= 0x7F;
                    value_len = u32::from_be_bytes([
                        *arg.data.get_unchecked(size),
                        *arg.data.get_unchecked(size + 1),
                        *arg.data.get_unchecked(size + 2),
                        *arg.data.get_unchecked(size + 3),
                    ]) as usize;
                    size += 4;
                }
                if size + key_len + value_len > len {
                    break;
                }
            }
            let key = unsafe { arg.data.get_unchecked(size..size + key_len) };
            size += key_len;
            let value = match String::from_utf8(unsafe { arg.data.get_unchecked(size..size + value_len) }.to_vec()) {
                Ok(value) => value,
                Err(_) => break,
            };
            size += value_len;
            // We will take some of the headers right away, and leave some for the user
            match key {
                b"HTTP_X_REQUESTED_WITH" => ajax = value.to_lowercase().eq("xmlhttprequest"),
                b"HTTP_HOST" => host = value,
                b"REQUEST_SCHEME" => scheme = value,
                b"HTTP_USER_AGENT" => agent = value,
                b"HTTP_REFERER" => referer = value,
                b"REMOTE_ADDR" => {
                    if let Ok(addr) = value.parse::<IpAddr>() {
                        ip = Some(addr);
                    }
                }
                b"REQUEST_METHOD" => method = value,
                b"DOCUMENT_ROOT" => path = value,
                b"REDIRECT_URL" => {
                    if let Some(u) = value.split('?').next() {
                        if let Ok(u) = percent_decode_str(u).decode_utf8() {
                            url = u.to_string();
                        }
                    }
                }
                b"QUERY_STRING" => {
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
                b"CONTENT_TYPE" => content_type = Some(value),
                b"HTTP_COOKIE" => {
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
                    let key = match String::from_utf8(key.to_vec()) {
                        Ok(key) => key,
                        Err(_) => break,
                    };
                    params.insert(key, value);
                }
            }
        }
        params.shrink_to_fit();
        let method = method.parse().unwrap_or(HttpMethod::Get);
        let site = format!("{}://{}", scheme, host);
        let request = Request {
            ajax,
            host,
            scheme,
            agent,
            referer,
            ip,
            method,
            root: Arc::new(path.into()),
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
            version: HttpVersion::None,
            content_type,
        };

        FastCGIParam {
            request,
            #[cfg(any(feature = "session-memory", feature = "session-file", feature = "session-db"))]
            session,
        }
    }

    /// Read one record from TcpStream
    async fn read_record_raw(stream: &mut StreamRead, timeout: u64) -> RecordType {
        // There is not enough buffer
        let mut buf = stream.get(stream.available());

        while buf.len() < FASTCGI_HEADER_LEN {
            if stream.read(timeout).await.is_err() {
                return RecordType::StreamClose;
            }
            buf = stream.get(stream.available());
        }

        let header = FastCGI::read_header(unsafe { buf.get_unchecked(..FASTCGI_HEADER_LEN) });
        stream.shift(FASTCGI_HEADER_LEN);
        let mut total = header.content_length as usize;

        // It is necessary to determine how much data is in the record,
        // if it is more than FASTCGI_MAX_CONTENT_LEN, then we read in several approaches
        let mut max_read;
        let mut buf_len;
        let mut vec = Vec::with_capacity(total);
        while total > 0 {
            max_read = min(total, stream.available());
            while max_read == 0 {
                if stream.read(300).await.is_err() {
                    return RecordType::StreamClose;
                }
                max_read = min(total, stream.available());
            }
            buf = stream.get(max_read);
            buf_len = buf.len();
            vec.extend_from_slice(buf);
            stream.shift(buf_len);
            total -= buf_len;
        }
        stream.shift(header.padding_length as usize);
        RecordType::Some(Record { header, data: vec })
    }

    /// Reads the FastCGI header
    ///
    /// # Safety
    ///
    /// You have to ensure that data length = FASTCGI_HEADER_LEN
    fn read_header(data: &[u8]) -> Header {
        unsafe {
            Header {
                header_type: *data.get_unchecked(1),
                content_length: u16::from_be_bytes([*data.get_unchecked(4), *data.get_unchecked(5)]),
                padding_length: *data.get_unchecked(6),
            }
        }
    }

    /// Writes answer to server
    pub fn write(answer: Vec<u8>, end: bool) -> Vec<u8> {
        let mut seek: usize = 0;
        let len = answer.len();
        let capacity = len + FASTCGI_HEADER_LEN * (4 + len / FASTCGI_MAX_CONTENT_LEN);

        let mut data: Vec<u8> = Vec::with_capacity(capacity);
        let mut size;

        // The maximum record size must not exceed FASTCGI_MAX_CONTENT_LEN
        while seek < len {
            if seek + FASTCGI_MAX_CONTENT_LEN < len {
                size = FASTCGI_MAX_CONTENT_LEN;
            } else {
                size = len - seek;
            };
            data.push(1_u8);
            data.push(FASTCGI_STDOUT);
            data.extend_from_slice(&U16_BE_1);
            data.extend_from_slice(&u16::to_be_bytes(size as u16));
            data.push(0);
            data.push(0);
            data.extend_from_slice(unsafe { answer.get_unchecked(seek..seek + size) });
            seek += size;
        }
        if end {
            // Empty FASTCGI_STDOUT
            data.push(1_u8);
            data.push(FASTCGI_STDOUT);
            data.extend_from_slice(&U16_BE_1);
            data.push(0);
            data.push(0);
            data.push(0);
            data.push(0);

            // Empty FASTCGI_END_REQUEST
            data.push(1_u8);
            data.push(FASTCGI_END_REQUEST);
            data.extend_from_slice(&U16_BE_1);
            data.extend_from_slice(&U16_BE_8);
            data.push(0);
            data.push(0);

            // FASTCGI_END_REQUEST data
            data.push(0);
            data.push(0);
            data.push(0);
            data.push(0);
            data.push(0);
            data.push(0);
            data.push(0);
            data.push(0);
        }
        data
    }
}
