use std::{cmp::min, collections::HashMap, sync::Arc};

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};

use crate::{
    sys::{
        action::{ActionData, Input, Request, WebFile},
        log::Log,
        worker::{Worker, WorkerData, BUFFER_SIZE},
    },
    TINY_KEY,
};

/// SCGI protocol
pub struct Net;
/// Alias for SCGI protocol
type Scgi = Net;

impl Net {
    /// The entry point in the SCGI protocol
    pub async fn run(
        mut stream: TcpStream,
        data: WorkerData,
        mut buf: [u8; BUFFER_SIZE],
        mut len: usize,
    ) {
        // Check package size
        if len < 7 {
            Log::warning(2200, Some(len.to_string()));
            return;
        }
        // Read package separator
        let shift = match Scgi::read_separator(&buf[0..7]) {
            Some(shift) => {
                if shift == 0 {
                    Log::warning(2202, None);
                    return;
                }
                shift
            }
            None => {
                Log::warning(2201, None);
                return;
            }
        };
        // Get package
        let header_len = match shift {
            1 => buf[0] as usize - 0x30,
            2 => 10 * (buf[0] as usize - 0x30) + (buf[1] as usize - 0x30),
            3 => {
                100 * (buf[0] as usize - 0x30)
                    + 10 * (buf[1] as usize - 0x30)
                    + (buf[2] as usize - 0x30)
            }
            4 => {
                1000 * (buf[0] as usize - 0x30)
                    + 100 * (buf[1] as usize - 0x30)
                    + 10 * (buf[2] as usize - 0x30)
                    + (buf[3] as usize - 0x30)
            }
            5 => {
                10000 * (buf[0] as usize - 0x30)
                    + 1000 * (buf[1] as usize - 0x30)
                    + 100 * (buf[2] as usize - 0x30)
                    + 10 * (buf[3] as usize - 0x30)
                    + (buf[4] as usize - 0x30)
            }
            6 => {
                100000 * (buf[0] as usize - 0x30)
                    + 10000 * (buf[1] as usize - 0x30)
                    + 1000 * (buf[2] as usize - 0x30)
                    + 100 * (buf[3] as usize - 0x30)
                    + 10 * (buf[4] as usize - 0x30)
                    + (buf[5] as usize - 0x30)
            }
            7 => {
                1000000 * (buf[0] as usize - 0x30)
                    + 100000 * (buf[1] as usize - 0x30)
                    + 10000 * (buf[2] as usize - 0x30)
                    + 1000 * (buf[3] as usize - 0x30)
                    + 100 * (buf[4] as usize - 0x30)
                    + 10 * (buf[5] as usize - 0x30)
                    + (buf[6] as usize - 0x30)
            }
            _ => {
                Log::warning(2203, Some(shift.to_string()));
                return;
            }
        };
        let (mut request, content_type, session, content_len) =
            match Scgi::read_header(&mut stream, &mut buf, &mut len, shift + 1, header_len).await {
                Some(c) => c,
                None => return,
            };
        let (post, file) =
            match Scgi::read_input(&mut stream, &mut buf, len, content_type, content_len).await {
                Some(c) => c,
                None => return,
            };
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
        Scgi::write(&mut stream, answer).await;
    }

    /// Read post and file datas from SCGI record.
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
    async fn read_input(
        stream: &mut TcpStream,
        buf: &mut [u8; BUFFER_SIZE],
        mut len: usize,
        content_type: Option<String>,
        mut content_len: usize,
    ) -> Option<(HashMap<String, String>, HashMap<String, Vec<WebFile>>)> {
        let mut data = Vec::with_capacity(content_len);
        while content_len > 0 {
            if len == 0 {
                len = match stream.read(&mut buf[0..]).await {
                    Ok(len) => len,
                    Err(e) => {
                        Log::warning(2000, Some(e.to_string()));
                        return None;
                    }
                }
            }
            let max_read = min(content_len, len);
            data.extend_from_slice(&buf[0..max_read]);
            content_len -= max_read;
            len -= max_read;
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
        stream: &mut TcpStream,
        buf: &mut [u8; BUFFER_SIZE],
        len: &mut usize,
        mut shift: usize,
        mut header_len: usize,
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
        while header_len > 0 {
            if *len == 0 {
                *len = match stream.read(&mut buf[0..]).await {
                    Ok(len) => len,
                    Err(e) => {
                        Log::warning(2000, Some(e.to_string()));
                        return None;
                    }
                }
            }
            let max_read = min(header_len, *len - shift);

            if !is_param {
                // Read param
                match Scgi::read_next(&buf[shift..shift + max_read]) {
                    Some(found) => {
                        param.extend_from_slice(&buf[shift..shift + found]);
                        shift += found + 1;
                        header_len -= found + 1;
                        is_param = true;
                    }
                    None => {
                        param.extend_from_slice(&buf[shift..shift + max_read]);
                        header_len -= max_read;
                        *len = 0;
                        shift = 0;
                        continue;
                    }
                }
            } else if !is_value {
                // Read values
                match Scgi::read_next(&buf[shift..shift + max_read]) {
                    Some(found) => {
                        value.extend_from_slice(&buf[shift..shift + found]);
                        shift += found + 1;
                        header_len -= found + 1;
                        is_value = true;
                    }
                    None => {
                        value.extend_from_slice(&buf[shift..shift + max_read]);
                        header_len -= max_read;
                        *len = 0;
                        shift = 0;
                        continue;
                    }
                }
            } else {
                let key = match String::from_utf8(param.clone()) {
                    Ok(key) => key,
                    Err(e) => {
                        Log::warning(2204, Some(e.to_string()));
                        return None;
                    }
                };
                let val = match String::from_utf8(value.clone()) {
                    Ok(value) => value,
                    Err(e) => {
                        Log::warning(2204, Some(e.to_string()));
                        return None;
                    }
                };
                match key.as_str() {
                    "CONTENT_LENGTH" => {
                        if let Ok(c) = val.parse::<usize>() {
                            content_len = c;
                        }
                    }
                    "HTTP_X_REQUESTED_WITH" => ajax = val.to_lowercase().eq("xmlhttprequest"),
                    "HTTP_HOST" => host = val,
                    "REQUEST_SCHEME" => scheme = val,
                    "HTTP_USER_AGENT" => agent = val,
                    "HTTP_REFERER" => referer = val,
                    "REMOTE_ADDR" => ip = val,
                    "REQUEST_METHOD" => method = val,
                    "DOCUMENT_ROOT" => path = val,
                    "REDIRECT_URL" => {
                        if let Some(u) = val.split('?').next() {
                            url = u.to_owned();
                        }
                    }
                    "QUERY_STRING" => {
                        if !val.is_empty() {
                            let gets: Vec<&str> = val.split('&').collect();
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
                    "CONTENT_TYPE" => content_type = Some(val),
                    "HTTP_COOKIE" => {
                        let cooks: Vec<&str> = val.split("; ").collect();
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
        buf.copy_within(shift.., 0);
        *len -= shift;
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

    fn read_next(buf: &[u8]) -> Option<usize> {
        for (i, byte) in buf.iter().enumerate() {
            if *byte == 0 {
                return Some(i);
            }
        }
        None
    }

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

    /// Writes answer to server
    async fn write(tcp: &mut TcpStream, answer: Vec<u8>) {
        match tcp.write(&answer).await {
            Ok(i) => {
                if i != answer.len() {
                    Log::warning(2205, Some(i.to_string()));
                }
            }
            Err(e) => {
                Log::warning(2206, Some(e.to_string()));
            }
        }
    }
}
