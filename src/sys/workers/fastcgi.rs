use std::{cmp::min, collections::HashMap, sync::Arc};

use crate::sys::{
    action::{ActionData, Input, Request},
    worker::{StreamRead, StreamWrite, Worker, WorkerData},
};

/// Describes a header in a FastCGI record.
///
/// # Values
///
/// * `header_type: u8` - FastCGI header type.
/// * `content_length: u16` - Content length.
/// * `padding_length: u8` - Padding length.
#[derive(Debug)]
pub(crate) struct Header {
    /// FastCGI header type.
    pub header_type: u8,
    /// Content length.
    pub content_length: u16,
    /// Padding length.
    pub padding_length: u8,
}

/// Describes one record in the FastCGI protocol
///
/// # Values
///
/// * `header: Header` - FastCGI header.
/// * `data: Vec<u8>` - Data.
#[derive(Debug)]
pub(crate) struct Record {
    /// FastCGI header.
    pub header: Header,
    /// Data.
    pub data: Vec<u8>,
}

/// The record type when reading it from the stream
///
/// # Values
///
/// * `Some(Record)` - Some FastCGI value.
/// * `Error(Vec<u8>)` - Error recognizing FastCGI record.
/// * `StreamClose` - The stream was closed.
#[derive(Debug)]
pub(crate) enum RecordType {
    /// Some FastCGI value.
    Some(Record),
    /// The stream was closed.
    StreamClose,
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

/// FastCGI protocol
pub(crate) struct Net;
/// Alias for FastCGI protocol
type FastCGI = Net;

impl Net {
    /// The entry point in the FastCGI protocol
    pub async fn run(mut stream_read: StreamRead, stream_write: Arc<StreamWrite>, data: WorkerData) {
        loop {
            // Gets one Record
            let record = match FastCGI::read_record_raw(&mut stream_read, 0).await {
                RecordType::Some(r) => r,
                RecordType::StreamClose => break,
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
                    let record = match FastCGI::read_record_raw(&mut stream_read, 1000).await {
                        RecordType::Some(r) => r,
                        RecordType::StreamClose => break,
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
                        _ => return,
                    }
                    if is_stdin_done && is_param_done {
                        break;
                    }
                }
                // Reads params
                let (mut request, content_type, session) = FastCGI::read_param(params, Arc::clone(&data.session_key));

                // Reads POST data
                let (post, file) = Worker::read_input(stdin, content_type).await;
                request.input.file = file;
                request.input.post = post;

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
                };

                // Run main controller
                let answer = Worker::call_action(data).await;
                stream_write.write(answer).await;
            } else {
                break;
            }
        }
    }

    /// Read params from FastCGI record
    ///
    /// # Return
    ///
    /// * `Request` - Request struct for web engine.
    /// * `Option<String>` - CONTENT_TYPE parameter for recognizing FASTCGI_STDIN.
    /// * `Option<String>` - key of session.
    fn read_param(mut data: Vec<u8>, session: Arc<String>) -> (Request, Option<String>, Option<String>) {
        let mut params = HashMap::with_capacity(16);
        let len = data.len();
        let mut size = 0;

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
                if (*data.get_unchecked(size) >> 7) == 0 {
                    if size + 1 > len {
                        break;
                    }
                    key_len = usize::from(*data.get_unchecked(size));
                    size += 1;
                } else {
                    if size + 4 > len {
                        break;
                    }
                    let elem = data.get_unchecked_mut(size);
                    *elem &= 0x7F;
                    key_len = u32::from_be_bytes([
                        *data.get_unchecked(size),
                        *data.get_unchecked(size + 1),
                        *data.get_unchecked(size + 2),
                        *data.get_unchecked(size + 3),
                    ]) as usize;
                    size += 4;
                }
                if key_len == 0 {
                    break;
                }
                // Gets value length
                if (*data.get_unchecked(size) >> 7) == 0 {
                    if size + 1 > len {
                        break;
                    }
                    value_len = usize::from(*data.get_unchecked(size));
                    size += 1;
                } else {
                    if size + 4 > len {
                        break;
                    }
                    let elem = data.get_unchecked_mut(size);
                    *elem &= 0x7F;
                    value_len = u32::from_be_bytes([
                        *data.get_unchecked(size),
                        *data.get_unchecked(size + 1),
                        *data.get_unchecked(size + 2),
                        *data.get_unchecked(size + 3),
                    ]) as usize;
                    size += 4;
                }
                if size + key_len + value_len > len {
                    break;
                }
            }
            let key = unsafe { data.get_unchecked(size..size + key_len) };
            size += key_len;
            let value = match String::from_utf8(unsafe { data.get_unchecked(size..size + value_len) }.to_vec()) {
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
                b"REMOTE_ADDR" => ip = value,
                b"REQUEST_METHOD" => method = value,
                b"DOCUMENT_ROOT" => path = value,
                b"REDIRECT_URL" => {
                    if let Some(u) = value.split('?').next() {
                        u.clone_into(&mut url);
                    }
                }
                b"QUERY_STRING" => {
                    if !value.is_empty() {
                        let gets: Vec<&str> = value.split('&').collect();
                        get.reserve(gets.len());
                        for v in gets {
                            let key: Vec<&str> = v.splitn(2, '=').collect();
                            match key.len() {
                                1 => get.insert(v.to_owned(), String::new()),
                                _ => get.insert(key[0].to_owned(), key[1].to_owned()),
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
                    let key = match String::from_utf8(key.to_vec()) {
                        Ok(key) => key,
                        Err(_) => break,
                    };
                    params.insert(key, value);
                }
            }
        }
        params.shrink_to_fit();
        (
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
        )
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
                if stream.read(1000).await.is_err() {
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
