use std::sync::Arc;

#[cfg(feature = "pgsql")]
use postgres::types::ToSql;

#[cfg(feature = "mssql")]
use tiberius::ToSql;

#[cfg(not(any(feature = "pgsql", feature = "mssql")))]
use super::without_sql::ToSql;

use tokio::sync::{Mutex, Semaphore};

use crate::sys::{data::Data, init::DBConfig, log::Log};

#[cfg(feature = "pgsql")]
use super::pgsql::PgSql;

#[cfg(feature = "mssql")]
use super::mssql::MsSql;

#[cfg(not(any(feature = "pgsql", feature = "mssql")))]
use super::without_sql::WithoutSql;

#[cfg(feature = "pgsql")]
type QueryParam<'a> = &'a [&'a (dyn ToSql + Sync)];

#[cfg(feature = "mssql")]
type QueryParam<'a> = &'a [&'a dyn ToSql];

#[cfg(not(any(feature = "pgsql", feature = "mssql")))]
type QueryParam<'a> = &'a [&'a dyn ToSql];

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
    /// Vector of database connections.
    #[cfg(not(any(feature = "pgsql", feature = "mssql")))]
    connections: Vec<Arc<Mutex<WithoutSql>>>,
    /// Semaphore for finding free connection.
    semaphore: Arc<Semaphore>,
    /// The database is in use
    is_used: bool,
}

impl DB {
    /// Initialize pool of database connections for asynchronous work.
    ///
    /// # Parameters
    ///
    /// * `size: usize` - Pool size; =0 - when install mode
    /// * `config: Arc<DBConfig>` - Configuration.
    ///
    /// # Return
    ///
    /// New poll of database connections for asynchronous work.
    pub(crate) async fn new(size: usize, config: Arc<DBConfig>) -> Option<DB> {
        let mut connections = Vec::with_capacity(size);
        let mut asize = 0;
        for _ in 0..size {
            #[cfg(feature = "pgsql")]
            let mut db = PgSql::new(Arc::clone(&config))?;
            #[cfg(feature = "mssql")]
            let mut db = MsSql::new(Arc::clone(&config))?;
            #[cfg(not(any(feature = "pgsql", feature = "mssql")))]
            let mut db = WithoutSql::new(Arc::clone(&config))?;
            if db.connect().await {
                asize += 1;
                connections.push(Arc::new(Mutex::new(db)));
            } else {
                Log::stop(610, None);
                return None;
            }
        }
        let semaphore = Arc::new(Semaphore::new(asize));

        Some(DB {
            connections,
            semaphore,
            is_used: asize > 0,
        })
    }
    /// Is library uses database
    pub fn in_use(&self) -> bool {
        self.is_used
    }

    pub(crate) async fn check_db(config: &DBConfig) -> Result<String, String> {
        #[cfg(feature = "pgsql")]
        return PgSql::check_db(config).await;

        #[cfg(feature = "mssql")]
        return MsSql::check_db(config).await;

        #[cfg(not(any(feature = "pgsql", feature = "mssql")))]
        return Ok(String::new());
    }

    /// Execute query to database synchronously
    ///
    /// # Parmeters
    ///
    /// * `query: &str` - SQL query;
    /// * `params: &[&(dyn ToSql + Sync)]` - Array of params. (for PgSql)
    /// * `params: &[&dyn ToSql]` - Array of params. (for MsSql)
    /// * `assoc: bool` - Return columns as associate array if True or Vecor id False.
    ///
    /// # Return
    ///
    /// * `Option::None` - When error query or diconnected;
    /// * `Option::Some(Vec<Data::Map>)` - Results, if assoc = true.
    /// * `Option::Some(Vec<Data::Vec>)` - Results, if assoc = false.
    pub async fn query<'a>(&self, query: &str, params: QueryParam<'a>, assoc: bool) -> Option<Vec<Data>> {
        if !self.is_used {
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
                let res = db.query(&query, params, assoc).await;
                drop(permit);
                return res;
            };
        }
        drop(permit);
        Log::warning(607, None);
        None
    }

    pub(crate) async fn query_prepare<'a>(&self, query: i64, params: QueryParam<'a>, assoc: bool) -> Option<Vec<Data>> {
        if !self.is_used {
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
                let res = db.query(&query, params, assoc).await;
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
    /// * `params: &[&(dyn ToSql + Sync)]` - Array of params. (for PgSql)
    /// * `params: &[&dyn ToSql]` - Array of params. (for MsSql)
    ///
    /// # Return
    ///
    /// * `Option::None` - When error query or diconnected;
    /// * `Option::Some(())` - Successed.
    pub async fn execute<'a>(&self, query: &str, params: QueryParam<'a>) -> Option<()> {
        if !self.is_used {
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
                let res = db.execute(&query, params).await;
                drop(permit);
                return res;
            };
        }
        drop(permit);
        Log::warning(607, None);
        None
    }

    pub(crate) async fn execute_prepare<'a>(&self, query: i64, params: QueryParam<'a>) -> Option<()> {
        if !self.is_used {
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
                let res = db.execute(&query, params).await;
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
    /// * `params: &[&(dyn ToSql + Sync)]` - Array of params. (for PgSql)
    /// * `params: &[&dyn ToSql]` - Array of params. (for MsSql)
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
    pub async fn query_group<'a>(
        &self,
        query: &str,
        params: QueryParam<'a>,
        assoc: bool,
        conds: &[&[impl StrOrI64OrUSize]],
    ) -> Option<Data> {
        if !self.is_used {
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
                let res = db.query_group(&query, params, assoc, conds).await;
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
