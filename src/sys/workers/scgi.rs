use std::{cmp::min, collections::HashMap, sync::Arc};

use crate::sys::{
    action::{ActionData, Input, Request, WebFile},
    worker::{StreamRead, StreamWrite, Worker, WorkerData},
};

pub const SCGI_LEN_PACKAGE_SIZE: usize = 7;

/// SCGI protocol
pub(crate) struct Net;
/// Alias for SCGI protocol
type Scgi = Net;

impl Net {
    /// The entry point in the SCGI protocol
    pub async fn run(mut stream_read: StreamRead, stream_write: Arc<StreamWrite>, data: WorkerData) {
        // Check package size
        let mut buf = stream_read.get(SCGI_LEN_PACKAGE_SIZE);
        while buf.len() < SCGI_LEN_PACKAGE_SIZE {
            if stream_read.read(1000).await.is_err() {
                return;
            }
            buf = stream_read.get(SCGI_LEN_PACKAGE_SIZE);
        }

        // Read package separators
        let shift = match Scgi::read_separator(buf) {
            Some(shift) => {
                if shift == 0 {
                    return;
                }
                shift
            }
            None => return,
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
        let (mut request, content_type, session, content_len) =
            match Scgi::read_header(&mut stream_read, header_len, Arc::clone(&data.session_key)).await {
                Some(c) => c,
                None => return,
            };

        // Reads POST data
        let (post, file) = match Scgi::read_input(&mut stream_read, content_type, content_len).await {
            Some(c) => c,
            None => return,
        };
        request.input.file = file;
        request.input.post = post;

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
            stop: data.stop,
        };

        // Run main controller
        let answer = Worker::call_action(data).await;
        stream_write.write(answer).await;
    }

    pub fn write(answer: Vec<u8>, _end: bool) -> Vec<u8> {
        answer
    }

    /// Read post and file datas from SCGI record.
    ///
    /// # Return
    ///
    /// * `HashMap<String, String>` - Post data.
    /// * `HashMap<String, Vec<WebFile>>` - File data.
    async fn read_input(
        stream: &mut StreamRead,
        content_type: Option<String>,
        mut content_len: usize,
    ) -> Option<(HashMap<String, String>, HashMap<String, Vec<WebFile>>)> {
        let mut data = Vec::with_capacity(content_len);
        let mut max_read;
        let mut buf;
        let mut buf_len;
        while content_len > 0 {
            max_read = min(content_len, stream.available());
            while max_read == 0 {
                if stream.read(1000).await.is_err() {
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
        Some(Worker::read_input(data, content_type).await)
    }

    /// Read params from SCGI header
    ///
    /// # Return
    ///
    /// * `Request` - Request struct for web engine.
    /// * `Option<String>` - CONTENT_TYPE parameter for recognizing FASTCGI_STDIN.
    /// * `Option<String>` - key of session.
    /// * `usize` - key of session.
    async fn read_header(
        stream: &mut StreamRead,
        mut header_len: usize,
        session: Arc<String>,
    ) -> Option<(Request, Option<String>, Option<String>, usize)> {
        let mut ajax = false;
        let mut host = String::new();
        let mut scheme = "https".to_owned();
        let mut agent = String::new();
        let mut referer = String::new();
        let mut ip = String::new();
        let mut method = String::new();
        let mut path = String::new();
        let mut url = String::new();

        let mut get = HashMap::new();
        let mut cookie = HashMap::new();
        let mut content_type = None;
        let mut session_key = None;

        let mut content_len = 0;
        let mut param: Vec<u8> = Vec::with_capacity(1024);
        let mut is_param = false;
        let mut value: Vec<u8> = Vec::with_capacity(1024);
        let mut is_value = false;
        let mut params = HashMap::with_capacity(16);
        let mut max_read;
        let mut buf;
        let mut buf_len;
        while header_len > 0 {
            max_read = min(header_len, stream.available());
            while max_read == 0 {
                if stream.read(1000).await.is_err() {
                    return None;
                }
                max_read = min(header_len, stream.available());
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
                        header_len -= buf_len + 1;
                        if header_len == 0 {
                            is_param = true;
                        }
                        continue;
                    }
                    None => {
                        buf_len = buf.len();
                        param.extend_from_slice(buf);
                        stream.shift(buf_len);
                        header_len -= buf_len;
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
                        header_len -= buf_len + 1;
                        is_value = true;
                    }
                    None => {
                        buf_len = buf.len();
                        value.extend_from_slice(buf);
                        stream.shift(buf_len);
                        header_len -= buf_len;
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
                    b"REMOTE_ADDR" => ip = val,
                    b"REQUEST_METHOD" => method = val,
                    b"DOCUMENT_ROOT" => path = val,
                    b"REDIRECT_URL" => {
                        if let Some(u) = val.split('?').next() {
                            u.clone_into(&mut url);
                        }
                    }
                    b"QUERY_STRING" => {
                        if !val.is_empty() {
                            let gets: Vec<&str> = val.split('&').collect();
                            get.reserve(gets.len());
                            for v in gets {
                                let key: Vec<&str> = v.splitn(2, '=').collect();
                                match key.len() {
                                    1 => get.insert(v.to_owned(), String::new()),
                                    _ => get.insert(unsafe { (*key.get_unchecked(0)).to_owned() }, unsafe {
                                        (*key.get_unchecked(1)).to_owned()
                                    }),
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
                            if key.len() == 2 {
                                if unsafe { *key.get_unchecked(0) } == session.as_str() {
                                    let val = unsafe { *key.get_unchecked(1) };
                                    if val.len() == 128 {
                                        for b in val.as_bytes() {
                                            if !((*b > 47 && *b < 58) || (*b > 96 && *b < 103)) {
                                                continue;
                                            }
                                        }
                                        session_key = Some(unsafe { *key.get_unchecked(0) }.to_owned());
                                    }
                                } else {
                                    cookie.insert(
                                        unsafe { *key.get_unchecked(0) }.to_owned(),
                                        unsafe { *key.get_unchecked(0) }.to_owned(),
                                    );
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
        Some((
            Request {
                ajax,
                host,
                scheme,
                agent,
                referer,
                ip,
                method,
                path,
                url,
                input: Input {
                    get,
                    post: HashMap::new(),
                    file: HashMap::new(),
                    cookie,
                    params,
                },
            },
            content_type,
            session_key,
            content_len,
        ))
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
