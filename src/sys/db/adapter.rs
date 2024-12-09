use std::sync::Arc;

use tokio::sync::{Mutex, Semaphore};

#[cfg(all(
    feature = "pgsql",
    any(
        feature = "session-db",
        feature = "redirect-db",
        feature = "route-db",
        feature = "access-db",
        feature = "setting-db",
    )
))]
use postgres::Row;

#[cfg(all(
    feature = "mssql",
    any(
        feature = "session-db",
        feature = "redirect-db",
        feature = "route-db",
        feature = "access-db",
        feature = "setting-db",
    )
))]
use tiberius::Row;

use crate::{
    log,
    sys::app::init::{AutoCount, DBConfig},
};

#[cfg(feature = "pgsql")]
use super::pgsql::{DataRow, PgSql, QueryParam, QueryStream};

#[cfg(feature = "mssql")]
use super::mssql::{DataRow, MsSql, QueryParam};

#[cfg(feature = "row-data")]
use super::pgsql::PgColumn;

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
    #[cfg(feature = "pgsql")]
    connections: Vec<Arc<Mutex<PgSql>>>,
    /// Vector of database connections.
    #[cfg(feature = "mssql")]
    connections: Vec<Arc<Mutex<MsSql>>>,
    /// Semaphore for finding free connection.
    semaphore: Arc<Semaphore>,
}

impl DB {
    /// Initialize pool of database connections for asynchronous work.
    pub(crate) async fn new(config: Arc<DBConfig>) -> Result<DB, ()> {
        let size = match config.max {
            AutoCount::Auto => 3 * num_cpus::get(),
            AutoCount::Count(max) => max,
        };
        let mut connections = Vec::with_capacity(size);
        let mut list = Vec::with_capacity(size);

        for _ in 0..size {
            let config = Arc::clone(&config);
            let handle = tokio::spawn(async move {
                #[cfg(feature = "pgsql")]
                let mut db = PgSql::new(config)?;
                #[cfg(feature = "mssql")]
                let mut db = MsSql::new(Arc::clone(&config))?;
                if db.connect().await {
                    Some(db)
                } else {
                    None
                }
            });
            list.push(handle);
        }
        for handle in list {
            let db = match handle.await {
                Ok(db) => match db {
                    Some(db) => Arc::new(Mutex::new(db)),
                    None => return Err(()),
                },
                Err(_e) => {
                    log!(stop, 0, "{}", _e);
                    return Err(());
                }
            };
            connections.push(db);
        }
        let semaphore = Arc::new(Semaphore::new(size));

        Ok(DB { connections, semaphore })
    }

    /// Execute query to database
    #[cfg(feature = "row-native")]
    pub async fn query(&self, query: &str, params: QueryParam<'_>) -> Option<Vec<DataRow>> {
        let permit = match self.semaphore.acquire().await {
            Ok(p) => p,
            Err(_e) => {
                log!(warning, 0, "{}", _e);
                return None;
            }
        };
        for connection_mutex in &self.connections {
            if let Ok(mut db) = connection_mutex.try_lock() {
                let res = db.query(query, params).await;
                drop(db);
                drop(permit);
                return res;
            };
        }
        drop(permit);
        log!(warning, 0);
        None
    }

    #[cfg(feature = "row-data")]
    pub async fn query(&self, query: &str, params: QueryParam<'_>, assoc: bool) -> Option<Vec<DataRow>> {
        let permit = match self.semaphore.acquire().await {
            Ok(p) => p,
            Err(_e) => {
                log!(warning, 0, "{}", _e);
                return None;
            }
        };
        for connection_mutex in &self.connections {
            if let Ok(mut db) = connection_mutex.try_lock() {
                let res = db.query(query, params, assoc).await;
                drop(db);
                drop(permit);
                return res;
            };
        }
        drop(permit);
        log!(warning, 0);
        None
    }

    #[cfg(all(feature = "row-native", not(feature = "mssql")))]
    pub async fn query_stream<'a>(&'a self, query: &str, params: QueryParam<'_>) -> Option<QueryStream<'a>> {
        let permit = match self.semaphore.acquire().await {
            Ok(p) => p,
            Err(_e) => {
                log!(warning, 0, "{}", _e);
                return None;
            }
        };
        for connection_mutex in &self.connections {
            if let Ok(mut db) = connection_mutex.try_lock() {
                if let Some(stream) = db.query_stream(query, params).await {
                    return Some(QueryStream {
                        permit,
                        db,
                        stream: Box::pin(stream),
                        #[cfg(any(feature = "debug-v", feature = "debug-vv", feature = "debug-vvv"))]
                        sql: query,
                    });
                }
            };
        }
        drop(permit);
        log!(warning, 0);
        None
    }

    #[cfg(feature = "row-data")]
    pub async fn query_stream<'a>(&'a self, query: &'a str, params: QueryParam<'_>, assoc: bool) -> Option<QueryStream<'a>> {
        let permit = match self.semaphore.acquire().await {
            Ok(p) => p,
            Err(_e) => {
                log!(warning, 0, "{}", _e);
                return None;
            }
        };
        for connection_mutex in &self.connections {
            if let Ok(mut db) = connection_mutex.try_lock() {
                if let Some(stream) = db.query_stream(query, params).await {
                    return Some(QueryStream {
                        permit,
                        db,
                        stream: Box::pin(stream),
                        #[cfg(any(feature = "debug-v", feature = "debug-vv", feature = "debug-vvv"))]
                        sql: query,
                        cols: if assoc { PgColumn::Map(None) } else { PgColumn::Vec(None) },
                    });
                }
            };
        }
        drop(permit);
        log!(warning, 0);
        None
    }

    #[cfg(any(
        feature = "session-db",
        feature = "redirect-db",
        feature = "route-db",
        feature = "access-db",
        feature = "setting-db",
    ))]
    pub(crate) async fn query_prepare<'a>(&self, query: i64, params: QueryParam<'a>) -> Option<Vec<Row>> {
        let permit = match self.semaphore.acquire().await {
            Ok(p) => p,
            Err(_e) => {
                log!(warning, 0, "{}", _e);
                return None;
            }
        };
        for connection_mutex in &self.connections {
            if let Ok(mut db) = connection_mutex.try_lock() {
                let res = db.query_prepare(query, params).await;
                drop(db);
                drop(permit);
                return res;
            };
        }
        drop(permit);
        log!(warning, 0);
        None
    }

    /// Execute query to database synchronously without results
    pub async fn execute<'a>(&self, query: &str, params: QueryParam<'a>) -> Option<()> {
        let permit = match self.semaphore.acquire().await {
            Ok(p) => p,
            Err(_e) => {
                log!(warning, 0, "{}", _e);
                return None;
            }
        };
        for connection_mutex in &self.connections {
            if let Ok(mut db) = connection_mutex.try_lock() {
                let res = db.execute(query, params).await;
                drop(db);
                drop(permit);
                return res;
            };
        }
        drop(permit);
        log!(warning, 0);
        None
    }

    #[cfg(any(feature = "session-db", feature = "mail-db"))]
    pub(crate) async fn execute_prepare<'a>(&self, query: i64, params: QueryParam<'a>) -> Option<()> {
        let permit = match self.semaphore.acquire().await {
            Ok(p) => p,
            Err(_e) => {
                log!(warning, 0, "{}", _e);
                return None;
            }
        };
        for connection_mutex in &self.connections {
            if let Ok(mut db) = connection_mutex.try_lock() {
                let res = db.execute_prepare(query, params).await;
                drop(db);
                drop(permit);
                return res;
            };
        }
        drop(permit);
        log!(warning, 0);
        None
    }
}
