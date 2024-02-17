use std::{
    collections::{btree_map::Entry, BTreeMap},
    sync::Arc,
};

use chrono::{DateTime, Utc};
use native_tls::Protocol;
use postgres::{types::ToSql, NoTls, Row, Statement, ToStatement};
use postgres_native_tls::MakeTlsConnector;
use serde_json::Value;
use tokio_postgres::{types::Type, Column};

use tiny_web_macro::fnv1a_64;

use super::{action::Data, init::DBConfig, log::Log};

/// Response to the result of the query
///
/// # Properties
///
/// * `Ok(Vec<Row>)` - The request was completed successfully;
/// * `NoClient` - Connection is empty;
/// * `ErrQuery(String)` - Query execution error;
/// * `ErrConnect(String)` - Connection is lost.
#[derive(Debug)]
enum DBResult {
    /// The request was completed successfully.
    Ok(Vec<Row>),
    /// Connection is empty.
    NoClient,
    /// Query execution error.
    ErrQuery(String),
    /// Connection is lost.
    ErrConnect(String),
    /// No prepare query
    ErrPrepare,
}

/// Responsible for working with postgresql database:
///
/// # Properties
///
/// * `client: Option<tokio_postgres::Client>` - Client for connection to database;
/// * `sql_conn: tokio_postgres::Config` - Connection config;
/// * `tls: Option<MakeTlsConnector>` - Use tls for connection when sslmode=require;
/// * `zone: Option<String>` - Time zone to init database;
/// * `prepare: Vec<DBStatement>` - Prepare statement to database.
pub struct DBOne {
    /// Client for connection to database.
    client: Option<tokio_postgres::Client>,
    /// Connection config.
    sql_conn: tokio_postgres::Config,
    /// Use tls for connection when sslmode=require.
    tls: Option<MakeTlsConnector>,
    /// Time zone to init database.
    zone: Option<String>,
    /// Prepare statements to database.
    prepare: BTreeMap<i64, DBStatement>,
    /// External prepare statements to database.
    external: BTreeMap<i64, DBPrepare>,
}

/// External prepare statements
#[derive(Debug, Clone)]
pub struct DBPrepare {
    /// Query string
    pub query: String,
    /// Prepare types
    pub types: Vec<Type>,
}

/// Statement to database
///
/// # Properties
///
/// * `statement: Statement` - Statement to database.
/// * `sql: String` - Sql query to database.
pub struct DBStatement {
    /// Statement to database
    statement: Statement,
    /// Sql query to database
    sql: String,
}

/// Search correct type to query
pub enum KeyStatement<'a> {
    Key(&'a DBStatement),
    Query(&'a str),
}

/// Names of columns
type ColumnName = (usize, fn(&Row, usize) -> Data);

impl DBOne {
    /// Initializes a new object `DBOne`
    ///
    /// # Parameters
    ///
    /// * `config: Arc<DBConfig>` - database configuration.
    ///
    /// # Return
    ///
    /// * `DB` - new DB object
    pub fn new(config: Arc<DBConfig>, prepare: BTreeMap<i64, DBPrepare>) -> Option<DBOne> {
        let mut conn_str = String::with_capacity(512);
        //host
        conn_str.push_str("host='");
        conn_str.push_str(&config.host);
        conn_str.push_str("' ");
        //port
        if let Some(p) = &config.port {
            conn_str.push_str("port='");
            conn_str.push_str(&p.to_string());
            conn_str.push_str("' ");
        }
        // Database name
        conn_str.push_str("dbname='");
        conn_str.push_str(&config.name);
        conn_str.push_str("' ");
        //user
        if let Some(u) = &config.user {
            conn_str.push_str("user='");
            conn_str.push_str(u);
            conn_str.push_str("' ");
        }
        //password
        if let Some(p) = &config.pwd {
            conn_str.push_str("password='");
            conn_str.push_str(p);
            conn_str.push_str("' ");
        }
        //sslmode
        if config.sslmode {
            conn_str.push_str("sslmode=require ");
        }
        //connect_timeout
        conn_str.push_str("connect_timeout=1 ");
        //application_name
        conn_str.push_str("application_name='");
        conn_str.push_str(env!("CARGO_PKG_NAME"));
        conn_str.push(' ');
        conn_str.push_str(env!("CARGO_PKG_VERSION"));
        conn_str.push_str("' ");
        //options
        conn_str.push_str("options='--client_encoding=UTF8'");

        let sql_conn: tokio_postgres::Config = match conn_str.parse() {
            Ok(c) => c,
            Err(e) => {
                Log::stop(609, Some(e.to_string()));
                return None;
            }
        };
        let tls = if config.sslmode {
            match native_tls::TlsConnector::builder()
                .danger_accept_invalid_certs(true)
                .min_protocol_version(Some(Protocol::Tlsv12))
                .build()
            {
                Ok(connector) => Some(MakeTlsConnector::new(connector)),
                Err(e) => {
                    Log::stop(600, Some(e.to_string()));
                    return None;
                }
            }
        } else {
            None
        };
        Some(DBOne {
            client: None,
            sql_conn,
            tls,
            zone: config.zone.clone(),
            prepare: BTreeMap::new(),
            external: prepare,
        })
    }

    /// Connect to the database
    ///
    /// # Return
    ///
    /// * `true` - the connection was successful;
    /// * `false` - the connection was fail.
    pub async fn connect(&mut self) -> bool {
        match &self.client {
            Some(c) => {
                if c.is_closed() {
                    self.try_connect().await
                } else {
                    true
                }
            }
            None => self.try_connect().await,
        }
    }

    /// Trying to connect to the database
    ///
    /// # Return
    ///
    /// * `true` - the connection was successful;
    /// * `false` - the connection was fail.
    async fn try_connect(&mut self) -> bool {
        match self.tls.clone() {
            Some(tls) => match self.sql_conn.connect(tls).await {
                Ok((client, connection)) => {
                    tokio::spawn(async move {
                        if let Err(e) = connection.await {
                            Log::stop(612, Some(e.to_string()));
                        }
                    });
                    self.client = Some(client);
                }
                Err(e) => {
                    Log::warning(601, Some(format!("Error: {} => {:?}", e, &self.sql_conn)));
                    return false;
                }
            },
            None => match self.sql_conn.connect(NoTls).await {
                Ok((client, connection)) => {
                    tokio::spawn(async move {
                        if let Err(e) = connection.await {
                            Log::warning(612, Some(e.to_string()));
                        }
                    });
                    self.client = Some(client);
                }
                Err(e) => {
                    Log::warning(601, Some(format!("Error: {} => {:?}", e, &self.sql_conn)));
                    return false;
                }
            },
        }
        if let Some(z) = &self.zone {
            let query = format!("SET timezone TO '{}';", z);
            match DBOne::exec(&self.client, &query, &[]).await {
                DBResult::Ok(_) => (),
                _ => {
                    Log::warning(602, Some(query));
                    return false;
                }
            }
        }
        self.prepare().await
    }

    /// Prepare sql statement
    ///
    /// # Return
    ///
    /// * `true` - the operation was successful;
    /// * `false` - the operation was fail.
    async fn prepare(&mut self) -> bool {
        self.prepare.clear();
        match &self.client {
            Some(client) => {
                let mut map = BTreeMap::new();

                // *0 Get session
                let sql = r#"
                    WITH upd AS (
                        UPDATE session
                        SET 
                            last = now()
                        WHERE
                            session=$1
                        RETURNING session_id, user_id, data, lang_id
                    )
                    SELECT 
                        s.session_id, s.user_id, u.role_id, s.data, s.user_id, s.lang_id 
                    FROM 
                        upd s
                        INNER JOIN "user" u ON u.user_id=s.user_id
                "#;
                map.insert(fnv1a_64!("lib_get_session"), (client.prepare_typed(sql, &[Type::TEXT]), sql.to_owned()));

                // *1 Update session
                let sql = "
                    UPDATE session
                    SET 
                        user_id=$1,
                        lang_id=$2,
                        data=$3,
                        last=now(),
                        ip=$4,
                        user_agent=$5
                    WHERE
                        session_id=$6
                ";
                map.insert(
                    fnv1a_64!("lib_set_session"),
                    (
                        client.prepare_typed(sql, &[Type::INT8, Type::INT8, Type::BYTEA, Type::TEXT, Type::TEXT, Type::INT8]),
                        sql.to_owned(),
                    ),
                );

                // *2 Insert session
                let sql = "
                    INSERT INTO session (user_id, lang_id, session, data, created, last, ip, user_agent)
                    SELECT $1, $2, $3, $4, now(), now(), $5, $6
                ";
                map.insert(
                    fnv1a_64!("lib_add_session"),
                    (
                        client.prepare_typed(sql, &[Type::INT8, Type::INT8, Type::TEXT, Type::BYTEA, Type::TEXT, Type::TEXT]),
                        sql.to_owned(),
                    ),
                );

                // *3 Get redirect
                let sql = "
                    SELECT redirect, permanently FROM redirect WHERE url=$1
                ";
                map.insert(fnv1a_64!("lib_get_redirect"), (client.prepare_typed(sql, &[Type::TEXT]), sql.to_owned()));

                // *4 Get route
                let sql = "
                    SELECT 
                        c.module, c.class, c.action,
                        c.module_id, c.class_id, c.action_id,
                        r.params, r.lang_id
                    FROM 
                        route r
                        INNER JOIN controller c ON c.controller_id=r.controller_id
                    WHERE r.url=$1
                ";
                map.insert(fnv1a_64!("lib_get_route"), (client.prepare_typed(sql, &[Type::TEXT]), sql.to_owned()));
                // *12 Get route from module/class/action
                let sql = r#"
                    SELECT r.url 
                    FROM 
                        controller c
                        INNER JOIN route r ON 
                            r.controller_id=c.controller_id AND r.lang_id=$5 AND r.params = $4
                    WHERE 
                        c.module_id=$1 AND c.class_id=$2 AND c.action_id=$3
                "#;
                map.insert(
                    fnv1a_64!("lib_get_url"),
                    (client.prepare_typed(sql, &[Type::INT8, Type::INT8, Type::INT8, Type::TEXT, Type::INT8]), sql.to_owned()),
                );

                // *5 Get auth permissions
                let sql = r#"
                    SELECT COALESCE(MAX(a.access::int), 0)::bool AS access
                    FROM
                        access a
                        INNER JOIN "user" u ON u.role_id=a.role_id
                        INNER JOIN controller c ON a.controller_id=c.controller_id
                    WHERE
                        a.access AND a.role_id=$1 AND (
                            (c.module_id=-3750763034362895579 AND c.class_id=-3750763034362895579 AND c.action_id=-3750763034362895579)
                            OR (c.module_id=$2 AND c.class_id=-3750763034362895579 AND c.action_id=-3750763034362895579)
                            OR (c.module_id=$3 AND c.class_id=$5 AND c.action_id=-3750763034362895579)
                            OR (c.module_id=$4 AND c.class_id=$6 AND c.action_id=$7)
                        )
                "#;
                map.insert(
                    fnv1a_64!("lib_get_auth"),
                    (
                        client.prepare_typed(
                            sql,
                            &[Type::INT8, Type::INT8, Type::INT8, Type::INT8, Type::INT8, Type::INT8, Type::INT8],
                        ),
                        sql.to_owned(),
                    ),
                );
                // *6 Get not found
                let sql = "
                    SELECT url
                    FROM route
                    WHERE controller_id=3 AND lang_id=$1
                ";
                map.insert(fnv1a_64!("lib_get_not_found"), (client.prepare_typed(sql, &[Type::INT8]), sql.to_owned()));
                // *9 Get settings
                let sql = r#"
                    SELECT data FROM setting WHERE key=$1
                "#;
                map.insert(fnv1a_64!("lib_get_setting"), (client.prepare_typed(sql, &[Type::INT8]), sql.to_owned()));
                // *7 Insert email
                let sql = r#"
                    INSERT INTO mail(user_id, mail, "create", err, transport)
                    VALUES ($1, $2, now(), false, $3)
                    RETURNING mail_id;
                "#;
                map.insert(
                    fnv1a_64!("lib_mail_new"),
                    (client.prepare_typed(sql, &[Type::INT8, Type::JSON, Type::TEXT]), sql.to_owned()),
                );
                // *8 Insert email without provider
                let sql = r#"
                    INSERT INTO mail(user_id, mail, "create", send, err, transport)
                    VALUES ($1, $2, now(), now(), false, 'None')
                "#;
                map.insert(fnv1a_64!("lib_mail_add"), (client.prepare_typed(sql, &[Type::INT8, Type::JSON]), sql.to_owned()));
                // *10 Insert error send email
                let sql = r#"
                    UPDATE mail
                    SET err=true, send=now(), err_text=$1
                    WHERE mail_id=$2
                "#;
                map.insert(fnv1a_64!("lib_mail_err"), (client.prepare_typed(sql, &[Type::TEXT, Type::INT8]), sql.to_owned()));
                // *11 Insert success send email
                let sql = r#"
                    UPDATE mail
                    SET err=false, send=now()
                    WHERE mail_id=$1
                "#;
                map.insert(fnv1a_64!("lib_mail_ok"), (client.prepare_typed(sql, &[Type::INT8]), sql.to_owned()));

                // Add config prepare
                for (key, sql) in &self.external {
                    map.insert(*key, (client.prepare_typed(&sql.query, &sql.types), sql.query.to_owned()));
                }

                // Prepare statements
                for (key, (prepare, sql)) in map {
                    match prepare.await {
                        Ok(s) => {
                            self.prepare.insert(key, DBStatement { statement: s, sql });
                        }
                        Err(e) => {
                            Log::stop(613, Some(format!("Error={}. sql={}", e, sql)));
                            return false;
                        }
                    }
                }
                true
            }
            None => false,
        }
    }

    /// Executes a statement, returning the result
    ///
    /// # Parmeters
    ///
    /// * `query: &str` - SQL query;
    /// * `params: &[&(dyn ToSql + Sync)]` - Array of params;
    ///
    /// # Return
    ///
    /// * `DBResult` - Results of query.
    async fn exec<T>(client: &Option<tokio_postgres::Client>, query: &T, params: &[&(dyn ToSql + Sync)]) -> DBResult
    where
        T: ?Sized + ToStatement,
    {
        match client {
            Some(sql) => match sql.query(query, params).await {
                Ok(res) => DBResult::Ok(res),
                Err(e) => {
                    if e.is_closed() {
                        DBResult::ErrConnect(e.to_string())
                    } else {
                        DBResult::ErrQuery(e.to_string())
                    }
                }
            },
            None => DBResult::NoClient,
        }
    }

    /// Execute one query to the database
    async fn query_db(&self, query: &impl KeyOrQuery, params: &[&(dyn ToSql + Sync)]) -> DBResult {
        if query.is_key() {
            let stat = match self.prepare.get(&query.to_i64()) {
                Some(s) => s,
                None => return DBResult::ErrPrepare,
            };
            DBOne::exec(&self.client, &stat.statement, params).await
        } else {
            DBOne::exec(&self.client, query.to_str(), params).await
        }
    }

    /// Execute query to database and return a raw result
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
    pub async fn query_raw(&mut self, query: impl KeyOrQuery, params: &[&(dyn ToSql + Sync)]) -> Option<Vec<Row>> {
        match self.query_db(&query, params).await {
            DBResult::Ok(r) => return Some(r),
            DBResult::ErrQuery(e) => {
                if query.is_key() {
                    Log::warning(602, Some(format!("Statement key={} error={}", query.to_i64(), e)));
                } else {
                    Log::warning(602, Some(format!("{} error={}", query.to_str(), e)));
                }
                return None;
            }
            DBResult::ErrPrepare => {
                Log::warning(615, Some(format!("{:?}", query.to_i64())));
                return None;
            }
            DBResult::NoClient => Log::warning(604, None),
            DBResult::ErrConnect(e) => Log::warning(603, Some(e)),
        };
        self.client = None;
        if self.try_connect().await {
            if let DBResult::Ok(r) = self.query_db(&query, params).await {
                return Some(r);
            }
        }
        None
    }

    /// Execute query to database and return a result
    ///
    /// # Parmeters
    ///
    /// * `query: &str` - SQL query;
    /// * `query: i64` - Key of Statement;
    /// * `params: &[&(dyn ToSql + Sync)]` - Array of params.
    /// * `assoc: bool` - Return columns as associate array if True or Vecor id False.
    ///
    /// # Return
    ///
    /// * `Option::None` - When error query or diconnected;
    /// * `Option::Some(Vec<Data::Map>)` - Results, if assoc = true.
    /// * `Option::Some(Vec<Data::Vec>)` - Results, if assoc = false.
    pub async fn query(&mut self, query: impl KeyOrQuery, params: &[&(dyn ToSql + Sync)], assoc: bool) -> Option<Vec<Data>> {
        let rows = self.query_raw(query, params).await?;
        if rows.is_empty() {
            return Some(Vec::new());
        };

        Some(self.convert(rows, assoc))
    }

    /// Execute query to database and return a result,  
    /// and grouping tabular data according to specified conditions.
    ///
    /// # Parmeters
    ///
    /// * `query: &str` - SQL query;
    /// * `query: i64` - Key of Statement;
    /// * `params: &[&(dyn ToSql + Sync)]` - Array of params.
    /// * `assoc: bool` - Return columns as associate array if True or Vecor id False.
    /// * `conds: Vec<Vec<&str>>` - Grouping condition.  
    ///
    /// Grouping condition:
    /// * The number of elements in the first-level array corresponds to the hierarchy levels in the group.
    /// * The number of elements in the second-level array corresponds to the number of items in one hierarchy. The first element of the group (index=0) is considered unique.
    /// * &str - field names for `Data::Vec<Data::Map<...>>`.
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
    /// * `Some(Data::Map<cond[0][0], Data::Vec<...>>)` in hierarchical structure.  
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
        &mut self,
        query: impl KeyOrQuery,
        params: &[&(dyn ToSql + Sync)],
        assoc: bool,
        conds: &[&[impl StrOrI64OrUSize]],
    ) -> Option<Data> {
        if conds.is_empty() {
            return None;
        }
        let rows = self.query_raw(query, params).await?;
        if rows.is_empty() {
            return Some(Data::Map(BTreeMap::new()));
        }
        if assoc {
            Some(self.convert_map(rows, conds))
        } else {
            Some(self.convert_vec(rows, conds))
        }
    }

    /// Convert Vec<Row> to Data::Map<Data::Map<...>>
    fn convert_map(&self, rows: Vec<Row>, conds: &[&[impl StrOrI64OrUSize]]) -> Data {
        let mut map = BTreeMap::new();
        let cols = unsafe { rows.get_unchecked(0) }.columns();
        let columns = self.get_column_type_name(cols);
        for row in &rows {
            let mut item = &mut map;
            for row_conds in conds {
                if row_conds.is_empty() {
                    break;
                }
                item = match self.fill_map(row, &columns, row_conds, item) {
                    Some(i) => i,
                    None => break,
                };
            }
        }
        Data::Map(map)
    }

    /// Convert Vec<Row> to Data::Map<Data::Vec<...>>
    fn convert_vec(&self, rows: Vec<Row>, conds: &[&[impl StrOrI64OrUSize]]) -> Data {
        let mut map = BTreeMap::new();
        let cols = unsafe { rows.get_unchecked(0) }.columns();
        let columns = self.get_column_type(cols);
        for row in &rows {
            let mut item = &mut map;
            for row_conds in conds {
                if row_conds.is_empty() {
                    break;
                }
                item = match self.fill_vec(row, &columns, row_conds, item) {
                    Some(i) => i,
                    None => break,
                };
            }
        }
        Data::Map(map)
    }

    /// Fill tree items in map
    fn fill_map<'a>(
        &self,
        row: &Row,
        columns: &BTreeMap<i64, ColumnName>,
        conds: &[impl StrOrI64OrUSize],
        map: &'a mut BTreeMap<i64, Data>,
    ) -> Option<&'a mut BTreeMap<i64, Data>> {
        let mut index = unsafe { conds.get_unchecked(0) }.to_i64();
        if index == 0 {
            return None;
        }
        let (idx, func) = match columns.get(&index) {
            Some(f) => f,
            None => return None,
        };
        let val = if let Data::I64(val) = func(row, *idx) {
            val
        } else {
            return None;
        };
        let res_map = match map.entry(val) {
            Entry::Vacant(v) => {
                let mut new_map = BTreeMap::new();
                new_map.insert(index, Data::I64(val));
                let mut turple;
                for item in &conds[1..] {
                    index = item.to_i64();
                    if index == 0 {
                        return None;
                    }
                    turple = match columns.get(&index) {
                        Some(f) => f,
                        None => return None,
                    };
                    new_map.insert(index, turple.1(row, turple.0));
                }
                new_map.insert(fnv1a_64!("sub"), Data::Map(BTreeMap::new()));
                v.insert(Data::Map(new_map))
            }
            Entry::Occupied(o) => o.into_mut(),
        };
        if let Data::Map(found_map) = res_map {
            if let Data::Map(submap) = found_map.get_mut(&fnv1a_64!("sub"))? {
                Some(submap)
            } else {
                None
            }
        } else {
            None
        }
    }

    /// Fill tree items in vec
    fn fill_vec<'a>(
        &self,
        row: &Row,
        columns: &[fn(&Row, usize) -> Data],
        conds: &[impl StrOrI64OrUSize],
        map: &'a mut BTreeMap<i64, Data>,
    ) -> Option<&'a mut BTreeMap<i64, Data>> {
        let mut index = unsafe { conds.get_unchecked(0) }.to_usize();
        if index == usize::MAX {
            return None;
        }
        let mut func = unsafe { columns.get_unchecked(index) };
        let val = if let Data::I64(val) = func(row, index) {
            val
        } else {
            return None;
        };
        let res_map = match map.entry(val) {
            Entry::Vacant(v) => {
                let mut new_vec = Vec::with_capacity(conds.len() + 1);
                new_vec.push(Data::I64(val));
                for item in &conds[1..] {
                    index = item.to_usize();
                    if index == usize::MAX {
                        return None;
                    }
                    func = unsafe { columns.get_unchecked(index) };
                    new_vec.push(func(row, index));
                }
                new_vec.push(Data::Map(BTreeMap::new()));
                v.insert(Data::Vec(new_vec))
            }
            Entry::Occupied(o) => o.into_mut(),
        };
        if let Data::Vec(found_vec) = res_map {
            if let Data::Map(submap) = found_vec.last_mut()? {
                Some(submap)
            } else {
                None
            }
        } else {
            None
        }
    }

    /// Convert Vec<Row> to Vec<Data>
    fn convert(&self, rows: Vec<Row>, assoc: bool) -> Vec<Data> {
        let mut vec = Vec::with_capacity(rows.len());
        let cols = unsafe { rows.get_unchecked(0) }.columns();
        if !assoc {
            let columns = self.get_column_type(cols);
            let mut func;
            for row in &rows {
                let mut v = Vec::with_capacity(columns.len());
                for idx in 0..columns.len() {
                    func = unsafe { columns.get_unchecked(idx) };
                    v.push(func(row, idx))
                }
                vec.push(Data::Vec(v));
            }
        } else {
            let columns = self.get_column_type_name(cols);
            for row in &rows {
                let mut t = BTreeMap::new();
                for (name, turple) in &columns {
                    t.insert(*name, turple.1(row, turple.0));
                }
                vec.push(Data::Map(t));
            }
        }
        vec
    }

    /// Detect columns' type with columns' name
    fn get_column_type<'a>(&self, cols: &'a [Column]) -> Vec<fn(&Row, usize) -> Data> {
        let mut columns = Vec::with_capacity(cols.len());
        for col in cols {
            let func = match col.type_() {
                &Type::BOOL => Self::get_bool,
                &Type::BYTEA => Self::get_bytea,
                &Type::TEXT => Self::get_string,
                &Type::JSON => Self::get_json,
                &Type::JSONB => Self::get_json,
                &Type::UUID => Self::get_uuid,
                &Type::VARCHAR => Self::get_string,
                &Type::INT8 => Self::get_i64,
                &Type::INT2 => Self::get_i16,
                &Type::INT4 => Self::get_i32,
                &Type::FLOAT4 => Self::get_f32,
                &Type::FLOAT8 => Self::get_f64,
                &Type::TIMESTAMPTZ => Self::get_date,
                u => {
                    Log::warning(614, Some(format!("Type: {}", u)));
                    Self::get_unknown
                }
            };
            columns.push(func);
        }
        columns
    }

    /// Detect columns' type with columns' name
    fn get_column_type_name(&self, cols: &[Column]) -> BTreeMap<i64, ColumnName> {
        let mut columns = BTreeMap::new();
        for (idx, col) in cols.iter().enumerate() {
            let func = match col.type_() {
                &Type::BOOL => Self::get_bool,
                &Type::BYTEA => Self::get_bytea,
                &Type::TEXT => Self::get_string,
                &Type::JSON => Self::get_json,
                &Type::JSONB => Self::get_json,
                &Type::UUID => Self::get_uuid,
                &Type::VARCHAR => Self::get_string,
                &Type::INT8 => Self::get_i64,
                &Type::INT2 => Self::get_i16,
                &Type::INT4 => Self::get_i32,
                &Type::FLOAT4 => Self::get_f32,
                &Type::FLOAT8 => Self::get_f64,
                &Type::TIMESTAMPTZ => Self::get_date,
                u => {
                    Log::warning(614, Some(format!("Type: {}", u)));
                    Self::get_unknown
                }
            };
            columns.insert(crate::fnv1a_64(col.name().as_bytes()), (idx, func));
        }
        columns
    }

    /// Unknown Row type to Data::None
    #[inline]
    fn get_unknown(_: &Row, _: usize) -> Data {
        Data::None
    }

    /// Row::i16 to Data::I16
    #[inline]
    fn get_i16(row: &Row, idx: usize) -> Data {
        let i: Option<i16> = row.get(idx);
        match i {
            Some(i) => Data::I16(i),
            None => Data::None,
        }
    }

    /// Row::i32 to Data::I32
    #[inline]
    fn get_i32(row: &Row, idx: usize) -> Data {
        let i: Option<i32> = row.get(idx);
        match i {
            Some(i) => Data::I32(i),
            None => Data::None,
        }
    }

    /// Row::f32 to Data::F32
    #[inline]
    fn get_f32(row: &Row, idx: usize) -> Data {
        let f: Option<f32> = row.get(idx);
        match f {
            Some(f) => Data::F32(f),
            None => Data::None,
        }
    }

    /// Row::Uuid to Data::String
    #[inline]
    fn get_uuid(row: &Row, idx: usize) -> Data {
        let u: Option<uuid::Uuid> = row.get(idx);
        match u {
            Some(u) => Data::String(u.to_string()),
            None => Data::None,
        }
    }

    /// Row::Json to Data::Json
    #[inline]
    fn get_json(row: &Row, idx: usize) -> Data {
        let j: Option<Value> = row.get(idx);
        match j {
            Some(j) => Data::Json(j),
            None => Data::None,
        }
    }

    /// Row::DateTime<Utc> to Data::DateTime<Utc>
    #[inline]
    fn get_date(row: &Row, idx: usize) -> Data {
        let d: Option<DateTime<Utc>> = row.get(idx);
        match d {
            Some(d) => Data::Date(d),
            None => Data::None,
        }
    }

    /// Row::f64 to Data::F64
    #[inline]
    fn get_f64(row: &Row, idx: usize) -> Data {
        let f: Option<f64> = row.get(idx);
        match f {
            Some(f) => Data::F64(f),
            None => Data::None,
        }
    }

    /// Row::i64 to Data::I64
    #[inline]
    fn get_i64(row: &Row, idx: usize) -> Data {
        let i: Option<i64> = row.get(idx);
        match i {
            Some(i) => Data::I64(i),
            None => Data::None,
        }
    }

    /// Row::String to Data::String
    #[inline]
    fn get_string(row: &Row, idx: usize) -> Data {
        let s: Option<String> = row.get(idx);
        match s {
            Some(s) => Data::String(s),
            None => Data::None,
        }
    }

    /// Row::Vec<u8> to Data::Raw
    #[inline]
    fn get_bytea(row: &Row, idx: usize) -> Data {
        let r: Option<Vec<u8>> = row.get(idx);
        match r {
            Some(r) => Data::Raw(r),
            None => Data::None,
        }
    }

    /// Row::Bool to Data::Bool
    #[inline]
    fn get_bool(row: &Row, idx: usize) -> Data {
        let b: Option<bool> = row.get(idx);
        match b {
            Some(b) => Data::Bool(b),
            None => Data::None,
        }
    }
}

impl std::fmt::Debug for DBOne {
    /// Formats the value using the given formatter.
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        let tls = self.tls.clone().map(|_| "TlsConnector");
        let DBOne {
            client,
            sql_conn,
            tls: _,
            zone,
            prepare,
            external,
        } = self;
        f.debug_struct("DB")
            .field("client", &client)
            .field("sql_conn", &sql_conn)
            .field("tls", &tls)
            .field("zone", &zone)
            .field("prepare", &prepare)
            .field("external", &external)
            .finish()
    }
}

impl std::fmt::Debug for DBStatement {
    /// Formats the value using the given formatter.
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        let DBStatement { statement, sql } = self;
        f.debug_struct("DBStatement")
            .field("sql", &sql)
            .field("columns", &statement.columns())
            .field("params", &statement.params())
            .finish()
    }
}

/// Trait representing types that can be converted to a query or a key statement.
pub trait KeyOrQuery {
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
