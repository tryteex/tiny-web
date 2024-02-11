use std::{collections::BTreeMap, sync::Arc};

use postgres::{types::ToSql, Row};
use tokio::sync::{Mutex, Semaphore};

use super::{
    action::Data,
    db_one::{DBOne, DBPrepare, KeyOrQuery},
    init::DBConfig,
    log::Log,
};

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
    connections: Vec<Arc<Mutex<DBOne>>>,
    /// Semaphore for finding free connection.
    semaphore: Arc<Semaphore>,
    /// Number of connected databases.
    pub size: usize,
}

impl DB {
    /// Initialize pool of database connections for asynchronous work.
    ///
    /// # Parameters
    ///
    /// * `size: usize` - Pool size;
    /// * `config: Arc<DBConfig>` - Configuration.
    /// * `prepare: BTreeMap<i64, DBPrepare>` - Prepare sql queries.
    ///
    /// # Return
    ///
    /// New poll of database connections for asynchronous work.
    pub async fn new(size: usize, config: Arc<DBConfig>, prepare: BTreeMap<i64, DBPrepare>) -> Option<DB> {
        let mut connections = Vec::with_capacity(size);
        let mut asize = 0;
        for _ in 0..size {
            let mut db = DBOne::new(Arc::clone(&config), prepare.clone())?;
            if db.connect().await {
                asize += 1;
                connections.push(Arc::new(Mutex::new(db)));
            } else {
                Log::stop(610, None);
                return None;
            }
        }
        let semaphore = Arc::new(Semaphore::new(asize));

        Some(DB { connections, semaphore, size: asize })
    }

    /// Execute query to database and return a raw result synchronously
    ///
    /// # Parmeters
    ///
    /// * `query: &str` - SQL query;
    /// * `query: i64` - Key of Statement;
    /// * `params: &[&(dyn ToSql + Sync)]` - Array of params.
    ///
    /// # Return
    ///
    /// * `Option::None` - When error query or diconnected;
    /// * `Option::Some(Vec<Row>)` - Results.
    pub async fn query_raw<T>(&self, query: T, params: &[&(dyn ToSql + Sync)]) -> Option<Vec<Row>>
    where
        T: for<'a> KeyOrQuery<'a>,
    {
        let permit = match self.semaphore.acquire().await {
            Ok(p) => p,
            Err(e) => {
                Log::warning(606, Some(e.to_string()));
                return None;
            }
        };
        for connection_mutex in &self.connections {
            if let Ok(mut db) = connection_mutex.try_lock() {
                let res = db.query_raw(&query, params).await;
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
    /// * `text: &str` - SQL query;
    /// * `text: i64` - Key of Statement;
    /// * `params: &[&(dyn ToSql + Sync)]` - Array of params.
    /// * `assoc: bool` - Return columns as associate array if True or Vecor id False.
    ///
    /// # Return
    ///
    /// * `Option::None` - When error query or diconnected;
    /// * `Option::Some(Vec<Data::Map>)` - Results, if assoc = true.
    /// * `Option::Some(Vec<Data::Vec>)` - Results, if assoc = false.
    pub async fn query<T>(&self, query: &T, params: &[&(dyn ToSql + Sync)], assoc: bool) -> Option<Vec<Data>>
    where
        T: for<'a> KeyOrQuery<'a>,
    {
        let permit = match self.semaphore.acquire().await {
            Ok(p) => p,
            Err(e) => {
                Log::warning(606, Some(e.to_string()));
                return None;
            }
        };
        for connection_mutex in &self.connections {
            if let Ok(mut db) = connection_mutex.try_lock() {
                let res = db.query(query, params, assoc).await;
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
