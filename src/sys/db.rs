use std::{collections::BTreeMap, sync::Arc};

use postgres::{types::ToSql, Row};
use tokio::sync::{Mutex, Semaphore};

use super::{
    action::Data,
    db_one::{DBOne, DBPrepare, KeyOrQuery, StrOrI64OrUSize},
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
    pub async fn query_raw(&self, query: impl KeyOrQuery, params: &[&(dyn ToSql + Sync)]) -> Option<Vec<Row>> {
        let permit = match self.semaphore.acquire().await {
            Ok(p) => p,
            Err(e) => {
                Log::warning(606, Some(e.to_string()));
                return None;
            }
        };
        for connection_mutex in &self.connections {
            if let Ok(mut db) = connection_mutex.try_lock() {
                let res = db.query_raw(query, params).await;
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
    pub async fn query(&self, query: impl KeyOrQuery, params: &[&(dyn ToSql + Sync)], assoc: bool) -> Option<Vec<Data>> {
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

    /// Execute query to database and return a result,  
    /// and grouping tabular data according to specified conditions.
    ///
    /// # Parmeters
    ///
    /// * `text: &str` - SQL query;
    /// * `text: i64` - Key of Statement;
    /// * `params: &[&(dyn ToSql + Sync)]` - Array of params.
    /// * `assoc: bool` - Return columns as associate array if True or Vecor id False.
    /// * `conds: Vec<Vec<&str>>` - Grouping condition.  
    ///
    /// Grouping condition:
    /// * The number of elements in the first-level array corresponds to the hierarchy levels in the group.
    /// * The number of elements in the second-level array corresponds to the number of items in one hierarchy. The first element of the group (index=0) is considered unique.
    /// * &str - field names for ```Data::Vec<Data::Map<...>>```.
    /// The first value in the second-level array must be of type ```Data::I64```.
    ///
    /// For each group, a new field with the name ```sub``` (encoded using ```fnv1a_64```) will be created, where child groups will be located.
    ///
    /// If the data does not match the format ```Data::Vec<Data::Map<...>>```, grouping will not occur, ```Option::None``` will be returned.  
    /// If the data does not match the tabular format, grouping will not occur, ```Option::None``` will be returned.
    ///
    /// Fields that are not included in the group will be excluded.
    ///
    /// # Return
    /// * Option::None - If the fields failed to group.  
    /// ## if assoc = true  
    /// * ```Some(Data::Map<cond[0][0], Data::Map<...>>)``` in hierarchical structure.  
    /// ```struct
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
    /// ```
    /// ## if assoc = false  
    /// * ```Some(Data::Map<cond[0][0], Data::Map<...>>)``` in hierarchical structure.  
    /// ```struct
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
    /// ```
    pub async fn query_group(
        &self,
        query: impl KeyOrQuery,
        params: &[&(dyn ToSql + Sync)],
        assoc: bool,
        conds: &[&[impl StrOrI64OrUSize]],
    ) -> Option<Data> {
        let permit = match self.semaphore.acquire().await {
            Ok(p) => p,
            Err(e) => {
                Log::warning(606, Some(e.to_string()));
                return None;
            }
        };
        for connection_mutex in &self.connections {
            if let Ok(mut db) = connection_mutex.try_lock() {
                let res = db.query_group(query, params, assoc, conds).await;
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
