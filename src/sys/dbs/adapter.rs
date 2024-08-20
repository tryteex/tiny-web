use std::{
    future::Future,
    io,
    mem::transmute,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};

use postgres::tls::{ChannelBinding, MakeTlsConnect, TlsConnect};
use ring::digest;
use rustls::{pki_types::ServerName, ClientConfig};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio_rustls::{client::TlsStream, TlsConnector};
use x509_certificate::{
    DigestAlgorithm::{Sha1, Sha256, Sha384, Sha512},
    SignatureAlgorithm::{EcdsaSha256, EcdsaSha384, Ed25519, NoSignature, RsaSha1, RsaSha256, RsaSha384, RsaSha512},
    X509Certificate,
};

use crate::sys::action::Data;
use crate::sys::init::DBConfig;
use crate::sys::log::Log;

use super::{mssql::MsSql, pgsql::PgSql};
use postgres::types::ToSql as pgToSql;
use tiberius::ToSql as msToSql;
use tokio::sync::{Mutex, Semaphore};

/// Supported databases
#[derive(Debug, Clone)]
pub enum DBEngine {
    None,
    Pgsql,
    Mssql,
}

/// Adapter to databases
#[derive(Debug)]
pub(crate) enum DBConnect {
    Pgsql(PgSql),
    Mssql(MsSql),
}

/// Pool of database connections for asynchronous work.
///
/// # Values
///
/// * `connections: Vec<Arc<Mutex<DB>>>` - Vector of database connections;
/// * `semaphore: Arc<Semaphore>` - Semaphore for finding free connection;
/// * `size: usize` - Number of connected databases.
#[derive(Debug)]
pub struct DB {
    /// Vector of database connections.
    connections: Vec<Arc<Mutex<DBConnect>>>,
    /// Semaphore for finding free connection.
    semaphore: Arc<Semaphore>,
    /// Supported databases
    engine: DBEngine,
}

impl DB {
    /// Initialize pool of database connections for asynchronous work.
    ///
    /// # Parameters
    ///
    /// * `size: usize` - Pool size;
    /// * `config: Arc<DBConfig>` - Configuration.
    ///
    /// # Return
    ///
    /// New poll of database connections for asynchronous work.
    pub(crate) async fn new(size: usize, config: Arc<DBConfig>) -> Option<DB> {
        let mut connections = Vec::with_capacity(size);
        let mut asize = 0;
        for _ in 0..size {
            match &config.engine {
                DBEngine::Pgsql => {
                    let mut db = PgSql::new(Arc::clone(&config))?;
                    if db.connect().await {
                        asize += 1;
                        connections.push(Arc::new(Mutex::new(DBConnect::Pgsql(db))));
                    } else {
                        Log::stop(610, None);
                        return None;
                    }
                }
                DBEngine::Mssql => {
                    let mut db = MsSql::new(Arc::clone(&config))?;
                    if db.connect().await {
                        asize += 1;
                        connections.push(Arc::new(Mutex::new(DBConnect::Mssql(db))));
                    } else {
                        Log::stop(610, None);
                        return None;
                    }
                }
                _ => return None,
            };
        }
        let semaphore = Arc::new(Semaphore::new(asize));

        Some(DB {
            connections,
            semaphore,
            engine: config.engine.clone(),
        })
    }

    /// Is library uses database
    pub fn in_use(&self) -> bool {
        !matches!(self.engine, DBEngine::None)
    }

    /// Execute query to database synchronously
    ///
    /// # Parmeters
    ///
    /// * `query: &str` - SQL query;
    /// * `params: &[&(dyn ToSql + Sync)]` - Array of params.
    /// * `assoc: bool` - Return columns as associate array if True or Vecor id False.
    ///
    /// # Return
    ///
    /// * `Option::None` - When error query or diconnected;
    /// * `Option::Some(Vec<Data::Map>)` - Results, if assoc = true.
    /// * `Option::Some(Vec<Data::Vec>)` - Results, if assoc = false.
    pub async fn query(&self, query: &str, params: &[&dyn ToSql], assoc: bool) -> Option<Vec<Data>> {
        if let DBEngine::None = self.engine {
            return None;
        }

        let permit = match self.semaphore.acquire().await {
            Ok(p) => p,
            Err(e) => {
                Log::warning(606, Some(e.to_string()));
                return None;
            }
        };
        for connection_mutex in &self.connections {
            if let Ok(mut db) = connection_mutex.try_lock() {
                let res = match *db {
                    DBConnect::Pgsql(ref mut pg) => {
                        pg.query(&query, unsafe { transmute::<&[&dyn ToSql], &[&(dyn pgToSql + Sync)]>(params) }, assoc).await
                    }
                    DBConnect::Mssql(ref mut ms) => {
                        ms.query(&query, unsafe { transmute::<&[&dyn ToSql], &[&dyn msToSql]>(params) }, assoc).await
                    }
                };
                drop(permit);
                return res;
            };
        }
        drop(permit);
        Log::warning(607, None);
        None
    }

    pub(crate) async fn query_prepare(&self, query: i64, params: &[&dyn ToSql], assoc: bool) -> Option<Vec<Data>> {
        if let DBEngine::None = self.engine {
            return None;
        }

        let permit = match self.semaphore.acquire().await {
            Ok(p) => p,
            Err(e) => {
                Log::warning(606, Some(e.to_string()));
                return None;
            }
        };
        for connection_mutex in &self.connections {
            if let Ok(mut db) = connection_mutex.try_lock() {
                let res = match *db {
                    DBConnect::Pgsql(ref mut pg) => {
                        pg.query(&query, unsafe { transmute::<&[&dyn ToSql], &[&(dyn pgToSql + Sync)]>(params) }, assoc).await
                    }
                    DBConnect::Mssql(ref mut ms) => {
                        ms.query(&query, unsafe { transmute::<&[&dyn ToSql], &[&dyn msToSql]>(params) }, assoc).await
                    }
                };
                drop(permit);
                return res;
            };
        }
        drop(permit);
        Log::warning(607, None);
        None
    }

    /// Execute query to database synchronously without results
    ///
    /// # Parmeters
    ///
    /// * `query: &str` - SQL query;
    /// * `params: &[&(dyn ToSql + Sync)]` - Array of params.
    ///
    /// # Return
    ///
    /// * `Option::None` - When error query or diconnected;
    /// * `Option::Some(())` - Successed.
    pub async fn execute(&self, query: &str, params: &[&dyn ToSql]) -> Option<()> {
        if let DBEngine::None = self.engine {
            return None;
        }

        let permit = match self.semaphore.acquire().await {
            Ok(p) => p,
            Err(e) => {
                Log::warning(606, Some(e.to_string()));
                return None;
            }
        };
        for connection_mutex in &self.connections {
            if let Ok(mut db) = connection_mutex.try_lock() {
                let res = match *db {
                    DBConnect::Pgsql(ref mut pg) => {
                        pg.execute(&query, unsafe { transmute::<&[&dyn ToSql], &[&(dyn pgToSql + Sync)]>(params) }).await
                    }
                    DBConnect::Mssql(ref mut ms) => {
                        ms.execute(&query, unsafe { transmute::<&[&dyn ToSql], &[&dyn msToSql]>(params) }).await
                    }
                };
                drop(permit);
                return res;
            };
        }
        drop(permit);
        Log::warning(607, None);
        None
    }

    pub(crate) async fn execute_prepare(&self, query: i64, params: &[&dyn ToSql]) -> Option<()> {
        if let DBEngine::None = self.engine {
            return None;
        }

        let permit = match self.semaphore.acquire().await {
            Ok(p) => p,
            Err(e) => {
                Log::warning(606, Some(e.to_string()));
                return None;
            }
        };
        for connection_mutex in &self.connections {
            if let Ok(mut db) = connection_mutex.try_lock() {
                let res = match *db {
                    DBConnect::Pgsql(ref mut pg) => {
                        pg.execute(&query, unsafe { transmute::<&[&dyn ToSql], &[&(dyn pgToSql + Sync)]>(params) }).await
                    }
                    DBConnect::Mssql(ref mut ms) => {
                        ms.execute(&query, unsafe { transmute::<&[&dyn ToSql], &[&dyn msToSql]>(params) }).await
                    }
                };
                drop(permit);
                return res;
            };
        }
        drop(permit);
        Log::warning(607, None);
        None
    }

    /// Execute query to database and return a result,  
    /// and grouping tabular data according to specified conditions.
    ///
    /// # Parmeters
    ///
    /// * `query: &str` - SQL query;
    /// * `params: &[&dyn ToSql]` - Array of params.
    /// * `assoc: bool` - Return columns as associate array if True or Vecor id False.
    /// * `conds: Vec<Vec<&str>>` - Grouping condition.  
    ///
    /// Grouping condition:
    /// * The number of elements in the first-level array corresponds to the hierarchy levels in the group.
    /// * The number of elements in the second-level array corresponds to the number of items in one hierarchy. The first element of the group (index=0) is considered unique.
    /// * &str - field names for `Data::Vec<Data::Map<...>>`.
    ///
    /// The first value in the second-level array must be of type `Data::I64`.
    ///
    /// For each group, a new field with the name `sub` (encoded using `fnv1a_64`) will be created, where child groups will be located.
    ///
    /// If the data does not match the format `Data::Vec<Data::Map<...>>`, grouping will not occur, `Option::None` will be returned.  
    /// If the data does not match the tabular format, grouping will not occur, `Option::None` will be returned.
    ///
    /// Fields that are not included in the group will be excluded.
    ///
    /// # Return
    /// * Option::None - If the fields failed to group.  
    /// ## if assoc = true  
    /// * `Some(Data::Map<cond[0][0], Data::Map<...>>)` in hierarchical structure.  
    ///
    /// `struct
    /// value=Data::Map
    /// ├── [value1 from column_name=cond[0][0]] => [value=Data::Map]  : The unique value of the grouping field
    /// │   ├── [key=cond[0][0]] => [value1 from column_name=cond[0][0]] : The unique value of the grouping field
    /// │   ├── [key=cond[0][1]] => [value from column_name=cond[0][1]]
    /// │   │   ...  
    /// │   ├── [key=cond[0][last]] => [value from column_name=cond[0][last]]
    /// │   └── [key="sub"] => [value=Data::Map] : (encoded using fnv1a_64)
    /// │       ├── [value1 from column_name=cond[1][0]] => [value=Data::Map]  : The unique value of the grouping field
    /// │       │   ├── [cond[1][0]] => [value1 from column_name=cond[1][0]] : The unique value of the grouping field
    /// │       │   ├── [cond[1][1]] => [value from column_name=cond[1][1]]  
    /// │       │   │   ...
    /// │       │   ├── [cond[0][last]] => [value from column_name=cond[1][last]]  
    /// │       │   └── [key="sub"] => [value Data::Map] : (encoded using fnv1a_64)
    /// │       └── [value2 from column_name=cond[1][0]] => [value=Data::Map]  : The unique value of the grouping field
    /// │           │    ...
    /// ├── [value2 from column_name=cond[0][0]] => [value=Data::Map]  : The unique value of the grouping field
    /// │   ├── [key=cond[0][0]] => [value2 from column_name=cond[0][0]] : The unique value of the grouping field
    /// │   ├── [key=cond[0][1]] => [value from column_name=cond[0][1]]
    /// │   │   ...  
    /// │   ├── [key=cond[0][last]] => [value from column_name=cond[0][last]]
    /// │   ├── [key="sub"] => [value Data::Map] : (encoded using fnv1a_64)
    /// ...
    /// `
    /// ## if assoc = false  
    /// * `Some(Data::Map<cond[0][0], Data::Map<...>>)` in hierarchical structure.  
    ///
    /// `struct
    /// value=Data::Map
    /// ├── [value1 from column_name=cond[0][0]] => [value=Data::Vec]  : The unique value of the grouping field
    /// │   ├── [0] => [value1 from column_name=cond[0][0]] : The unique value of the grouping field
    /// │   ├── [1] => [value from column_name=cond[0][1]]
    /// │   │   ...  
    /// │   ├── [last] => [value from column_name=cond[0][last]]
    /// │   └── [last + 1] => [value=Data::Map] : (encoded using fnv1a_64)
    /// │       ├── [value1 from column_name=cond[1][0]] => [value=Data::Vec]  : The unique value of the grouping field
    /// │       │   ├── [0] => [value1 from column_name=cond[1][0]] : The unique value of the grouping field
    /// │       │   ├── [1] => [value from column_name=cond[1][1]]  
    /// │       │   │   ...
    /// │       │   ├── [last] => [value from column_name=cond[1][last]]  
    /// │       │   └── [last+1] => [value Data::Map] : (encoded using fnv1a_64)
    /// │       └── [value2 from column_name=cond[1][0]] => [value=Data::Vec]  : The unique value of the grouping field
    /// │           │    ...
    /// ├── [value2 from column_name=cond[0][0]] => [value=Data::Vec]  : The unique value of the grouping field
    /// │   ├── [0] => [value2 from column_name=cond[0][0]] : The unique value of the grouping field
    /// │   ├── [1] => [value from column_name=cond[0][1]]
    /// │   │   ...  
    /// │   ├── [last] => [value from column_name=cond[0][last]]
    /// │   ├── [last + 1] => [value Data::Map] : (encoded using fnv1a_64)
    /// ...
    /// `
    pub async fn query_group(
        &self,
        query: &str,
        params: &[&dyn ToSql],
        assoc: bool,
        conds: &[&[impl StrOrI64OrUSize]],
    ) -> Option<Data> {
        if let DBEngine::None = self.engine {
            return None;
        }

        let permit = match self.semaphore.acquire().await {
            Ok(p) => p,
            Err(e) => {
                Log::warning(606, Some(e.to_string()));
                return None;
            }
        };
        for connection_mutex in &self.connections {
            if let Ok(mut db) = connection_mutex.try_lock() {
                let res = match *db {
                    DBConnect::Pgsql(ref mut pg) => {
                        pg.query_group(
                            &query,
                            unsafe { transmute::<&[&dyn ToSql], &[&(dyn pgToSql + Sync)]>(params) },
                            assoc,
                            conds,
                        )
                        .await
                    }
                    DBConnect::Mssql(ref mut ms) => {
                        ms.query_group(&query, unsafe { transmute::<&[&dyn ToSql], &[&dyn msToSql]>(params) }, assoc, conds).await
                    }
                };
                drop(permit);
                return res;
            }
        }
        drop(permit);
        Log::warning(607, None);
        None
    }
}

/// Trait representing types that can be converted to a query or a key statement.
pub(crate) trait KeyOrQuery {
    /// Return key
    fn to_i64(&self) -> i64;
    /// Return text of query
    fn to_str(&self) -> &str;
    /// If value is key
    fn is_key(&self) -> bool;
}

impl KeyOrQuery for i64 {
    /// Return key
    fn to_i64(&self) -> i64 {
        *self
    }

    /// Return text of query
    fn to_str(&self) -> &str {
        "key_statement"
    }

    fn is_key(&self) -> bool {
        true
    }
}

impl KeyOrQuery for &str {
    /// Return key
    fn to_i64(&self) -> i64 {
        0
    }

    /// Return text of query
    fn to_str(&self) -> &str {
        self
    }

    fn is_key(&self) -> bool {
        false
    }
}

/// A trait representing types that can be converted to either `i64` or `usize`.
pub trait StrOrI64OrUSize {
    /// Converts the implementor to an `i64`.
    fn to_i64(&self) -> i64;

    /// Converts the implementor to a `usize`.
    fn to_usize(&self) -> usize;
}

impl StrOrI64OrUSize for i64 {
    /// Converts `i64` to itself.
    fn to_i64(&self) -> i64 {
        *self
    }

    /// Converts `i64` to `usize`, always returning `0`.
    fn to_usize(&self) -> usize {
        usize::MAX
    }
}

impl StrOrI64OrUSize for &str {
    /// Converts `&str` to an `i64` using the FNV1a hash algorithm.
    fn to_i64(&self) -> i64 {
        crate::fnv1a_64(self.as_bytes())
    }

    /// Converts `&str` to `usize`, always returning `0`.
    fn to_usize(&self) -> usize {
        usize::MAX
    }
}

impl StrOrI64OrUSize for usize {
    /// Converts `usize` to `i64`, always returning `0`.
    fn to_i64(&self) -> i64 {
        0
    }

    /// Converts `usize` to itself.
    fn to_usize(&self) -> usize {
        *self
    }
}

pub trait ToSql: pgToSql + msToSql + Sync {}
impl<T: pgToSql + msToSql + Sync> ToSql for T {}

#[derive(Clone)]
pub(crate) struct MakeTinyTlsConnect {
    config: Arc<ClientConfig>,
}

impl MakeTinyTlsConnect {
    pub fn new(config: ClientConfig) -> Self {
        Self { config: Arc::new(config) }
    }
}

impl<S> MakeTlsConnect<S> for MakeTinyTlsConnect
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    type Stream = TinyTlsStream<S>;
    type TlsConnect = TinyTlsConnect;
    type Error = rustls::pki_types::InvalidDnsNameError;

    fn make_tls_connect(&mut self, hostname: &str) -> Result<TinyTlsConnect, Self::Error> {
        ServerName::try_from(hostname).map(|dns_name| {
            TinyTlsConnect(TinyTlsConnectData {
                hostname: dns_name.to_owned(),
                connector: Arc::clone(&self.config).into(),
            })
        })
    }
}

pub(crate) struct TinyTlsConnect(TinyTlsConnectData);

struct TinyTlsConnectData {
    hostname: ServerName<'static>,
    connector: TlsConnector,
}

impl<S> TlsConnect<S> for TinyTlsConnect
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    type Stream = TinyTlsStream<S>;
    type Error = io::Error;
    type Future = Pin<Box<dyn Future<Output = io::Result<TinyTlsStream<S>>> + Send>>;

    fn connect(self, stream: S) -> Self::Future {
        Box::pin(async move { self.0.connector.connect(self.0.hostname, stream).await.map(|s| TinyTlsStream(Box::pin(s))) })
    }
}

pub(crate) struct TinyTlsStream<S>(Pin<Box<TlsStream<S>>>);

impl<S> tokio_postgres::tls::TlsStream for TinyTlsStream<S>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    fn channel_binding(&self) -> ChannelBinding {
        let (_, session) = self.0.get_ref();
        match session.peer_certificates() {
            Some(certs) if !certs.is_empty() => X509Certificate::from_der(&certs[0])
                .ok()
                .and_then(|cert| cert.signature_algorithm())
                .map(|algorithm| match algorithm {
                    RsaSha1 | RsaSha256 | EcdsaSha256 => &digest::SHA256,
                    RsaSha384 | EcdsaSha384 => &digest::SHA384,
                    RsaSha512 | Ed25519 => &digest::SHA512,
                    NoSignature(algo) => match algo {
                        Sha1 | Sha256 => &digest::SHA256,
                        Sha384 => &digest::SHA384,
                        Sha512 => &digest::SHA512,
                    },
                })
                .map(|algorithm| {
                    let hash = digest::digest(algorithm, certs[0].as_ref());
                    ChannelBinding::tls_server_end_point(hash.as_ref().into())
                })
                .unwrap_or(ChannelBinding::none()),
            _ => ChannelBinding::none(),
        }
    }
}

impl<S> AsyncRead for TinyTlsStream<S>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    fn poll_read(mut self: Pin<&mut Self>, cx: &mut Context, buf: &mut ReadBuf<'_>) -> Poll<tokio::io::Result<()>> {
        self.0.as_mut().poll_read(cx, buf)
    }
}

impl<S> AsyncWrite for TinyTlsStream<S>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    fn poll_write(mut self: Pin<&mut Self>, cx: &mut Context, buf: &[u8]) -> Poll<tokio::io::Result<usize>> {
        self.0.as_mut().poll_write(cx, buf)
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<tokio::io::Result<()>> {
        self.0.as_mut().poll_flush(cx)
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<tokio::io::Result<()>> {
        self.0.as_mut().poll_shutdown(cx)
    }
}
