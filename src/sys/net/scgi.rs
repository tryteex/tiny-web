use std::{
    cmp::min,
    collections::HashMap,
    net::IpAddr,
    sync::{atomic::Ordering, Arc},
};

use percent_encoding::percent_decode_str;

use super::{
    stream::{StreamError, StreamRead, StreamWrite},
    worker::{Worker, WorkerData},
};
use crate::{
    log,
    sys::web::{
        action::ActionData,
        request::{HttpMethod, HttpVersion, Input, RawData, Request, WebFile},
    },
};

const SCGI_LEN_PACKAGE_SIZE: usize = 7;

struct ScgiParam {
    request: Request,
    content_len: usize,
    #[cfg(any(feature = "session-memory", feature = "session-file", feature = "session-db"))]
    session: Option<String>,
}

struct ScgiArg {
    header_len: usize,
    #[cfg(any(feature = "session-memory", feature = "session-file", feature = "session-db"))]
    session_key: Arc<String>,
}

/// SCGI protocol
pub(super) struct Scgi;

impl Scgi {
    /// The entry point in the SCGI protocol
    pub async fn run(mut stream_read: StreamRead, stream_write: Arc<StreamWrite>, data: WorkerData) {
        if let Err(e) = stream_read.read(0).await {
            match e {
                StreamError::Closed => {}
                _e => {
                    log!(warning, 0, "{}", _e);
                }
            }
            return;
        }

        let id = data.mon.total.fetch_add(1, Ordering::Relaxed);
        let online = Arc::clone(&data.mon.online);
        online.fetch_add(1, Ordering::Relaxed);

        // Check package size
        let mut buf = stream_read.get(SCGI_LEN_PACKAGE_SIZE);
        while buf.len() < SCGI_LEN_PACKAGE_SIZE {
            if stream_read.read(300).await.is_err() {
                online.fetch_sub(1, Ordering::Relaxed);
                return;
            }
            buf = stream_read.get(SCGI_LEN_PACKAGE_SIZE);
        }

        // Read package separators
        let shift = match Scgi::read_separator(buf) {
            Some(shift) => {
                if shift == 0 {
                    online.fetch_sub(1, Ordering::Relaxed);
                    return;
                }
                shift
            }
            None => {
                online.fetch_sub(1, Ordering::Relaxed);
                return;
            }
        };

        // Get package
        let header_len = unsafe {
            match shift {
                1 => *buf.get_unchecked(0) as usize - 0x30,
                2 => 10 * (*buf.get_unchecked(0) as usize - 0x30) + (*buf.get_unchecked(1) as usize - 0x30),
                3 => {
                    100 * (*buf.get_unchecked(0) as usize - 0x30)
                        + 10 * (*buf.get_unchecked(1) as usize - 0x30)
                        + (*buf.get_unchecked(2) as usize - 0x30)
                }
                4 => {
                    1000 * (*buf.get_unchecked(0) as usize - 0x30)
                        + 100 * (*buf.get_unchecked(1) as usize - 0x30)
                        + 10 * (*buf.get_unchecked(2) as usize - 0x30)
                        + (*buf.get_unchecked(3) as usize - 0x30)
                }
                5 => {
                    10000 * (*buf.get_unchecked(0) as usize - 0x30)
                        + 1000 * (*buf.get_unchecked(1) as usize - 0x30)
                        + 100 * (*buf.get_unchecked(2) as usize - 0x30)
                        + 10 * (*buf.get_unchecked(3) as usize - 0x30)
                        + (*buf.get_unchecked(4) as usize - 0x30)
                }
                6 => {
                    100000 * (*buf.get_unchecked(0) as usize - 0x30)
                        + 10000 * (*buf.get_unchecked(1) as usize - 0x30)
                        + 1000 * (*buf.get_unchecked(2) as usize - 0x30)
                        + 100 * (*buf.get_unchecked(3) as usize - 0x30)
                        + 10 * (*buf.get_unchecked(4) as usize - 0x30)
                        + (*buf.get_unchecked(5) as usize - 0x30)
                }
                7 => {
                    1000000 * (*buf.get_unchecked(0) as usize - 0x30)
                        + 100000 * (*buf.get_unchecked(1) as usize - 0x30)
                        + 10000 * (*buf.get_unchecked(2) as usize - 0x30)
                        + 1000 * (*buf.get_unchecked(3) as usize - 0x30)
                        + 100 * (*buf.get_unchecked(4) as usize - 0x30)
                        + 10 * (*buf.get_unchecked(5) as usize - 0x30)
                        + (*buf.get_unchecked(6) as usize - 0x30)
                }
                _ => return,
            }
        };
        stream_read.shift(shift + 1);
        // Reads header
        let arg = ScgiArg {
            header_len,
            #[cfg(any(feature = "session-memory", feature = "session-file", feature = "session-db"))]
            session_key: Arc::clone(&data.session.session_key),
        };
        let param = match Scgi::read_header(&mut stream_read, arg).await {
            Some(c) => c,
            None => {
                online.fetch_sub(1, Ordering::Relaxed);
                return;
            }
        };
        // Reads POST data
        let (post, file, raw) = match Scgi::read_input(&mut stream_read, &param.request.content_type, param.content_len).await {
            Some(c) => c,
            None => {
                online.fetch_sub(1, Ordering::Relaxed);
                return;
            }
        };
        let mut request = param.request;
        request.input.file = Arc::new(file);
        request.input.post = Arc::new(post);
        request.input.raw = Arc::new(raw);

        let data = ActionData {
            id,
            mon: data.mon,
            engine: data.engine,
            salt: data.salt,
            request,
            tx: Arc::clone(&stream_write.tx),
            index: data.index,
            not_found: data.not_found.clone(),
            #[cfg(any(feature = "pgsql", feature = "mssql"))]
            db: data.db,
            #[cfg(any(feature = "html-static", feature = "html-reload"))]
            html: data.html,
            #[cfg(any(feature = "session-memory", feature = "session-file", feature = "session-db"))]
            session_loader: data.session,
            #[cfg(any(feature = "session-memory", feature = "session-file", feature = "session-db"))]
            session: param.session,
            #[cfg(any(feature = "lang-static", feature = "lang-reload"))]
            lang: data.lang,
            #[cfg(any(feature = "mail-sendmail", feature = "mail-smtp", feature = "mail-file"))]
            mail: data.mail,
            #[cfg(feature = "cache")]
            cache: data.cache,
        };

        // Run main controller
        let answer = Worker::call_action(data).await;
        stream_write.write(answer).await;

        online.fetch_sub(1, Ordering::Relaxed);
    }

    /// Read post and file datas from SCGI record.
    ///
    /// # Return
    ///
    /// * `HashMap<String, String>` - Post data.
    /// * `HashMap<String, Vec<WebFile>>` - File data.
    async fn read_input(
        stream: &mut StreamRead,
        content_type: &Option<String>,
        mut content_len: usize,
    ) -> Option<(HashMap<String, String>, Vec<WebFile>, RawData)> {
        let mut data = Vec::with_capacity(content_len);
        let mut max_read;
        let mut buf;
        let mut buf_len;
        while content_len > 0 {
            max_read = min(content_len, stream.available());
            while max_read == 0 {
                if stream.read(300).await.is_err() {
                    return None;
                }
                max_read = min(content_len, stream.available());
            }
            buf = stream.get(max_read);
            buf_len = buf.len();
            data.extend_from_slice(buf);
            stream.shift(buf_len);
            content_len -= buf_len;
        }
        Some(Worker::read_input(data, content_type.as_deref()).await)
    }

    /// Read params from SCGI header
    ///
    /// # Return
    ///
    /// * `Request` - Request struct for web engine.
    /// * `Option<String>` - CONTENT_TYPE parameter for recognizing FASTCGI_STDIN.
    /// * `Option<String>` - key of session.
    /// * `usize` - key of session.
    async fn read_header(stream: &mut StreamRead, mut arg: ScgiArg) -> Option<ScgiParam> {
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

        let mut content_len = 0;
        let mut param: Vec<u8> = Vec::with_capacity(1024);
        let mut is_param = false;
        let mut value: Vec<u8> = Vec::with_capacity(1024);
        let mut is_value = false;
        let mut params = HashMap::with_capacity(16);
        let mut max_read;
        let mut buf;
        let mut buf_len;
        while arg.header_len > 0 {
            max_read = min(arg.header_len, stream.available());
            while max_read == 0 {
                if stream.read(300).await.is_err() {
                    return None;
                }
                max_read = min(arg.header_len, stream.available());
            }
            if !is_param {
                // Read param
                buf = stream.get(max_read);
                match Scgi::read_next(buf) {
                    Some(found) => {
                        buf = stream.get(found);
                        buf_len = buf.len();
                        param.extend_from_slice(buf);
                        stream.shift(buf_len + 1);
                        arg.header_len -= buf_len + 1;
                        if arg.header_len == 0 {
                            is_param = true;
                        }
                        continue;
                    }
                    None => {
                        buf_len = buf.len();
                        param.extend_from_slice(buf);
                        stream.shift(buf_len);
                        arg.header_len -= buf_len;
                        continue;
                    }
                }
            } else if !is_value {
                // Read values
                buf = stream.get(max_read);
                match Scgi::read_next(buf) {
                    Some(found) => {
                        buf = stream.get(found);
                        buf_len = buf.len();
                        value.extend_from_slice(buf);
                        stream.shift(buf_len + 1);
                        arg.header_len -= buf_len + 1;
                        is_value = true;
                    }
                    None => {
                        buf_len = buf.len();
                        value.extend_from_slice(buf);
                        stream.shift(buf_len);
                        arg.header_len -= buf_len;
                        continue;
                    }
                }
            } else {
                let key = param.clone();
                let val = match String::from_utf8(value.clone()) {
                    Ok(value) => value,
                    Err(_) => return None,
                };
                match key.as_slice() {
                    b"CONTENT_LENGTH" => {
                        if let Ok(c) = val.parse::<usize>() {
                            content_len = c;
                        }
                    }
                    b"HTTP_X_REQUESTED_WITH" => ajax = val.to_lowercase().eq("xmlhttprequest"),
                    b"HTTP_HOST" => host = val,
                    b"REQUEST_SCHEME" => scheme = val,
                    b"HTTP_USER_AGENT" => agent = val,
                    b"HTTP_REFERER" => referer = val,
                    b"REMOTE_ADDR" => {
                        if let Ok(addr) = val.parse::<IpAddr>() {
                            ip = Some(addr);
                        }
                    }
                    b"REQUEST_METHOD" => method = val,
                    b"DOCUMENT_ROOT" => path = val,
                    b"REDIRECT_URL" => {
                        if let Some(u) = val.split('?').next() {
                            if let Ok(u) = percent_decode_str(u).decode_utf8() {
                                url = u.to_string();
                            }
                        }
                    }
                    b"QUERY_STRING" => {
                        if !val.is_empty() {
                            let gets: Vec<&str> = val.split('&').collect();
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
                    b"CONTENT_TYPE" => content_type = Some(val),
                    b"HTTP_COOKIE" => {
                        let cooks: Vec<&str> = val.split("; ").collect();
                        cookie.reserve(cooks.len());
                        for v in cooks {
                            let key: Vec<&str> = v.splitn(2, '=').collect();

                            #[cfg(not(any(feature = "session-memory", feature = "session-file", feature = "session-db")))]
                            if key.len() == 2 {
                                cookie.insert((*unsafe { key.get_unchecked(0) }).to_owned(), (*unsafe { key.get_unchecked(1) }).to_owned());
                            }
                            #[cfg(any(feature = "session-memory", feature = "session-file", feature = "session-db"))]
                            if key.len() == 2 {
                                if unsafe { *key.get_unchecked(0) } == arg.session_key.as_str() {
                                    let val = unsafe { *key.get_unchecked(1) };
                                    if val.len() == 128 {
                                        for b in val.as_bytes() {
                                            if !((*b > 47 && *b < 58) || (*b > 96 && *b < 103)) {
                                                continue;
                                            }
                                        }
                                        session = Some(unsafe { *key.get_unchecked(0) }.to_owned());
                                    }
                                } else {
                                    cookie.insert(unsafe { *key.get_unchecked(0) }.to_owned(), unsafe { *key.get_unchecked(0) }.to_owned());
                                }
                            }
                        }
                    }
                    _ => {
                        let key = match String::from_utf8(key) {
                            Ok(key) => key,
                            Err(_) => return None,
                        };
                        params.insert(key, val);
                    }
                }
                param.clear();
                is_param = false;
                value.clear();
                is_value = false;
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

        Some(ScgiParam {
            request,
            content_len,
            #[cfg(any(feature = "session-memory", feature = "session-file", feature = "session-db"))]
            session,
        })
    }

    /// Search for the first character "0"
    fn read_next(buf: &[u8]) -> Option<usize> {
        for (i, byte) in buf.iter().enumerate() {
            if *byte == 0 {
                return Some(i);
            }
        }
        None
    }

    /// Search for the first character ":"
    fn read_separator(buf: &[u8]) -> Option<usize> {
        for (i, byte) in buf.iter().enumerate() {
            if *byte == 0x3a {
                return Some(i);
            }
            // Only digit
            if *byte < 0x30 || *byte > 0x39 {
                return None;
            }
        }
        None
    }
}
