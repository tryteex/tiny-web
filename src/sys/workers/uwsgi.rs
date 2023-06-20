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

/// UWSGI protocol
pub struct Net;
/// Alias for UWSGI protocol
type Uwsgi = Net;

impl Net {
    /// The entry point in the UWSGI protocol
    pub async fn run(
        mut stream: TcpStream,
        data: WorkerData,
        mut buf: [u8; BUFFER_SIZE],
        mut len: usize,
    ) {
        loop {
            // Check package size
            if len < 4 || buf[0] != 0 || buf[3] != 0 {
                Log::warning(2300, Some(len.to_string()));
                return;
            }
            let packet_len = u16::from_le_bytes([buf[1], buf[2]]) as usize;

            let (mut request, content_type, session, content_len) =
                match Uwsgi::read_header(&mut stream, &mut buf, &mut len, 4, packet_len).await {
                    Some(c) => c,
                    None => return,
                };
            let (post, file) = match Uwsgi::read_input(
                &mut stream,
                &mut buf,
                len,
                content_type,
                content_len,
            )
            .await
            {
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
            if Uwsgi::write(&mut stream, answer).await.is_err() {
                return;
            }
            // Wait next client
            len = match stream.read(&mut buf).await {
                Ok(len) => len,
                Err(e) => {
                    Log::warning(2000, Some(e.to_string()));
                    return;
                }
            };
            if len == 0 {
                return;
            }
        }
    }

    /// Writes answer to server
    async fn write(tcp: &mut TcpStream, answer: Vec<u8>) -> Result<(), ()> {
        match tcp.write(&answer).await {
            Ok(i) => {
                if i != answer.len() {
                    Log::warning(2304, Some(i.to_string()));
                    return Err(());
                }
            }
            Err(e) => {
                Log::warning(2305, Some(e.to_string()));
                return Err(());
            }
        }
        Ok(())
    }

    /// Read post and file datas from UWSGI record.
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

    /// Read params from UWSGI header
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
        mut packet_len: usize,
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
        while packet_len > 0 {
            let max_read = min(packet_len, *len - shift);
            if !is_param {
                if param_size == 0 {
                    if max_read < 2 {
                        if Uwsgi::read(stream, buf, len, &mut shift).await.is_err() {
                            return None;
                        }
                        continue;
                    }
                    param_size = u16::from_le_bytes([buf[shift], buf[shift + 1]]) as usize;
                    if param_size == 0 {
                        Log::warning(2302, None);
                        return None;
                    }
                    packet_len -= 2;
                    shift += 2;
                    continue;
                }
                if param_size <= max_read {
                    param.extend_from_slice(&buf[shift..shift + param_size]);
                    shift += param_size;
                    packet_len -= param_size;
                    is_param = true;
                    param_size = 0;
                } else {
                    param.extend_from_slice(&buf[shift..shift + max_read]);
                    shift += max_read;
                    packet_len -= max_read;
                    param_size -= max_read;
                    if Uwsgi::read(stream, buf, len, &mut shift).await.is_err() {
                        return None;
                    }
                }
            } else if !is_value {
                if value_size == 0 {
                    if max_read < 2 {
                        if Uwsgi::read(stream, buf, len, &mut shift).await.is_err() {
                            return None;
                        }
                        continue;
                    }
                    value_size = u16::from_le_bytes([buf[shift], buf[shift + 1]]) as usize;
                    packet_len -= 2;
                    shift += 2;
                    if value_size == 0 {
                        is_value = true;
                    }
                    continue;
                }
                if value_size <= max_read {
                    value.extend_from_slice(&buf[shift..shift + value_size]);
                    shift += value_size;
                    packet_len -= value_size;
                    is_value = true;
                    value_size = 0;
                } else {
                    value.extend_from_slice(&buf[shift..shift + max_read]);
                    shift += max_read;
                    packet_len -= max_read;
                    value_size -= max_read;
                    if Uwsgi::read(stream, buf, len, &mut shift).await.is_err() {
                        return None;
                    }
                }
            } else {
                let key = match String::from_utf8(param.clone()) {
                    Ok(key) => key,
                    Err(e) => {
                        Log::warning(2303, Some(e.to_string()));
                        return None;
                    }
                };
                let val = match String::from_utf8(value.clone()) {
                    Ok(value) => value,
                    Err(e) => {
                        Log::warning(2303, Some(e.to_string()));
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

    async fn read(
        stream: &mut TcpStream,
        buf: &mut [u8; BUFFER_SIZE],
        len: &mut usize,
        shift: &mut usize,
    ) -> Result<(), ()> {
        if *len - *shift == BUFFER_SIZE {
            Log::warning(2301, None);
            return Err(());
        }
        if *shift < *len {
            buf.copy_within(*shift..*len, 0);
        }
        *len -= *shift;
        *shift = 0;
        *len += match stream.read(&mut buf[*len..]).await {
            Ok(l) => l,
            Err(e) => {
                Log::warning(2000, Some(e.to_string()));
                return Err(());
            }
        };
        Ok(())
    }
}
