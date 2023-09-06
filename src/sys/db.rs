use std::{collections::BTreeMap, sync::Arc};

use chrono::{DateTime, Utc};
use native_tls::Protocol;
use postgres::{types::ToSql, NoTls, Row, Statement, ToStatement};
use postgres_native_tls::MakeTlsConnector;
use serde_json::Value;
use tokio_postgres::types::Type;

use crate::fnv1a_64;

use super::{action::Data, init::DBConfig, log::Log};

const PREPARE_CAPACITY: usize = 8;

/// Response to the result of the query
///
/// # Properties
///
/// * `Ok(Vec<Row>)` - The request was completed successfully;
/// * `NoClient` - Connection is empty;
/// * `ErrQuery(String)` - Query execution error;
/// * `ErrConnect(String)` - Connection is lost.
enum DBResult {
    /// The request was completed successfully.
    Ok(Vec<Row>),
    /// Connection is empty.
    NoClient,
    /// Query execution error.
    ErrQuery(String),
    /// Connection is lost.
    ErrConnect(String),
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
pub struct DB {
    /// Client for connection to database.
    client: Option<tokio_postgres::Client>,
    /// Connection config.
    sql_conn: tokio_postgres::Config,
    /// Use tls for connection when sslmode=require.
    tls: Option<MakeTlsConnector>,
    /// Time zone to init database.
    zone: Option<String>,
    /// Prepare statement to database.
    prepare: Vec<DBStatement>,
}

/// Statement to database
///
/// # Properties
///
/// * `statement: Statement` - Statement to database.
/// * `sql: &'static str` - Sql query to database.
struct DBStatement {
    statement: Statement,
    sql: &'static str,
}

impl DB {
    /// Initializes a new object `DB`
    ///
    /// # Parameters
    ///
    /// * `config: Arc<DBConfig>` - database configuration.
    ///
    /// # Return
    ///
    /// * `DB` - new DB object
    pub async fn new(config: Arc<DBConfig>) -> DB {
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
                Log::warning(609, Some(e.to_string()));
                tokio_postgres::Config::new()
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
                    Log::warning(600, Some(e.to_string()));
                    None
                }
            }
        } else {
            None
        };
        DB {
            client: None,
            sql_conn,
            tls,
            zone: config.zone.clone(),
            prepare: Vec::with_capacity(PREPARE_CAPACITY),
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
                    Log::stop(601, Some(format!("Error: {} => {:?}", e, &self.sql_conn)));
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
                    Log::stop(601, Some(format!("Error: {} => {:?}", e, &self.sql_conn)));
                    return false;
                }
            },
        }
        if let Some(z) = &self.zone {
            let query = format!("SET timezone TO '{}';", z);
            match self.exec(&query, &[]).await {
                DBResult::Ok(_) => (),
                _ => {
                    Log::stop(602, Some(query));
                    return false;
                }
            }
        }
        self.prepare().await
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

    /// Prepare sql statement
    ///
    /// # Return
    ///
    /// * `true` - the operation was successful;
    /// * `false` - the operation was fail.
    async fn prepare(&mut self) -> bool {
        match &self.client {
            Some(client) => {
                let mut vec = Vec::with_capacity(PREPARE_CAPACITY);

                // 0 Get session
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
                vec.push((client.prepare_typed(sql, &[Type::TEXT]), sql));

                // 1 Update session
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
                vec.push((
                    client.prepare_typed(sql, &[Type::INT8, Type::INT8, Type::BYTEA, Type::TEXT, Type::TEXT, Type::INT8]),
                    sql,
                ));

                // 2 Insert session
                let sql = "
                    INSERT INTO session (user_id, lang_id, session, data, created, last, ip, user_agent)
                    SELECT $1, $2, $3, $4, now(), now(), $5, $6
                ";
                vec.push((
                    client.prepare_typed(sql, &[Type::INT8, Type::INT8, Type::TEXT, Type::BYTEA, Type::TEXT, Type::TEXT]),
                    sql,
                ));

                // 3 Get redirect
                let sql = "
                    SELECT redirect, permanently FROM redirect WHERE url=$1
                ";
                vec.push((client.prepare_typed(sql, &[Type::TEXT]), sql));

                // 4 Get route
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
                vec.push((client.prepare_typed(sql, &[Type::TEXT]), sql));
                // 5 Get auth permissions
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
                vec.push((
                    client.prepare_typed(
                        sql,
                        &[Type::INT8, Type::INT8, Type::INT8, Type::INT8, Type::INT8, Type::INT8, Type::INT8],
                    ),
                    sql,
                ));
                // 6 Get not found
                let sql = "
                    SELECT url
                    FROM route
                    WHERE controller_id=3 AND lang_id=$1
                ";
                vec.push((client.prepare_typed(sql, &[Type::INT8]), sql));
                // 7 Insert email
                let sql = r#"
                    INSERT INTO mail(user_id, mail, "create", err, transport)
                    VALUES ($1, $2, now(), false, $3)
                    RETURNING mail_id;
                "#;
                vec.push((client.prepare_typed(sql, &[Type::INT8, Type::JSON, Type::TEXT]), sql));
                // 8 Insert email without provider
                let sql = r#"
                    INSERT INTO mail(user_id, mail, "create", send, err, transport)
                    VALUES ($1, $2, now(), now(), false, 'None')
                "#;
                vec.push((client.prepare_typed(sql, &[Type::INT8, Type::JSON]), sql));
                // 9 Insert email without provider
                let sql = r#"
                    SELECT data FROM setting WHERE key=$1
                "#;
                vec.push((client.prepare_typed(sql, &[Type::INT8]), sql));
                // 10 Insert error send email
                let sql = r#"
                    UPDATE mail
                    SET err=true, send=now(), err_text=$1
                    WHERE mail_id=$2
                "#;
                vec.push((client.prepare_typed(sql, &[Type::TEXT, Type::INT8]), sql));
                // 11 Insert success send email
                let sql = r#"
                    UPDATE mail
                    SET err=false, send=now()
                    WHERE mail_id=$1
                "#;
                vec.push((client.prepare_typed(sql, &[Type::INT8]), sql));
                // Prepare statements
                for (prepare, sql) in vec {
                    match prepare.await {
                        Ok(s) => {
                            self.prepare.push(DBStatement { statement: s, sql });
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

    /// Execute query to database with paramaters
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
    pub async fn query_params(&mut self, query: &str, params: &[&(dyn ToSql + Sync)]) -> Option<Vec<Row>> {
        match self.exec(query, params).await {
            DBResult::Ok(r) => Some(r),
            DBResult::ErrQuery(e) => {
                Log::stop(602, Some(format!("{} error={}", query, e)));
                None
            }
            DBResult::ErrConnect(e) => {
                Log::stop(603, Some(e));
                self.client = None;
                self.prepare.clear();
                None
            }
            DBResult::NoClient => {
                Log::stop(604, None);
                self.client = None;
                self.prepare.clear();
                None
            }
        }
    }

    /// Execute query to database and return a raw result
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
    pub async fn query_raw(&mut self, query: &str) -> Option<Vec<Row>> {
        match self.exec(query, &[]).await {
            DBResult::Ok(r) => Some(r),
            DBResult::ErrQuery(e) => {
                Log::stop(602, Some(format!("{} error={}", query, e)));
                None
            }
            DBResult::ErrConnect(e) => {
                Log::stop(603, Some(e));
                self.client = None;
                self.prepare.clear();
                None
            }
            DBResult::NoClient => {
                Log::stop(604, None);
                self.client = None;
                self.prepare.clear();
                None
            }
        }
    }

    /// Execute query to database
    ///
    /// # Parmeters
    ///
    /// * `query: &str` - SQL query;
    /// * `params: &[&(dyn ToSql + Sync)]` - Array of params.
    ///
    /// # Return
    ///
    /// * `Option::None` - When error query or diconnected;
    /// * `Option::Some(Vec<Data>)` - Results.
    pub async fn query(&mut self, query: &str, assoc: bool) -> Option<Vec<Data>> {
        // Run query
        let res = self.query_raw(query).await?;
        // Convert Row to Data
        let mut vec = Vec::with_capacity(res.len());
        if res.is_empty() {
            return Some(vec);
        };
        // Detect columns
        let cols = unsafe { res.get_unchecked(0) }.columns();
        let mut columns = Vec::with_capacity(cols.len());
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
                    Log::warning(614, Some(format!("Type={}. sql={}", u, query)));
                    Self::get_unknown
                }
            };
            columns.push((idx, col.name(), func));
        }
        // Read data
        for row in &res {
            if !assoc {
                let mut v = Vec::with_capacity(columns.len());
                for (idx, _, func) in &columns {
                    v.push(func(row, *idx));
                }
                vec.push(Data::Vec(v));
            } else {
                let mut t = BTreeMap::new();
                for (idx, name, func) in &columns {
                    t.insert(fnv1a_64(name), func(row, *idx));
                }
                vec.push(Data::Map(t));
            };
        }
        Some(vec)
    }

    /// Unknown Row type to Data::None
    fn get_unknown(_: &Row, _: usize) -> Data {
        Data::None
    }

    /// Row::i16 to Data::I16
    fn get_i16(row: &Row, idx: usize) -> Data {
        let i: Option<i16> = row.get(idx);
        match i {
            Some(i) => Data::I16(i),
            None => Data::None,
        }
    }

    /// Row::i32 to Data::I32
    fn get_i32(row: &Row, idx: usize) -> Data {
        let i: Option<i32> = row.get(idx);
        match i {
            Some(i) => Data::I32(i),
            None => Data::None,
        }
    }

    /// Row::f32 to Data::F32
    fn get_f32(row: &Row, idx: usize) -> Data {
        let f: Option<f32> = row.get(idx);
        match f {
            Some(f) => Data::F32(f),
            None => Data::None,
        }
    }

    /// Row::Uuid to Data::String
    fn get_uuid(row: &Row, idx: usize) -> Data {
        let u: Option<uuid::Uuid> = row.get(idx);
        match u {
            Some(u) => Data::String(u.to_string()),
            None => Data::None,
        }
    }

    /// Row::Json to Data::Json
    fn get_json(row: &Row, idx: usize) -> Data {
        let j: Option<Value> = row.get(idx);
        match j {
            Some(j) => Data::Json(j),
            None => Data::None,
        }
    }

    /// Row::DateTime<Utc> to Data::DateTime<Utc>
    fn get_date(row: &Row, idx: usize) -> Data {
        let d: Option<DateTime<Utc>> = row.get(idx);
        match d {
            Some(d) => Data::Date(d),
            None => Data::None,
        }
    }

    /// Row::f64 to Data::F64
    fn get_f64(row: &Row, idx: usize) -> Data {
        let f: Option<f64> = row.get(idx);
        match f {
            Some(f) => Data::F64(f),
            None => Data::None,
        }
    }

    /// Row::i64 to Data::I64
    fn get_i64(row: &Row, idx: usize) -> Data {
        let i: Option<i64> = row.get(idx);
        match i {
            Some(i) => Data::I64(i),
            None => Data::None,
        }
    }

    /// Row::String to Data::String
    fn get_string(row: &Row, idx: usize) -> Data {
        let s: Option<String> = row.get(idx);
        match s {
            Some(s) => Data::String(s),
            None => Data::None,
        }
    }

    /// Row::Vec<u8> to Data::Raw
    fn get_bytea(row: &Row, idx: usize) -> Data {
        let r: Option<Vec<u8>> = row.get(idx);
        match r {
            Some(r) => Data::Raw(r),
            None => Data::None,
        }
    }

    /// Row::Bool to Data::Bool
    fn get_bool(row: &Row, idx: usize) -> Data {
        let b: Option<bool> = row.get(idx);
        match b {
            Some(b) => Data::Bool(b),
            None => Data::None,
        }
    }

    /// Execute prepare query to database
    ///
    /// # Parmeters
    ///
    /// * `index: usize` - index of prepare query;
    /// * `params: &[&(dyn ToSql + Sync)]` - Array of params.
    ///
    /// # Return
    ///
    /// * `Option::None` - When error query or diconnected;
    /// * `Option::Some(Vec<Row>)` - Results.
    pub async fn query_fast(&mut self, index: usize, params: &[&(dyn ToSql + Sync)]) -> Option<Vec<Row>> {
        let statement = match self.prepare.get(index) {
            Some(s) => s.statement.clone(),
            None => return None,
        };
        match self.exec(&statement, params).await {
            DBResult::Ok(r) => Some(r),
            DBResult::ErrQuery(e) => {
                Log::stop(602, Some(format!("Statement={} error={}", index, e)));
                None
            }
            DBResult::ErrConnect(e) => {
                Log::stop(603, Some(e));
                self.client = None;
                self.prepare.clear();
                None
            }
            DBResult::NoClient => {
                Log::stop(604, None);
                self.client = None;
                self.prepare.clear();
                None
            }
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
    async fn exec<T>(&mut self, query: &T, params: &[&(dyn ToSql + Sync)]) -> DBResult
    where
        T: ?Sized + ToStatement,
    {
        match self.client.as_mut() {
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
}

impl std::fmt::Debug for DB {
    /// Formats the value using the given formatter.
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        let tls = self.tls.clone().map(|_| "TlsConnector");
        let DB {
            client,
            sql_conn,
            tls: _,
            zone,
            prepare,
        } = self;
        f.debug_struct("DB")
            .field("client", &client)
            .field("sql_conn", &sql_conn)
            .field("tls", &tls)
            .field("zone", &zone)
            .field("prepare", &prepare)
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
