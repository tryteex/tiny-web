use std::{cmp::min, collections::HashMap, sync::Arc};

use percent_encoding::percent_decode_str;

use crate::sys::{
    action::ActionData,
    request::{HttpMethod, HttpVersion, Input, RawData, Request, WebFile},
    worker::{StreamRead, StreamWrite, Worker, WorkerData},
};

pub const UWSGI_LEN_PACKAGE_SIZE: usize = 4;

/// UWSGI protocol
pub(crate) struct Net;
/// Alias for UWSGI protocol
type Uwsgi = Net;

impl Net {
    /// The entry point in the UWSGI protocol
    pub async fn run(mut stream_read: StreamRead, stream_write: Arc<StreamWrite>, data: WorkerData) {
        loop {
            // Check package size
            let mut buf = stream_read.get(UWSGI_LEN_PACKAGE_SIZE);
            while buf.len() < UWSGI_LEN_PACKAGE_SIZE {
                if stream_read.read(0).await.is_err() {
                    return;
                }
                buf = stream_read.get(UWSGI_LEN_PACKAGE_SIZE);
            }
            // Check header
            if unsafe { *buf.get_unchecked(0) != 0 || *buf.get_unchecked(3) != 0 } {
                return;
            }
            // Get package length
            let packet_len = u16::from_le_bytes(unsafe { [*buf.get_unchecked(1), *buf.get_unchecked(2)] }) as usize;
            stream_read.shift(UWSGI_LEN_PACKAGE_SIZE);
            // Reads header
            let (mut request, content_type, session, content_len) =
                match Uwsgi::read_header(&mut stream_read, packet_len, Arc::clone(&data.session_key)).await {
                    Some(c) => c,
                    None => return,
                };

            // Reads POST data
            let (post, file, raw) = match Uwsgi::read_input(&mut stream_read, content_type, content_len).await {
                Some(c) => c,
                None => return,
            };
            request.input.file = file;
            request.input.post = post;
            request.input.raw = raw;

            let stop = match data.stop {
                Some((ref rpc, stop, ref path)) => Some((Arc::clone(rpc), stop, Arc::clone(path))),
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
        }
    }

    pub fn write(answer: Vec<u8>, _end: bool) -> Vec<u8> {
        answer
    }

    /// Read post and file datas from UWSGI record.
    ///
    /// # Return
    ///
    /// * `HashMap<String, String>` - Post data.
    /// * `HashMap<String, Vec<WebFile>>` - File data.
    async fn read_input(
        stream: &mut StreamRead,
        content_type: Option<String>,
        mut content_len: usize,
    ) -> Option<(HashMap<String, String>, HashMap<String, Vec<WebFile>>, RawData)> {
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
        Some(Worker::read_input(data, content_type).await)
    }

    /// Read params from UWSGI header
    ///
    /// # Return
    ///
    /// * `Request` - Request struct for web engine.
    /// * `Option<String>` - CONTENT_TYPE parameter for recognizing FASTCGI_STDIN.
    /// * `Option<String>` - key of session.
    /// * `usize` - key of session.
    async fn read_header(
        stream: &mut StreamRead,
        mut packet_len: usize,
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
        let mut param_size = 0;
        let mut value: Vec<u8> = Vec::with_capacity(1024);
        let mut is_value = false;
        let mut value_size = 0;
        let mut params = HashMap::with_capacity(16);

        let mut max_read;
        let mut buf;
        let mut buf_len;
        while packet_len > 0 {
            // Reads data to the buffer
            max_read = min(packet_len, stream.available());
            while max_read == 0 {
                if stream.read(300).await.is_err() {
                    return None;
                }
                max_read = min(packet_len, stream.available());
            }
            if !is_param {
                // Search params
                if param_size == 0 {
                    while max_read < 2 {
                        if stream.read(300).await.is_err() {
                            return None;
                        }
                        max_read = min(packet_len, stream.available());
                    }
                    buf = stream.get(2);
                    param_size = u16::from_le_bytes([unsafe { *buf.get_unchecked(0) }, unsafe { *buf.get_unchecked(1) }]) as usize;
                    if param_size == 0 {
                        return None;
                    }
                    packet_len -= 2;
                    stream.shift(2);
                    continue;
                }
                if param_size <= max_read {
                    buf = stream.get(param_size);
                    buf_len = buf.len();
                    param.extend_from_slice(buf);
                    stream.shift(buf_len);
                    packet_len -= buf_len;
                    param_size -= buf_len;
                    if param_size == 0 {
                        is_param = true;
                    }
                } else {
                    buf = stream.get(max_read);
                    buf_len = buf.len();
                    param.extend_from_slice(buf);
                    stream.shift(buf_len);
                    packet_len -= buf_len;
                    param_size -= buf_len;
                }
            } else if !is_value {
                // Search values
                if value_size == 0 {
                    while max_read < 2 {
                        if stream.read(300).await.is_err() {
                            return None;
                        }
                        max_read = min(packet_len, stream.available());
                    }
                    buf = stream.get(2);
                    value_size = u16::from_le_bytes([unsafe { *buf.get_unchecked(0) }, unsafe { *buf.get_unchecked(1) }]) as usize;
                    packet_len -= 2;
                    stream.shift(2);
                    if value_size == 0 {
                        is_value = true;
                    }
                    continue;
                }
                if value_size <= max_read {
                    buf = stream.get(value_size);
                    buf_len = buf.len();
                    value.extend_from_slice(buf);
                    stream.shift(buf_len);
                    packet_len -= buf_len;
                    value_size -= buf_len;
                    if value_size == 0 {
                        is_value = true;
                    }
                } else {
                    buf = stream.get(max_read);
                    buf_len = buf.len();
                    value.extend_from_slice(buf);
                    stream.shift(buf_len);
                    packet_len -= buf_len;
                    value_size -= buf_len;
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
                            if key.len() == 2 {
                                if unsafe { *key.get_unchecked(0) } == session.as_str() {
                                    let val = unsafe { *key.get_unchecked(1) };
                                    if val.len() == 128 {
                                        for b in val.as_bytes() {
                                            if !((*b > 47 && *b < 58) || (*b > 96 && *b < 103)) {
                                                continue;
                                            }
                                        }
                                        session_key = Some(unsafe { *key.get_unchecked(1) }.to_owned());
                                    }
                                } else {
                                    cookie.insert(unsafe { *key.get_unchecked(0) }.to_owned(), unsafe { *key.get_unchecked(1) }.to_owned());
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
                    raw: RawData::None,
                },
                site,
                version: HttpVersion::None,
            },
            content_type,
            session_key,
            content_len,
        ))
    }
}
