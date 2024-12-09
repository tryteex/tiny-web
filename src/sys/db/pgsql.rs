use std::{
    future::Future,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};

#[cfg(any(
    feature = "row-data",
    feature = "session-db",
    feature = "redirect-db",
    feature = "route-db",
    feature = "access-db",
    feature = "setting-db",
    feature = "mail-db"
))]
use std::collections::HashMap;

use futures_util::{pin_mut, TryStreamExt};

use postgres::{
    tls::{ChannelBinding, MakeTlsConnect, TlsConnect},
    types::ToSql,
    Error, NoTls, Row, ToStatement,
};

use ring::digest;

use rustls::{
    client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier},
    pki_types::{CertificateDer, ServerName, UnixTime},
    ClientConfig, DigitallySignedStruct, RootCertStore, SignatureScheme,
};

use tokio::{
    io::{AsyncRead, AsyncWrite, ReadBuf},
    sync::{MutexGuard, SemaphorePermit},
};

use tokio_postgres::{Client, RowStream};

use tokio_rustls::{client::TlsStream, TlsConnector};

use x509_certificate::{DigestAlgorithm, SignatureAlgorithm, X509Certificate};

#[cfg(feature = "row-data")]
use chrono::{DateTime, Utc};

#[cfg(feature = "row-data")]
use postgres::Column;

#[cfg(feature = "row-data")]
use serde_json::Value;

#[cfg(any(
    feature = "row-data",
    feature = "session-db",
    feature = "redirect-db",
    feature = "route-db",
    feature = "access-db",
    feature = "setting-db",
    feature = "mail-db"
))]
use postgres::types::Type;

#[cfg(any(
    feature = "session-db",
    feature = "redirect-db",
    feature = "route-db",
    feature = "access-db",
    feature = "setting-db",
    feature = "mail-db"
))]
use postgres::Statement;

#[cfg(any(
    feature = "session-db",
    feature = "redirect-db",
    feature = "route-db",
    feature = "access-db",
    feature = "setting-db",
    feature = "mail-db",
))]
use tiny_web_macro::fnv1a_64;

use crate::{log, sys::app::init::DBConfig};

#[cfg(feature = "row-data")]
use crate::sys::web::data::Data;

#[cfg(feature = "row-data")]
pub type DataRow = Data;

#[cfg(feature = "row-native")]
pub type DataRow = Row;

pub type QueryParam<'a> = &'a [&'a (dyn ToSql + Sync)];

#[cfg(feature = "row-data")]
type PgColumnName = (usize, fn(&Row, usize) -> Data);
#[cfg(feature = "row-data")]
type PgColumnNum = fn(&Row, usize) -> Data;

/// Response to the result of the query
enum DBResult {
    /// The request was completed successfully.
    Vec(Vec<Row>),
    /// Stream
    Stream(RowStream),
    /// The request was completed successfully without result.
    Void,
    /// Connection is empty.
    NoClient,
    /// Query execution error.
    ErrQuery(String),
    /// Connection is lost.
    ErrConnect(String),
}

/// Responsible for working with postgresql database
#[derive(Debug)]
pub(crate) struct PgSql {
    /// Client for connection to database.
    client: Option<Client>,
    /// Connection config.
    sql_conn: tokio_postgres::Config,
    /// Use tls for connection when sslmode=require.
    tls: Option<MakeRustlsConnect>,
    /// Prepare statements to database.
    #[cfg(any(
        feature = "session-db",
        feature = "redirect-db",
        feature = "route-db",
        feature = "access-db",
        feature = "setting-db",
        feature = "mail-db"
    ))]
    prepare: HashMap<i64, Statement>,
}

impl PgSql {
    /// Initializes a new object `PgSql`
    pub fn new(config: Arc<DBConfig>) -> Option<PgSql> {
        let (sql_conn, tls) = match PgSql::create_connect_string(&config) {
            Ok(v) => v,
            Err(_e) => {
                log!(stop, 0, "{}", _e);
                return None;
            }
        };

        Some(PgSql {
            client: None,
            sql_conn,
            tls,
            #[cfg(any(
                feature = "session-db",
                feature = "redirect-db",
                feature = "route-db",
                feature = "access-db",
                feature = "setting-db",
                feature = "mail-db",
            ))]
            prepare: HashMap::new(),
        })
    }

    fn create_connect_string(config: &DBConfig) -> Result<(tokio_postgres::Config, Option<MakeRustlsConnect>), Error> {
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
        if config.ssl {
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
            Err(e) => return Err(e),
        };
        let tls = if config.ssl {
            let mut config = ClientConfig::builder().with_root_certificates(RootCertStore::empty()).with_no_client_auth();
            config.dangerous().set_certificate_verifier(Arc::new(NoCertificateVerification {}));
            Some(MakeRustlsConnect::new(config))
        } else {
            None
        };

        Ok((sql_conn, tls))
    }

    /// Connect to the database
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
    async fn try_connect(&mut self) -> bool {
        match self.tls.clone() {
            Some(tls) => match self.sql_conn.connect(tls).await {
                Ok((client, connection)) => {
                    tokio::spawn(async move {
                        if let Err(_e) = connection.await {
                            log!(stop, 0, "{}", _e);
                        }
                    });
                    self.client = Some(client);
                }
                Err(_e) => {
                    log!(warning, 0, "Error: {} => {:?}", _e, &self.sql_conn);
                    return false;
                }
            },
            None => match self.sql_conn.connect(NoTls).await {
                Ok((client, connection)) => {
                    tokio::spawn(async move {
                        if let Err(_e) = connection.await {
                            log!(warning, 0, "{}", _e);
                        }
                    });
                    self.client = Some(client);
                }
                Err(_e) => {
                    log!(warning, 0, "Error: {} => {:?}", _e, &self.sql_conn);
                    return false;
                }
            },
        }
        #[cfg(any(
            feature = "session-db",
            feature = "redirect-db",
            feature = "route-db",
            feature = "access-db",
            feature = "setting-db",
            feature = "mail-db"
        ))]
        {
            self.prepare().await
        }
        #[cfg(not(any(
            feature = "session-db",
            feature = "redirect-db",
            feature = "route-db",
            feature = "access-db",
            feature = "setting-db",
            feature = "mail-db"
        )))]
        true
    }

    /// Prepare sql statement
    #[cfg(any(
        feature = "session-db",
        feature = "redirect-db",
        feature = "route-db",
        feature = "access-db",
        feature = "setting-db",
        feature = "mail-db"
    ))]
    async fn prepare(&mut self) -> bool {
        self.prepare.clear();
        match &self.client {
            Some(client) => {
                let mut map = HashMap::new();

                // Get avaible lang 4156762777733340057
                #[cfg(all(feature = "session-db", any(feature = "lang-static", feature = "lang-reload")))]
                {
                    let sql = r#"
                        SELECT lang_id, code, name, index
                        FROM lang
                        WHERE enable
                        ORDER BY sort
                    "#;
                    map.insert(fnv1a_64!("lib_get_langs"), (client.prepare_typed(sql, &[]), sql.to_owned()));
                }

                // Get all lang 3367482389811013093
                #[cfg(all(feature = "session-db", any(feature = "lang-static", feature = "lang-reload")))]
                {
                    let sql = r#"
                        SELECT lang_id, code, name, index
                        FROM lang
                        ORDER BY index, sort
                    "#;
                    map.insert(fnv1a_64!("lib_get_all_langs"), (client.prepare_typed(sql, &[]), sql.to_owned()));
                }

                // Get session 6716397077443474616
                #[cfg(feature = "session-db")]
                {
                    let sql = r#"
                        WITH upd AS (
                            UPDATE session
                            SET 
                                last = now()
                            WHERE
                                session_key=$1
                            RETURNING session_id, user_id, data, lang_id
                        )
                        SELECT 
                            s.user_id, u.role_id, s.data, s.lang_id 
                        FROM 
                            upd s
                            INNER JOIN "user" u ON u.user_id=s.user_id
                    "#;
                    map.insert(fnv1a_64!("lib_get_session"), (client.prepare_typed(sql, &[Type::INT4]), sql.to_owned()));
                }

                // Update session -400086351751991892
                #[cfg(feature = "session-db")]
                {
                    let sql = r#"
                        UPDATE session
                        SET 
                            user_id=$1,
                            lang_id=$2,
                            data=$3,
                            last=now()
                        WHERE
                            session_key=$4
                    "#;
                    map.insert(
                        fnv1a_64!("lib_set_session"),
                        (client.prepare_typed(sql, &[Type::INT8, Type::INT8, Type::BYTEA, Type::INT8]), sql.to_owned()),
                    );
                }

                // Insert session 8029853374838241583
                #[cfg(feature = "session-db")]
                {
                    let sql = r#"
                        INSERT INTO session (session, session_key, user_id, lang_id, data, created, last)
                        SELECT $1, $2, $3, $4, $5, now(), now()
                    "#;
                    map.insert(
                        fnv1a_64!("lib_add_session"),
                        (client.prepare_typed(sql, &[Type::TEXT, Type::INT8, Type::INT8, Type::INT8, Type::BYTEA]), sql.to_owned()),
                    );
                }

                // Get redirect -1566077906756142556
                #[cfg(feature = "redirect-db")]
                {
                    let sql = r#"
                        SELECT redirect, permanently FROM redirect WHERE url=$1
                    "#;
                    map.insert(fnv1a_64!("lib_get_redirect"), (client.prepare_typed(sql, &[Type::TEXT]), sql.to_owned()));
                }

                // Get route 3077841024002823969
                #[cfg(feature = "route-db")]
                {
                    let sql = r#"
                        SELECT 
                            c.module_id, c.class_id, c.action_id,
                            r.params, r.lang_id
                        FROM 
                            route r
                            INNER JOIN controller c ON c.controller_id=r.controller_id
                        WHERE r.url=$1
                    "#;
                    map.insert(fnv1a_64!("lib_get_route"), (client.prepare_typed(sql, &[Type::TEXT]), sql.to_owned()));
                }

                // Get route from module/class/action 8508883211214576597
                #[cfg(feature = "route-db")]
                {
                    let sql = r#"
                        SELECT r.url 
                        FROM 
                            controller c
                            INNER JOIN route r ON 
                                r.controller_id=c.controller_id AND (r.lang_id=$5 OR r.lang_id IS NULL) AND r.params = $4
                        WHERE 
                            c.module_id=$1 AND c.class_id=$2 AND c.action_id=$3
                        ORDER BY 
                            CASE WHEN r.lang_id IS NOT NULL THEN 0 ELSE 1 END
                        LIMIT 1
                    "#;
                    map.insert(
                        fnv1a_64!("lib_get_url"),
                        (client.prepare_typed(sql, &[Type::INT8, Type::INT8, Type::INT8, Type::TEXT, Type::INT8]), sql.to_owned()),
                    );
                }

                // Get auth permissions -4169186416014187350
                #[cfg(feature = "access-db")]
                {
                    let sql = r#"
                        SELECT COALESCE(MAX(a.access), false) AS access
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
                            client
                                .prepare_typed(sql, &[Type::INT8, Type::INT8, Type::INT8, Type::INT8, Type::INT8, Type::INT8, Type::INT8]),
                            sql.to_owned(),
                        ),
                    );
                }

                // Get setting 2305043036426846632
                #[cfg(feature = "setting-db")]
                {
                    let sql = r#"
                        SELECT data FROM setting WHERE key=$1
                    "#;
                    map.insert(fnv1a_64!("lib_get_setting"), (client.prepare_typed(sql, &[Type::INT8]), sql.to_owned()));
                }

                // Insert email 5843182919945045895
                #[cfg(feature = "mail-db")]
                {
                    let sql = r#"
                        INSERT INTO mail(user_id, mail, "create")
                        VALUES ($1, $2, now())
                    "#;
                    map.insert(fnv1a_64!("lib_mail_add"), (client.prepare_typed(sql, &[Type::INT8, Type::JSON]), sql.to_owned()));
                }

                // Prepare statements
                for (key, (prepare, _sql)) in map {
                    match prepare.await {
                        Ok(s) => {
                            self.prepare.insert(key, s);
                        }
                        Err(_e) => {
                            log!(stop, 0, "Error={}. sql={}", _e, _sql);
                            return false;
                        }
                    }
                }
                true
            }
            None => false,
        }
    }

    /// Executes a statement in database, returning the results
    async fn query_raw<T>(client: &Option<tokio_postgres::Client>, query: &T, params: &[&(dyn ToSql + Sync)]) -> DBResult
    where
        T: ?Sized + ToStatement,
    {
        match client {
            Some(sql) => match sql.query_raw(query, PgSql::slice_iter(params)).await {
                Ok(res) => {
                    pin_mut!(res);
                    match res.try_next().await {
                        Ok(row) => match row {
                            Some(r) => {
                                let mut result = match res.rows_affected() {
                                    Some(s) => Vec::with_capacity(s as usize),
                                    None => Vec::new(),
                                };
                                result.push(r);
                                loop {
                                    match res.try_next().await {
                                        Ok(row) => match row {
                                            Some(r) => {
                                                result.push(r);
                                            }
                                            None => break,
                                        },
                                        Err(e) => return DBResult::ErrQuery(e.to_string()),
                                    }
                                }
                                DBResult::Vec(result)
                            }
                            None => DBResult::Void,
                        },
                        Err(e) => DBResult::ErrQuery(e.to_string()),
                    }
                }
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

    async fn query_stream_raw<T>(client: &Option<tokio_postgres::Client>, query: &T, params: &[&(dyn ToSql + Sync)]) -> DBResult
    where
        T: ?Sized + ToStatement,
    {
        match client {
            Some(sql) => match sql.query_raw(query, PgSql::slice_iter(params)).await {
                Ok(res) => DBResult::Stream(res),
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

    /// Executes a statement in database, without results
    async fn execute_raw<T>(client: &Option<tokio_postgres::Client>, query: &T, params: &[&(dyn ToSql + Sync)]) -> DBResult
    where
        T: ?Sized + ToStatement,
    {
        match client {
            Some(sql) => match sql.execute_raw(query, PgSql::slice_iter(params)).await {
                Ok(_) => DBResult::Void,
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

    /// Slise to ToSql
    fn slice_iter<'a>(s: &'a [&'a (dyn ToSql + Sync)]) -> impl ExactSizeIterator<Item = &'a dyn ToSql> + 'a {
        s.iter().map(|s| *s as _)
    }

    /// Execute query to database without a result
    pub async fn execute(&mut self, query: &str, params: QueryParam<'_>) -> Option<()> {
        match PgSql::execute_raw(&self.client, query, params).await {
            DBResult::Void | DBResult::Vec(_) => return Some(()),
            DBResult::ErrQuery(_e) => {
                log!(warning, 0, "{} error={}", query, _e);
                return None;
            }
            DBResult::NoClient => log!(warning, 0),
            DBResult::ErrConnect(_e) => log!(warning, 0, "{}", _e),
            DBResult::Stream(_) => log!(warning, 0),
        };
        self.client = None;
        if self.try_connect().await {
            match PgSql::execute_raw(&self.client, query, params).await {
                DBResult::Void | DBResult::Vec(_) => return Some(()),
                _ => {}
            }
        }
        None
    }

    #[cfg(any(feature = "session-db", feature = "mail-db"))]
    pub async fn execute_prepare(&mut self, query: i64, params: QueryParam<'_>) -> Option<()> {
        let stat = match self.prepare.get(&query) {
            Some(s) => s,
            None => {
                log!(warning, 0, "{:?}", query);
                return None;
            }
        };

        match PgSql::execute_raw(&self.client, stat, params).await {
            DBResult::Void | DBResult::Vec(_) | DBResult::Stream(_) => return Some(()),
            DBResult::ErrQuery(_e) => {
                #[cfg(any(feature = "debug-v", feature = "debug-vv", feature = "debug-vvv"))]
                log!(warning, 0, "Statement key={} error={}", query, _e);
                return None;
            }
            DBResult::NoClient => log!(warning, 0),
            DBResult::ErrConnect(_e) => log!(warning, 0, "{}", _e),
        };
        self.client = None;
        if self.try_connect().await {
            let stat = match self.prepare.get(&query) {
                Some(s) => s,
                None => {
                    log!(warning, 0, "{:?}", query);
                    return None;
                }
            };
            match PgSql::execute_raw(&self.client, stat, params).await {
                DBResult::Void | DBResult::Vec(_) => return Some(()),
                _ => {}
            }
        }
        None
    }

    /// Execute query to database and return a result
    #[cfg(feature = "row-data")]
    pub async fn query(&mut self, query: &str, params: QueryParam<'_>, assoc: bool) -> Option<Vec<DataRow>> {
        match PgSql::query_raw(&self.client, query, params).await {
            DBResult::Vec(rows) => return Some(self.convert(rows, assoc)),
            DBResult::Void => return Some(Vec::new()),
            DBResult::ErrQuery(_e) => {
                log!(warning, 0, "{} error={}", query, _e);
                return None;
            }
            DBResult::NoClient => log!(warning, 0),
            DBResult::ErrConnect(_e) => log!(warning, 0, "{}", _e),
            DBResult::Stream(_) => log!(warning, 0),
        };
        self.client = None;
        if self.try_connect().await {
            match PgSql::query_raw(&self.client, query, params).await {
                DBResult::Vec(rows) => return Some(self.convert(rows, assoc)),
                DBResult::Void => return Some(Vec::new()),
                _ => {}
            }
        }
        None
    }

    /// Execute query to database and return a result
    #[cfg(feature = "row-native")]
    pub async fn query(&mut self, query: &str, params: QueryParam<'_>) -> Option<Vec<DataRow>> {
        match PgSql::query_raw(&self.client, query, params).await {
            DBResult::Vec(rows) => return Some(rows),
            DBResult::Void => return Some(Vec::new()),
            DBResult::ErrQuery(_e) => {
                log!(warning, 0, "{} error={}", query, _e);
                return None;
            }
            DBResult::NoClient => log!(warning, 0),
            DBResult::ErrConnect(_e) => log!(warning, 0, "{}", _e),
            DBResult::Stream(_) => log!(warning, 0),
        };
        self.client = None;
        if self.try_connect().await {
            match PgSql::query_raw(&self.client, query, params).await {
                DBResult::Vec(rows) => return Some(rows),
                DBResult::Void => return Some(Vec::new()),
                _ => {}
            }
        }
        None
    }

    pub async fn query_stream(&mut self, query: &str, params: QueryParam<'_>) -> Option<RowStream> {
        match PgSql::query_stream_raw(&self.client, query, params).await {
            DBResult::Stream(stream) => return Some(stream),
            DBResult::Vec(_) => return None,
            DBResult::Void => return None,
            DBResult::NoClient => log!(warning, 0),
            DBResult::ErrConnect(_e) => log!(warning, 0, "{}", _e),
            DBResult::ErrQuery(_e) => {
                log!(warning, 0, "{} error={}", query, _e);
                return None;
            }
        }
        self.client = None;
        if self.try_connect().await {
            if let DBResult::Stream(stream) = PgSql::query_stream_raw(&self.client, query, params).await {
                return Some(stream);
            }
        }
        None
    }

    /// Execute prepare query to database and return a result
    #[cfg(any(
        feature = "session-db",
        feature = "redirect-db",
        feature = "route-db",
        feature = "access-db",
        feature = "setting-db",
    ))]
    pub(crate) async fn query_prepare(&mut self, query: i64, params: QueryParam<'_>) -> Option<Vec<Row>> {
        let stat = match self.prepare.get(&query) {
            Some(s) => s,
            None => {
                log!(warning, 0, "{:?}", query);
                return None;
            }
        };
        match PgSql::query_raw(&self.client, stat, params).await {
            DBResult::Vec(rows) => return Some(rows),
            DBResult::Void | DBResult::Stream(_) => return Some(Vec::new()),
            DBResult::ErrQuery(_e) => {
                #[cfg(any(feature = "debug-v", feature = "debug-vv", feature = "debug-vvv"))]
                log!(warning, 0, "Statement key={} error={}", query, _e);
                return None;
            }
            DBResult::NoClient => log!(warning, 0),
            DBResult::ErrConnect(_e) => log!(warning, 0, "{}", _e),
        };
        self.client = None;
        if self.try_connect().await {
            let stat = match self.prepare.get(&query) {
                Some(s) => s,
                None => {
                    log!(warning, 0, "{:?}", query);
                    return None;
                }
            };
            match PgSql::query_raw(&self.client, stat, params).await {
                DBResult::Vec(rows) => return Some(rows),
                DBResult::Void => return Some(Vec::new()),
                _ => {}
            }
        }
        None
    }

    /// Convert Vec<Row> to Vec<Data>
    #[cfg(feature = "row-data")]
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
                let mut t = HashMap::with_capacity(columns.len());
                for (name, turple) in &columns {
                    t.insert(*name, turple.1(row, turple.0));
                }
                vec.push(Data::Map(t));
            }
        }
        vec
    }

    /// Detect columns' type with columns' name
    #[cfg(feature = "row-data")]
    fn get_column_type(&self, cols: &[Column]) -> Vec<PgColumnNum> {
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
                _u => {
                    log!(warning, 0, "Type: {}", _u);
                    Self::get_unknown
                }
            };
            columns.push(func);
        }
        columns
    }

    /// Detect columns' type with columns' name
    #[cfg(feature = "row-data")]
    fn get_column_type_name(&self, cols: &[Column]) -> HashMap<i64, PgColumnName> {
        let mut columns = HashMap::with_capacity(cols.len());
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
                _u => {
                    log!(warning, 0, "Type: {}", _u);
                    Self::get_unknown
                }
            };
            columns.insert(crate::fnv1a_64(col.name().as_bytes()), (idx, func));
        }
        columns
    }

    /// Unknown Row type to Data::None
    #[cfg(feature = "row-data")]
    #[inline]
    fn get_unknown(_: &Row, _: usize) -> Data {
        Data::None
    }

    /// Row::i16 to Data::I16
    #[cfg(feature = "row-data")]
    #[inline]
    fn get_i16(row: &Row, idx: usize) -> Data {
        let i: Option<i16> = row.get(idx);
        match i {
            Some(i) => Data::I16(i),
            None => Data::None,
        }
    }

    /// Row::i32 to Data::I32
    #[cfg(feature = "row-data")]
    #[inline]
    fn get_i32(row: &Row, idx: usize) -> Data {
        let i: Option<i32> = row.get(idx);
        match i {
            Some(i) => Data::I32(i),
            None => Data::None,
        }
    }

    /// Row::i64 to Data::I64
    #[cfg(feature = "row-data")]
    #[inline]
    fn get_i64(row: &Row, idx: usize) -> Data {
        let i: Option<i64> = row.get(idx);
        match i {
            Some(i) => Data::I64(i),
            None => Data::None,
        }
    }

    /// Row::f32 to Data::F32
    #[cfg(feature = "row-data")]
    #[inline]
    fn get_f32(row: &Row, idx: usize) -> Data {
        let f: Option<f32> = row.get(idx);
        match f {
            Some(f) => Data::F32(f),
            None => Data::None,
        }
    }

    /// Row::f64 to Data::F64
    #[cfg(feature = "row-data")]
    #[inline]
    fn get_f64(row: &Row, idx: usize) -> Data {
        let f: Option<f64> = row.get(idx);
        match f {
            Some(f) => Data::F64(f),
            None => Data::None,
        }
    }

    /// Row::DateTime<Utc> to Data::DateTime<Utc>
    #[cfg(feature = "row-data")]
    #[inline]
    fn get_date(row: &Row, idx: usize) -> Data {
        let d: Option<DateTime<Utc>> = row.get(idx);
        match d {
            Some(d) => Data::Date(d),
            None => Data::None,
        }
    }

    /// Row::Uuid to Data::String
    #[cfg(feature = "row-data")]
    #[inline]
    fn get_uuid(row: &Row, idx: usize) -> Data {
        let u: Option<uuid::Uuid> = row.get(idx);
        match u {
            Some(u) => Data::String(u.to_string()),
            None => Data::None,
        }
    }

    /// Row::Json to Data::Json
    #[cfg(feature = "row-data")]
    #[inline]
    fn get_json(row: &Row, idx: usize) -> Data {
        let j: Option<Value> = row.get(idx);
        match j {
            Some(j) => Data::Json(j),
            None => Data::None,
        }
    }

    /// Row::String to Data::String
    #[cfg(feature = "row-data")]
    #[inline]
    fn get_string(row: &Row, idx: usize) -> Data {
        let s: Option<String> = row.get(idx);
        match s {
            Some(s) => Data::String(s),
            None => Data::None,
        }
    }

    /// Row::Vec<u8> to Data::Raw
    #[cfg(feature = "row-data")]
    #[inline]
    fn get_bytea(row: &Row, idx: usize) -> Data {
        let r: Option<Vec<u8>> = row.get(idx);
        match r {
            Some(r) => Data::Raw(r),
            None => Data::None,
        }
    }

    /// Row::Bool to Data::Bool
    #[cfg(feature = "row-data")]
    #[inline]
    fn get_bool(row: &Row, idx: usize) -> Data {
        let b: Option<bool> = row.get(idx);
        match b {
            Some(b) => Data::Bool(b),
            None => Data::None,
        }
    }
}

#[cfg(feature = "row-data")]
pub(crate) enum PgColumn {
    Vec(Option<Vec<PgColumnNum>>),
    Map(Option<HashMap<i64, PgColumnName>>),
}

pub struct QueryStream<'a> {
    pub(crate) permit: SemaphorePermit<'a>,
    pub(crate) db: MutexGuard<'a, PgSql>,
    pub(crate) stream: Pin<Box<RowStream>>,
    #[cfg(any(feature = "debug-v", feature = "debug-vv", feature = "debug-vvv"))]
    pub(crate) sql: &'a str,
    #[cfg(feature = "row-data")]
    pub(crate) cols: PgColumn,
}

impl<'a> QueryStream<'a> {
    #[cfg(feature = "row-data")]
    pub async fn next(&mut self) -> Option<Data> {
        match self.stream.try_next().await {
            Ok(row) => Some(self.convert(row?)),
            Err(_e) => {
                log!(warning, 0, "{} error={}", self.sql, _e);
                None
            }
        }
    }

    #[cfg(feature = "row-native")]
    pub async fn next(&mut self) -> Option<Row> {
        match self.stream.try_next().await {
            Ok(row) => row,
            Err(_e) => {
                log!(warning, 0, "{} error={}", self.sql, _e);
                None
            }
        }
    }

    #[cfg(feature = "row-data")]
    fn convert(&mut self, row: Row) -> Data {
        match self.cols {
            PgColumn::Vec(ref mut vec) => {
                let cols = vec.get_or_insert_with(|| self.db.get_column_type(row.columns()));
                let mut v = Vec::with_capacity(cols.len());
                let mut func;
                for idx in 0..cols.len() {
                    func = unsafe { cols.get_unchecked(idx) };
                    v.push(func(&row, idx))
                }
                Data::Vec(v)
            }
            PgColumn::Map(ref mut map) => {
                let cols = map.get_or_insert_with(|| self.db.get_column_type_name(row.columns()));
                let mut t = HashMap::with_capacity(cols.len());
                for (name, turple) in cols {
                    t.insert(*name, turple.1(&row, turple.0));
                }
                Data::Map(t)
            }
        }
    }
}

impl Drop for QueryStream<'_> {
    fn drop(&mut self) {
        let _ = &mut self.stream;
        let _ = &mut self.db;
        let _ = &mut self.permit;
    }
}

#[derive(Debug, Clone)]
pub(crate) struct MakeRustlsConnect {
    config: Arc<ClientConfig>,
}

impl MakeRustlsConnect {
    pub fn new(config: ClientConfig) -> Self {
        Self { config: Arc::new(config) }
    }
}

impl<S> MakeTlsConnect<S> for MakeRustlsConnect
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    type Stream = RustlsStream<S>;
    type TlsConnect = RustlsConnect;
    type Error = rustls::pki_types::InvalidDnsNameError;

    fn make_tls_connect(&mut self, hostname: &str) -> Result<RustlsConnect, Self::Error> {
        ServerName::try_from(hostname).map(|dns_name| {
            RustlsConnect(RustlsConnectData {
                hostname: dns_name.to_owned(),
                connector: Arc::clone(&self.config).into(),
            })
        })
    }
}

pub(crate) struct RustlsConnect(RustlsConnectData);

struct RustlsConnectData {
    hostname: ServerName<'static>,
    connector: TlsConnector,
}

impl<S> TlsConnect<S> for RustlsConnect
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    type Stream = RustlsStream<S>;
    type Error = std::io::Error;
    type Future = Pin<Box<dyn Future<Output = std::io::Result<RustlsStream<S>>> + Send>>;

    fn connect(self, stream: S) -> Self::Future {
        Box::pin(async move { self.0.connector.connect(self.0.hostname, stream).await.map(|s| RustlsStream(Box::pin(s))) })
    }
}

pub(crate) struct RustlsStream<S>(Pin<Box<TlsStream<S>>>);

impl<S> tokio_postgres::tls::TlsStream for RustlsStream<S>
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
                    SignatureAlgorithm::RsaSha1 | SignatureAlgorithm::RsaSha256 | SignatureAlgorithm::EcdsaSha256 => &digest::SHA256,
                    SignatureAlgorithm::RsaSha384 | SignatureAlgorithm::EcdsaSha384 => &digest::SHA384,
                    SignatureAlgorithm::RsaSha512 | SignatureAlgorithm::Ed25519 => &digest::SHA512,
                    SignatureAlgorithm::NoSignature(algo) => match algo {
                        DigestAlgorithm::Sha1 | DigestAlgorithm::Sha256 => &digest::SHA256,
                        DigestAlgorithm::Sha384 => &digest::SHA384,
                        DigestAlgorithm::Sha512 => &digest::SHA512,
                    },
                })
                .map(|algorithm| {
                    let hash = digest::digest(algorithm, certs[0].as_ref());
                    ChannelBinding::tls_server_end_point(hash.as_ref().into())
                })
                .unwrap_or_else(ChannelBinding::none),
            _ => ChannelBinding::none(),
        }
    }
}

impl<S> AsyncRead for RustlsStream<S>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    fn poll_read(mut self: Pin<&mut Self>, cx: &mut Context, buf: &mut ReadBuf<'_>) -> Poll<tokio::io::Result<()>> {
        self.0.as_mut().poll_read(cx, buf)
    }
}

impl<S> AsyncWrite for RustlsStream<S>
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

#[derive(Debug)]
pub struct NoCertificateVerification;

impl ServerCertVerifier for NoCertificateVerification {
    fn verify_server_cert(
        &self,
        _: &CertificateDer<'_>,
        _: &[CertificateDer<'_>],
        _: &ServerName<'_>,
        _: &[u8],
        _: UnixTime,
    ) -> Result<ServerCertVerified, rustls::Error> {
        Ok(ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _: &[u8],
        _: &CertificateDer<'_>,
        _: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        Ok(HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _: &[u8],
        _: &CertificateDer<'_>,
        _: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        Ok(HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        vec![
            SignatureScheme::RSA_PKCS1_SHA1,
            SignatureScheme::ECDSA_SHA1_Legacy,
            SignatureScheme::RSA_PKCS1_SHA256,
            SignatureScheme::ECDSA_NISTP256_SHA256,
            SignatureScheme::RSA_PKCS1_SHA384,
            SignatureScheme::ECDSA_NISTP384_SHA384,
            SignatureScheme::RSA_PKCS1_SHA512,
            SignatureScheme::ECDSA_NISTP521_SHA512,
            SignatureScheme::RSA_PSS_SHA256,
            SignatureScheme::RSA_PSS_SHA384,
            SignatureScheme::RSA_PSS_SHA512,
            SignatureScheme::ED25519,
            SignatureScheme::ED448,
        ]
    }
}
