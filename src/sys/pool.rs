use std::sync::Arc;

use postgres::{types::ToSql, Row};
use tokio::sync::{Mutex, Semaphore};

use super::{db::DB, init::DBConfig, log::Log};

/// Pool of database connections for asynchronous work.
///
/// # Values
///
/// * `connections: Vec<Arc<Mutex<DB>>>` - Vector of database connections;
/// * `semaphore: Arc<Semaphore>` - Semaphore for finding free connection;
/// * `size: usize` - Number of connected databases.
#[derive(Debug)]
pub struct DBPool {
    /// Vector of database connections.
    connections: Vec<Arc<Mutex<DB>>>,
    /// Semaphore for finding free connection.
    semaphore: Arc<Semaphore>,
    /// Number of connected databases.
    pub size: usize,
}

impl DBPool {
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
    pub async fn new(size: usize, config: Arc<DBConfig>) -> DBPool {
        let mut connections = Vec::with_capacity(size);
        let mut asize = 0;
        for _ in 0..size {
            let mut db = DB::new(Arc::clone(&config)).await;
            if db.connect().await {
                asize += 1;
                connections.push(Arc::new(Mutex::new(db)));
            }
        }
        let semaphore = Arc::new(Semaphore::new(asize));

        DBPool {
            connections,
            semaphore,
            size: asize,
        }
    }

    /// Execute query to database with paramaters asynchronously
    ///
    /// # Parmeters
    ///
    /// * `query: &str` - SQL query;
    /// * `params: &[&(dyn ToSql + Sync)]` - Array of params.
    ///
    /// # Return
    ///
    /// * `Option::None` - When error query or diconnected;
    /// * `Option::Some(Vec<Row>)` - Get result.
    pub async fn query_params(
        &self,
        query: &str,
        params: &[&(dyn ToSql + Sync)],
    ) -> Option<Vec<Row>> {
        let permit = match self.semaphore.acquire().await {
            Ok(p) => p,
            Err(e) => {
                Log::warning(606, Some(e.to_string()));
                return None;
            }
        };
        for connection_mutex in &self.connections {
            if let Ok(mut db) = connection_mutex.try_lock() {
                let res = db.query_params(query, params).await;
                drop(db);
                drop(permit);
                return res;
            }
        }
        drop(permit);
        Log::warning(607, None);
        None
    }

    /// Execute query to database synchronously
    ///
    /// # Parmeters
    ///
    /// * `query: &str` - SQL query;
    /// * `params: &[&(dyn ToSql + Sync)]` - Array of params.
    ///
    /// # Return
    ///
    /// * `Option::None` - When error query or diconnected;
    /// * `Option::Some(Vec<Row>)` - Results.
    pub async fn query(&self, query: &str) -> Option<Vec<Row>> {
        let permit = match self.semaphore.acquire().await {
            Ok(p) => p,
            Err(e) => {
                Log::warning(606, Some(e.to_string()));
                return None;
            }
        };
        for connection_mutex in &self.connections {
            if let Ok(mut db) = connection_mutex.try_lock() {
                let res = db.query(query).await;
                drop(db);
                drop(permit);
                return res;
            }
        }
        drop(permit);
        Log::warning(607, None);
        None
    }

    /// Execute prepare query to database synchronously with paramaters asynchronously
    ///
    /// # Parmeters
    ///
    /// * `index: usize` - index of prepare query.
    /// * `params: &[&(dyn ToSql + Sync)]` - Array of params.
    ///
    /// # Return
    ///
    /// * `Option::None` - When error query or diconnected;
    /// * `Option::Some(Vec<Row>)` - Get result.
    pub async fn query_fast(
        &self,
        index: usize,
        params: &[&(dyn ToSql + Sync)],
    ) -> Option<Vec<Row>> {
        let permit = match self.semaphore.acquire().await {
            Ok(p) => p,
            Err(e) => {
                Log::warning(606, Some(e.to_string()));
                return None;
            }
        };
        for connection_mutex in &self.connections {
            if let Ok(mut db) = connection_mutex.try_lock() {
                let res = db.query_fast(index, params).await;
                drop(db);
                drop(permit);
                return res;
            }
        }
        drop(permit);
        Log::warning(607, None);
        None
    }
}
