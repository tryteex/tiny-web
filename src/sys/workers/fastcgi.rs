use std::{collections::HashMap, sync::Arc};

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};

use crate::{
    sys::{
        action::{ActionData, Input, Request},
        log::Log,
        worker::{Worker, WorkerData, BUFFER_SIZE},
    },
    TINY_KEY,
};

/// Describes a header in a FastCGI record.
///
/// # Values
///
/// * `header_type: u8` - FastCGI header type.
/// * `content_length: u16` - Content length.
/// * `padding_length: u8` - Padding length.
#[derive(Debug)]
pub struct Header {
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
pub struct Record {
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
pub enum RecordType {
    /// Some FastCGI value.
    Some(Record),
    /// Error recognizing FastCGI record.
    Error(Vec<u8>),
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
pub struct Net;
/// Alias for FastCGI protocol
type FastCGI = Net;

impl Net {
    /// The entry point in the FastCGI protocol
    pub async fn run(
        mut tcp: TcpStream,
        data: WorkerData,
        mut buffer: [u8; BUFFER_SIZE],
        len: usize,
    ) {
        let mut size = len;
        loop {
            // Gets one Record
            let record = match FastCGI::read_record_raw(&mut tcp, &mut buffer, &mut size).await {
                RecordType::Some(r) => r,
                RecordType::Error(e) => {
                    let s: String = e.iter().map(|byte| format!("{:02x}", byte)).collect();
                    Log::warning(2100, Some(s));
                    break;
                }
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
                    let record = match FastCGI::read_record_raw(&mut tcp, &mut buffer, &mut size)
                        .await
                    {
                        RecordType::Some(r) => r,
                        RecordType::Error(e) => {
                            let s: String = e.iter().map(|byte| format!("{:02x}", byte)).collect();
                            Log::warning(2100, Some(s));
                            break;
                        }
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
                        _ => {
                            Log::warning(2102, Some(format!("{:?}", record)));
                            return;
                        }
                    }
                    if is_stdin_done && is_param_done {
                        break;
                    }
                }
                let (mut request, content_type, session) = FastCGI::read_param(params);
                let (post, file) = Worker::read_input(stdin, content_type).await;
                request.input.file = file;
                request.input.post = post;
                let data = ActionData {
                    data: WorkerData {
                        engine: Arc::clone(&data.engine),
                        lang: Arc::clone(&data.lang),
                        html: Arc::clone(&data.html),
                        cache: Arc::clone(&data.cache),
                        db: Arc::clone(&data.db),
                        salt: data.salt.clone(),
                    },
                    request,
                    session,
                };
                let answer = Worker::call_action(data).await;
                if FastCGI::write(&mut tcp, answer).await.is_err() {
                    return;
                };
            } else {
                Log::warning(2101, Some(format!("{:?}", record)));
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
    fn read_param(mut data: Vec<u8>) -> (Request, Option<String>, Option<String>) {
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

            let key = match String::from_utf8((data[size..size + key_len]).to_vec()) {
                Ok(key) => key,
                Err(e) => {
                    Log::warning(2103, Some(e.to_string()));
                    break;
                }
            };
            size += key_len;
            let value = match String::from_utf8(data[size..size + value_len].to_vec()) {
                Ok(value) => value,
                Err(e) => {
                    Log::warning(2103, Some(e.to_string()));
                    break;
                }
            };
            size += value_len;
            // We will take some of the headers right away, and leave some for the user
            match key.as_str() {
                "HTTP_X_REQUESTED_WITH" => ajax = value.to_lowercase().eq("xmlhttprequest"),
                "HTTP_HOST" => host = value,
                "REQUEST_SCHEME" => scheme = value,
                "HTTP_USER_AGENT" => agent = value,
                "HTTP_REFERER" => referer = value,
                "REMOTE_ADDR" => ip = value,
                "REQUEST_METHOD" => method = value,
                "DOCUMENT_ROOT" => path = value,
                "REDIRECT_URL" => {
                    if let Some(u) = value.split('?').next() {
                        url = u.to_owned();
                    }
                }
                "QUERY_STRING" => {
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
                "CONTENT_TYPE" => content_type = Some(value),
                "HTTP_COOKIE" => {
                    let cooks: Vec<&str> = value.split("; ").collect();
                    cookie.reserve(cooks.len());
                    for v in cooks {
                        let key: Vec<&str> = v.splitn(2, '=').collect();
                        if key.len() == 2 {
                            if key[0] == TINY_KEY {
                                let val = key[1];
                                if val.len() == 128 {
                                    for b in val.as_bytes() {
                                        if !((*b > 47 && *b < 58) || (*b > 96 && *b < 103)) {
                                            continue;
                                        }
                                    }
                                    session_key = Some(key[1].to_owned());
                                }
                            } else {
                                cookie.insert(key[0].to_owned(), key[1].to_owned());
                            }
                        }
                    }
                }
                _ => {
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
    async fn read_record_raw(
        tcp: &mut TcpStream,
        buffer: &mut [u8; BUFFER_SIZE],
        size: &mut usize,
    ) -> RecordType {
        // There is not enough buffer
        if *size < FASTCGI_HEADER_LEN {
            let len = match tcp.read(&mut buffer[*size..]).await {
                Ok(len) => len,
                Err(e) => {
                    Log::warning(2000, Some(e.to_string()));
                    return RecordType::StreamClose;
                }
            };
            if len == 0 {
                // Stream was closed
                return RecordType::StreamClose;
            }
            *size += len;
        }
        // Something went wrong, they could not read some 8 bytes
        if *size < FASTCGI_HEADER_LEN {
            return RecordType::Error(buffer[0..*size].to_vec());
        }

        let header = FastCGI::read_header(&buffer[0..FASTCGI_HEADER_LEN]);
        let total = header.content_length as usize;
        let mut read = 0;

        // It is necessary to determine how much data is in the record,
        // if it is more than FASTCGI_MAX_CONTENT_LEN, then we read in several approaches
        let mut max_read = std::cmp::min(total, *size - FASTCGI_HEADER_LEN);

        let mut vec = Vec::with_capacity(total);
        let mut seek = FASTCGI_HEADER_LEN;
        loop {
            vec.extend_from_slice(&buffer[seek..seek + max_read]);
            read += max_read;
            if read == total {
                break;
            }
            *size = match tcp.read(buffer).await {
                Ok(len) => len,
                Err(e) => {
                    Log::warning(2000, Some(e.to_string()));
                    return RecordType::StreamClose;
                }
            };
            if *size == 0 {
                // Stream was closed
                return RecordType::StreamClose;
            }
            seek = 0;
            max_read = std::cmp::min(total - read, *size);
        }

        buffer.copy_within(header.padding_length as usize + seek + max_read..*size, 0);
        *size -= max_read + seek + header.padding_length as usize;

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
                content_length: u16::from_be_bytes([
                    *data.get_unchecked(4),
                    *data.get_unchecked(5),
                ]),
                padding_length: *data.get_unchecked(6),
            }
        }
    }

    /// Writes answer to server
    async fn write(tcp: &mut TcpStream, answer: Vec<u8>) -> Result<(), ()> {
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
            data.extend_from_slice(&answer[seek..seek + size]);
            seek += size;
        }
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

        match tcp.write(&data).await {
            Ok(i) => {
                if i != data.len() {
                    Log::warning(2104, Some(i.to_string()));
                    return Err(());
                }
            }
            Err(e) => {
                Log::warning(2105, Some(e.to_string()));
                return Err(());
            }
        }
        Ok(())
    }
}
