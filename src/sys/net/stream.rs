use std::{
    cmp::min,
    fmt::{Display, Formatter},
    io::{Error, ErrorKind},
    net::{IpAddr, SocketAddr},
    sync::Arc,
    time::Duration,
};

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream, UnixListener, UnixStream},
    sync::mpsc::{self, Sender},
    task::JoinHandle,
    time,
};

#[cfg(feature = "https")]
use tokio_rustls::TlsAcceptor;

use crate::{log, sys::app::init::SIGNAL_TIMEOUT};

#[cfg(feature = "fastcgi")]
use super::fastcgi::FastCGI;

pub(super) const BUFFER_SIZE: usize = 8192;

#[derive(Debug)]
pub(crate) enum Socket {
    Inet(SocketAddr),
    #[cfg(not(target_family = "windows"))]
    Unix(String),
}

impl Socket {
    pub(crate) async fn bind(&self) -> Result<Listener, Error> {
        match self {
            Socket::Inet(addr) => Ok(Listener::TcpListener(TcpListener::bind(addr).await?)),
            Socket::Unix(uds) => Ok(Listener::UnixListener(UnixListener::bind(uds)?)),
        }
    }
}

#[derive(Debug)]
pub(crate) enum Listener {
    TcpListener(TcpListener),
    #[cfg(not(target_family = "windows"))]
    UnixListener(UnixListener),
}

impl Listener {
    pub(crate) async fn accept(&self, ip: &IpAddr) -> Result<(Stream, Option<IpAddr>), Error> {
        match self {
            Listener::TcpListener(tcp) => {
                let (stream, addr) = tcp.accept().await?;
                if !ip.is_unspecified() && addr.ip() != *ip {
                    return Err(Error::new(ErrorKind::Interrupted, "IP address is not spe"));
                }
                stream.set_nodelay(true)?;
                Ok((Stream::Tcp(stream), Some(addr.ip())))
            }
            #[cfg(not(target_family = "windows"))]
            Listener::UnixListener(unix) => {
                let (stream, _) = unix.accept().await?;
                Ok((Stream::Unix(stream), None))
            }
        }
    }
}

pub(crate) enum Stream {
    Tcp(TcpStream),
    #[cfg(not(target_family = "windows"))]
    Unix(UnixStream),
}

impl Stream {
    pub(crate) async fn signal_read_i64(&mut self) -> Result<i64, Error> {
        match self {
            Stream::Tcp(stream) => match time::timeout(Duration::from_millis(SIGNAL_TIMEOUT), stream.read_i64()).await {
                Ok(r) => r,
                Err(e) => Err(Error::new(ErrorKind::TimedOut, e)),
            },
            Stream::Unix(stream) => match time::timeout(Duration::from_millis(SIGNAL_TIMEOUT), stream.read_i64()).await {
                Ok(r) => r,
                Err(e) => Err(Error::new(ErrorKind::TimedOut, e)),
            },
        }
    }

    pub(crate) async fn signal_write_u64(&mut self, signal: u64) -> Result<(), Error> {
        match self {
            Stream::Tcp(stream) => match time::timeout(Duration::from_millis(SIGNAL_TIMEOUT), stream.write_u64(signal)).await {
                Ok(r) => r,
                Err(e) => Err(Error::new(ErrorKind::TimedOut, e)),
            },
            Stream::Unix(stream) => match time::timeout(Duration::from_millis(SIGNAL_TIMEOUT), stream.write_u64(signal)).await {
                Ok(r) => r,
                Err(e) => Err(Error::new(ErrorKind::TimedOut, e)),
            },
        }
    }

    pub(crate) async fn signal_write_str(&mut self, data: &str) -> Result<(), Error> {
        match self {
            Stream::Tcp(stream) => match time::timeout(Duration::from_millis(SIGNAL_TIMEOUT), stream.write_all(data.as_bytes())).await {
                Ok(r) => r,
                Err(e) => Err(Error::new(ErrorKind::TimedOut, e)),
            },
            Stream::Unix(stream) => match time::timeout(Duration::from_millis(SIGNAL_TIMEOUT), stream.write_all(data.as_bytes())).await {
                Ok(r) => r,
                Err(e) => Err(Error::new(ErrorKind::TimedOut, e)),
            },
        }
    }

    #[cfg(not(feature = "https"))]
    pub(crate) fn into_split(self) -> (ReadHalf, WriteHalf) {
        match self {
            Stream::Tcp(stream) => {
                let (read, write) = stream.into_split();
                (ReadHalf::Tcp(read), WriteHalf::Tcp(write))
            }
            Stream::Unix(stream) => {
                let (read, write) = stream.into_split();
                (ReadHalf::Unix(read), WriteHalf::Unix(write))
            }
        }
    }

    #[cfg(feature = "https")]
    pub(crate) async fn into_split(self, acceptor: Arc<TlsAcceptor>) -> Result<(ReadHalf, WriteHalf), Error> {
        match self {
            Stream::Tcp(stream) => {
                let tls_stream = acceptor.accept(stream).await?;
                let (read, write) = tokio::io::split(tls_stream);
                Ok((ReadHalf::Tcp(read), WriteHalf::Tcp(write)))
            }
            Stream::Unix(stream) => {
                let tls_stream = acceptor.accept(stream).await?;
                let (read, write) = tokio::io::split(tls_stream);
                Ok((ReadHalf::Unix(read), WriteHalf::Unix(write)))
            }
        }
    }
}

pub(crate) enum ReadHalf {
    #[cfg(not(feature = "https"))]
    Tcp(tokio::net::tcp::OwnedReadHalf),
    #[cfg(feature = "https")]
    Tcp(tokio::io::ReadHalf<tokio_rustls::server::TlsStream<tokio::net::TcpStream>>),
    #[cfg(all(not(target_family = "windows"), not(feature = "https")))]
    Unix(tokio::net::unix::OwnedReadHalf),
    #[cfg(all(not(target_family = "windows"), feature = "https"))]
    Unix(tokio::io::ReadHalf<tokio_rustls::server::TlsStream<tokio::net::UnixStream>>),
}

impl ReadHalf {
    pub(crate) async fn read(&mut self, buf: &mut [u8]) -> Result<usize, Error> {
        match self {
            ReadHalf::Tcp(stream) => stream.read(buf).await,
            ReadHalf::Unix(stream) => stream.read(buf).await,
        }
    }
}

pub(crate) enum WriteHalf {
    #[cfg(not(feature = "https"))]
    Tcp(tokio::net::tcp::OwnedWriteHalf),
    #[cfg(feature = "https")]
    Tcp(tokio::io::WriteHalf<tokio_rustls::server::TlsStream<tokio::net::TcpStream>>),
    #[cfg(all(not(target_family = "windows"), not(feature = "https")))]
    Unix(tokio::net::unix::OwnedWriteHalf),
    #[cfg(all(not(target_family = "windows"), feature = "https"))]
    Unix(tokio::io::WriteHalf<tokio_rustls::server::TlsStream<tokio::net::UnixStream>>),
}

impl WriteHalf {
    pub(crate) async fn write_all(&mut self, src: &[u8]) -> Result<(), Error> {
        match self {
            WriteHalf::Tcp(stream) => stream.write_all(src).await,
            WriteHalf::Unix(stream) => stream.write_all(src).await,
        }
    }
}

pub(crate) struct StreamRead {
    pub stream: ReadHalf,
    pub buf: [u8; BUFFER_SIZE],
    pub len: usize,
    pub shift: usize,
}

impl StreamRead {
    pub(super) async fn read(&mut self, timeout: u64) -> Result<(), StreamError> {
        if self.shift == 0 && self.len == BUFFER_SIZE {
            return Err(StreamError::Buffer);
        } else if self.shift > 0 && self.len <= BUFFER_SIZE {
            self.buf.copy_within(self.shift.., 0);
            self.len -= self.shift;
            self.shift = 0;
        }

        if timeout > 0 {
            match tokio::time::timeout(std::time::Duration::from_millis(timeout), async {
                match self.stream.read(unsafe { self.buf.get_unchecked_mut(self.len..) }).await {
                    Ok(len) => {
                        if len == 0 {
                            Err(StreamError::Closed)
                        } else {
                            Ok(len)
                        }
                    }
                    Err(e) => Err(StreamError::Error(e)),
                }
            })
            .await
            {
                Ok(res) => match res {
                    Ok(len) => {
                        self.len += len;
                        Ok(())
                    }
                    Err(e) => Err(e),
                },
                Err(_) => Err(StreamError::Timeout),
            }
        } else {
            match self.stream.read(unsafe { self.buf.get_unchecked_mut(self.len..) }).await {
                Ok(len) => {
                    if len == 0 {
                        Err(StreamError::Closed)
                    } else {
                        self.len += len;
                        Ok(())
                    }
                }
                Err(e) => Err(StreamError::Error(e)),
            }
        }
    }

    pub(super) fn get(&self, size: usize) -> &[u8] {
        let size = min(self.shift + size, self.len);
        unsafe { self.buf.get_unchecked(self.shift..size) }
    }

    pub(super) fn shift(&mut self, shift: usize) {
        self.shift += shift;
        if self.shift > self.len {
            self.shift = self.len;
        }
    }

    pub(super) fn available(&self) -> usize {
        self.len - self.shift
    }
}

#[derive(Debug)]
pub(crate) enum MessageWrite {
    #[cfg(not(feature = "fastcgi"))]
    Message(Vec<u8>),
    #[cfg(feature = "fastcgi")]
    Message(Vec<u8>, bool),
    End,
}

pub(crate) struct StreamWrite {
    pub tx: Arc<Sender<MessageWrite>>,
}

impl StreamWrite {
    pub(super) async fn new(mut write: WriteHalf) -> (Arc<StreamWrite>, JoinHandle<()>) {
        let (tx, mut rx) = mpsc::channel(32);
        let stream = Arc::new(StreamWrite { tx: Arc::new(tx) });

        let handle = tokio::spawn(async move {
            while let Some(message) = rx.recv().await {
                match message {
                    #[cfg(feature = "fastcgi")]
                    MessageWrite::Message(message, end) => {
                        let message = FastCGI::write(message, end);

                        if let Err(_e) = write.write_all(&message).await {
                            log!(warning, 0, "{}", _e);
                        }
                    }
                    #[cfg(not(feature = "fastcgi"))]
                    MessageWrite::Message(message) => {
                        if let Err(_e) = write.write_all(&message).await {
                            log!(warning, 0, "{}", _e);
                        }
                    }
                    MessageWrite::End => break,
                }
            }
        });
        (stream, handle)
    }

    pub(super) async fn end(handle: JoinHandle<()>, tx: Arc<Sender<MessageWrite>>) {
        if let Err(_e) = tx.send(MessageWrite::End).await {
            log!(warning, 0, "{}", _e);
        }
        if let Err(_e) = handle.await {
            log!(warning, 0, "{}", _e);
        }
    }

    pub(super) async fn write(&self, data: Vec<u8>) {
        #[cfg(not(feature = "fastcgi"))]
        if let Err(_e) = self.tx.send(MessageWrite::Message(data)).await {
            log!(warning, 0, "{}", _e);
        }
        #[cfg(feature = "fastcgi")]
        if let Err(_e) = self.tx.send(MessageWrite::Message(data, true)).await {
            log!(warning, 0, "{}", _e);
        }
    }
}

#[derive(Debug)]
pub(super) enum StreamError {
    Closed,
    Error(Error),
    Buffer,
    Timeout,
}

impl Display for StreamError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            StreamError::Closed => write!(f, "Stream was closed"),
            StreamError::Error(err) => write!(f, "Stream has error: {}", err),
            StreamError::Buffer => write!(f, "Error buffer of stream"),
            StreamError::Timeout => write!(f, "Stream was closed by timeout"),
        }
    }
}
