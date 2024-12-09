use std::{borrow::Cow, sync::Arc, time::Duration};

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

use futures_util::TryStreamExt;

use tiberius::{error::Error, AuthMethod, Client, Config, EncryptionLevel, QueryItem, Row, ToSql};

use tokio::{net::TcpStream, time::timeout};

use tokio_util::compat::{Compat, TokioAsyncWriteCompatExt};

#[cfg(any(
    feature = "session-db",
    feature = "redirect-db",
    feature = "route-db",
    feature = "access-db",
    feature = "setting-db",
    feature = "mail-db"
))]
use tiny_web_macro::fnv1a_64;

#[cfg(feature = "row-data")]
use chrono::{DateTime, Utc};

#[cfg(feature = "row-data")]
use tiberius::Column;

use crate::{log, sys::app::init::DBConfig};

#[cfg(feature = "row-data")]
use crate::sys::web::data::Data;

#[cfg(feature = "row-data")]
pub type DataRow = Data;

#[cfg(feature = "row-native")]
pub type DataRow = Row;

pub type QueryParam<'a> = &'a [&'a dyn ToSql];

#[cfg(feature = "row-data")]
type MsColumnName = (usize, fn(&Row, usize) -> Data);

/// Response to the result of the query
enum DBResult {
    /// The request was completed successfully.
    Vec(Vec<Row>),
    /// Stream
    // Stream(RowStream<'a>),
    /// The request was completed successfully without result.
    Void,
    /// Connection is empty.
    NoClient,
    /// Query execution error.
    ErrQuery(String),
    /// Connection is lost.
    ErrConnect(String),
}

/// Responsible for working with MsSql database
#[derive(Debug)]
pub(crate) struct MsSql {
    config: Config,
    client: Option<Client<Compat<TcpStream>>>,
    #[cfg(any(
        feature = "session-db",
        feature = "redirect-db",
        feature = "route-db",
        feature = "access-db",
        feature = "setting-db",
        feature = "mail-db"
    ))]
    prepare: HashMap<i64, i64>,
}

impl MsSql {
    /// Initializes a new object `MsSql`
    pub fn new(config: Arc<DBConfig>) -> Option<MsSql> {
        Some(MsSql {
            config: MsSql::create_config(&config),
            client: None,
            #[cfg(any(
                feature = "session-db",
                feature = "redirect-db",
                feature = "route-db",
                feature = "access-db",
                feature = "setting-db",
                feature = "mail-db"
            ))]
            prepare: HashMap::new(),
        })
    }

    fn create_config(config: &DBConfig) -> Config {
        let mut cfg = Config::new();
        cfg.host(&config.host);
        if let Some(p) = &config.port {
            cfg.port(*p);
        }
        cfg.database(&config.name);
        let user = if let Some(u) = &config.user { u.to_owned() } else { "SA".to_owned() };
        if let Some(p) = &config.pwd {
            cfg.authentication(AuthMethod::sql_server(user, p));
        } else {
            cfg.authentication(AuthMethod::aad_token(user));
        }
        let app = format!("{} {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
        cfg.application_name(app);
        cfg.trust_cert();
        if config.ssl {
            cfg.encryption(EncryptionLevel::Required);
        } else {
            cfg.encryption(EncryptionLevel::NotSupported);
        }
        cfg
    }

    /// Connect to the database
    pub async fn connect(&mut self) -> bool {
        match &self.client {
            Some(_) => true,
            None => self.try_connect().await,
        }
    }

    /// Trying to connect to the database
    async fn try_connect(&mut self) -> bool {
        let tcp = match timeout(Duration::from_secs(1), TcpStream::connect(self.config.get_addr())).await {
            Ok(Ok(tcp)) => tcp,
            Ok(Err(_e)) => {
                log!(stop, 0, "{}", _e);
                return false;
            }
            Err(_e) => {
                log!(stop, 0, "{}", _e);
                return false;
            }
        };
        if let Err(_e) = tcp.set_nodelay(true) {
            log!(stop, 0, "{}", _e);
            return false;
        };
        let client = match Client::connect(self.config.clone(), tcp.compat_write()).await {
            Ok(client) => client,
            Err(_e) => {
                log!(stop, 0, "{}", _e);
                return false;
            }
        };
        self.client = Some(client);

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
        match self.client.as_mut() {
            Some(client) => {
                let mut map = HashMap::new();
                // Get avaible lang 4156762777733340057
                #[cfg(all(feature = "session-db", any(feature = "lang-static", feature = "lang-reload")))]
                {
                    let sql = r#"
                        SELECT [lang_id], [code], [name], [index]
                        FROM [lang]
                        WHERE [enable]=1
                        ORDER BY [sort]
                    "#;
                    map.insert(fnv1a_64!("lib_get_langs"), (String::new(), sql.to_owned()));
                }
                #[cfg(all(feature = "session-db", any(feature = "lang-static", feature = "lang-reload")))]
                {
                    // Get all lang 3367482389811013093
                    let sql = r#"
                        SELECT [lang_id], [code], [name], [index]
                        FROM [lang]
                        ORDER BY [index], sort]
                    "#;
                    map.insert(fnv1a_64!("lib_get_all_langs"), (String::new(), sql.to_owned()));
                }

                // Get session 6716397077443474616
                #[cfg(feature = "session-db")]
                {
                    let sql = r#"
                        UPDATE [session] 
                        SET
                            [last] = CURRENT_TIMESTAMP
                        OUTPUT INSERTED.[user_id], u.[role_id], INSERTED.[data], INSERTED.[lang_id]
                        FROM [session] s
                        INNER JOIN [user] u ON u.[user_id]=s.[user_id] 
                        WHERE
                            s.[session_key] = @P1
                    "#;
                    map.insert(fnv1a_64!("lib_get_session"), ("'@P1 BIGINT".to_owned(), sql.to_owned()));
                }

                // Update session -400086351751991892
                #[cfg(feature = "session-db")]
                {
                    let sql = r#"
                        UPDATE [session]
                        SET
                            [user_id] = @P1,
                            [lang_id] = @P2,
                            [data] = @P3,
                            [last] = CURRENT_TIMESTAMP,
                        WHERE
                            [session_key] = @P4
                    "#;
                    map.insert(
                        fnv1a_64!("lib_set_session"),
                        ("@P1 BIGINT, @P2 BIGINT, @P3 VARBINARY(MAX), @P4 BIGINT]".to_owned(), sql.to_owned()),
                    );
                }

                // Insert session 8029853374838241583
                #[cfg(feature = "session-db")]
                {
                    let sql = r#"
                        INSERT INTO [session] ([session], [session_key], [user_id], [lang_id], [data], [created], [last])
                        SELECT @P1, @P2, @P3, @P4, @P5, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP
                    "#;
                    map.insert(
                        fnv1a_64!("lib_add_session"),
                        ("@P1 VARCHAR(512), @P2 BIGINT, @P3 BIGINT, @P4 BIGINT, @P5 VARBINARY(MAX)".to_owned(), sql.to_owned()),
                    );
                }

                // Get redirect -1566077906756142556
                #[cfg(feature = "redirect-db")]
                {
                    let sql = r#"
                        SELECT [redirect], [permanently] FROM [redirect] WHERE [url]=@P1
                    "#;
                    map.insert(fnv1a_64!("lib_get_redirect"), ("@P1 VARCHAR(4000)".to_owned(), sql.to_owned()));
                }

                // Get route 3077841024002823969
                #[cfg(feature = "route-db")]
                {
                    let sql = r#"
                        SELECT
                            c.[module_id], c.[class_id], c.[action_id],
                            r.[params], r.[lang_id]
                        FROM
                            [route] r
                            INNER JOIN [controller] c ON c.[controller_id]=r.[controller_id]
                        WHERE r.[url]=@P1
                    "#;
                    map.insert(fnv1a_64!("lib_get_route"), ("@P1 VARCHAR(4000)".to_owned(), sql.to_owned()));
                }

                // Get route from module/class/action 8508883211214576597
                #[cfg(feature = "route-db")]
                {
                    let sql = r#"
                        SELECT TOP(1) r.[url]
                        FROM
                            [controller] c
                            INNER JOIN [route] r ON
                                r.[controller_id]=c.[controller_id] AND (r.[lang_id]=@P5 OR r.[lang_id] IS NULL) AND r.[params] = @P4
                        WHERE
                            c.[module_id]=@P1 AND c.[class_id]=@P2 AND c.[action_id]=@P3
                        ORDER BY CASE WHEN r.[lang_id] IS NOT NULL THEN 0 ELSE 1 END
                    "#;
                    map.insert(
                        fnv1a_64!("lib_get_url"),
                        ("@P1 BIGINT, @P2 BIGINT, @P3 BIGINT, @P4 VARCHAR(255), @P5 BIGINT".to_owned(), sql.to_owned()),
                    );
                }

                // Get auth permissions -4169186416014187350
                #[cfg(feature = "access-db")]
                {
                    let sql = r#"
                        SELECT ISNULL(MAX(CAST(a.[access] as TINYINT)), 0) AS [access]
                        FROM
                            [access] a
                            INNER JOIN [user] u ON u.[role_id]=a.[role_id]
                            INNER JOIN [controller] c ON a.[controller_id]=c.[controller_id]
                        WHERE
                            a.[access]=1 AND a.[role_id]=@P1 AND (
                                (c.[module_id]=-3750763034362895579 AND c.[class_id]=-3750763034362895579 AND c.[action_id]=-3750763034362895579)
                                OR (c.[module_id]=@P2 AND c.[class_id]=-3750763034362895579 AND c.[action_id]=-3750763034362895579)
                                OR (c.[module_id]=@P3 AND c.[class_id]=@P5 AND c.[action_id]=-3750763034362895579)
                                OR (c.[module_id]=@P4 AND c.[class_id]=@P6 AND c.[action_id]=@P7)
                            )
                    "#;
                    map.insert(
                        fnv1a_64!("lib_get_auth"),
                        ("@P1 BIGINT, @P2 BIGINT, @P3 BIGINT, @P4 BIGINT, @P5 BIGINT, @P6 BIGINT, @P7 BIGINT".to_owned(), sql.to_owned()),
                    );
                }

                // Get settings 2305043036426846632
                #[cfg(feature = "setting-db")]
                {
                    let sql = r#"
                        SELECT [data] FROM [setting] WHERE [key]=@P1
                    "#;
                    map.insert(fnv1a_64!("lib_get_setting"), ("@P1 BIGINT".to_owned(), sql.to_owned()));
                }

                // Insert email 5843182919945045895
                #[cfg(feature = "mail-db")]
                {
                    let sql = r#"
                        INSERT INTO [mail]([user_id], [mail], [create])
                        OUTPUT INSERTED.[mail_id]
                        VALUES (@P1, @P2, CURRENT_TIMESTAMP)
                    "#;
                    map.insert(fnv1a_64!("lib_mail_add"), ("@P1 BIGINT, @P2 NVARCHAR(MAX)".to_owned(), sql.to_owned()));
                }

                // Prepare statements
                for (key, (types, sql)) in map {
                    let sql = format!(
                        r#"
                        DECLARE @handle INT;
                        EXEC sp_prepare @handle OUTPUT,
                            N'{}',
                            N'{}';
                        SELECT @handle;
                    "#,
                        types, sql
                    );
                    match client.query(&sql, &[]).await {
                        Ok(mut stream) => match stream.try_next().await {
                            Ok(Some(QueryItem::Row(row))) => {
                                if let Some(statement) = row.get(0) {
                                    self.prepare.insert(key, statement);
                                } else {
                                    log!(stop, 0, "Error=No handle in prepare. sql={}", sql);
                                    return false;
                                }
                            }
                            Err(_e) => {
                                log!(stop, 0, "Error={}. sql={}", _e, sql);
                                return false;
                            }
                            _ => {
                                log!(stop, 0, "Error=No handle in prepare. sql={}", sql);
                                return false;
                            }
                        },
                        Err(_e) => {
                            log!(stop, 0, "Error={}. sql={}", _e, sql);
                            return false;
                        }
                    };
                }
                true
            }
            None => false,
        }
    }

    /// Executes a statement in database, returning the results
    async fn query_raw<'a, 'b>(
        client: &'a mut Option<Client<Compat<TcpStream>>>,
        query: impl Into<Cow<'b, str>>,
        params: &'b [&'b dyn ToSql],
    ) -> DBResult
    where
        'a: 'b,
    {
        match client {
            Some(sql) => match sql.query(query, params).await {
                Ok(mut s) => {
                    let mut vec = Vec::new();
                    loop {
                        match s.try_next().await {
                            Ok(Some(QueryItem::Row(row))) => vec.push(row),
                            Ok(None) => {
                                if !vec.is_empty() {
                                    break DBResult::Vec(vec);
                                } else {
                                    break DBResult::Void;
                                }
                            }
                            Ok(_) => {}
                            Err(e) => break MsSql::get_error(e),
                        }
                    }
                }
                Err(e) => MsSql::get_error(e),
            },
            None => DBResult::NoClient,
        }
    }

    // async fn query_stream_raw<'a, 'b>(
    //     client: &'a mut Option<Client<Compat<TcpStream>>>,
    //     query: impl Into<Cow<'b, str>>,
    //     params: &'b [&'b dyn ToSql],
    // ) -> DBResult<'a>
    // where
    //     'a: 'b,
    // {
    //     match client {
    //         Some(sql) => match sql.query(query, params).await {
    //             Ok(s) => DBResult::Stream(s),
    //             Err(e) => MsSql::get_error(e),
    //         },
    //         None => DBResult::NoClient,
    //     }
    // }

    /// Executes a statement in database, without results
    async fn execute_raw(
        client: &mut Option<Client<Compat<TcpStream>>>,
        query: impl Into<Cow<'_, str>>,
        params: &[&dyn ToSql],
    ) -> DBResult {
        match client.as_mut() {
            Some(sql) => match sql.execute(query, params).await {
                Ok(_) => DBResult::Void,
                Err(e) => MsSql::get_error(e),
            },
            None => DBResult::NoClient,
        }
    }

    /// Get Error from query
    fn get_error<'a>(e: Error) -> DBResult {
        match e {
            Error::Io { kind: _, message } => DBResult::ErrConnect(message),
            Error::Tls(e) => DBResult::ErrConnect(e),
            Error::Routing { host, port } => DBResult::ErrConnect(format!("Erro route: {}:{}", host, port)),
            Error::Protocol(e) => DBResult::ErrQuery(e.to_string()),
            Error::Encoding(e) => DBResult::ErrQuery(e.to_string()),
            Error::Conversion(e) => DBResult::ErrQuery(e.to_string()),
            Error::Utf8 => DBResult::ErrQuery("Error::Utf8".to_owned()),
            Error::Utf16 => DBResult::ErrQuery("Error::Utf16".to_owned()),
            Error::ParseInt(e) => DBResult::ErrQuery(e.to_string()),
            Error::Server(e) => DBResult::ErrQuery(e.to_string()),
            Error::BulkInput(e) => DBResult::ErrQuery(e.to_string()),
        }
    }

    /// Execute query to database and return a result
    #[cfg(feature = "row-data")]
    pub async fn query(&mut self, query: &str, params: QueryParam<'_>, assoc: bool) -> Option<Vec<Data>> {
        match MsSql::query_raw(&mut self.client, query, params).await {
            DBResult::Vec(rows) => return Some(self.convert(rows, assoc)),
            DBResult::Void => return Some(Vec::new()),
            DBResult::ErrQuery(_e) => {
                log!(warning, 0, "{} error={}", query, _e);
                return None;
            }
            DBResult::NoClient => log!(warning, 0),
            DBResult::ErrConnect(_e) => log!(warning, 0, "{}", _e),
        };
        self.client = None;
        if self.try_connect().await {
            match MsSql::query_raw(&mut self.client, query, params).await {
                DBResult::Vec(rows) => return Some(self.convert(rows, assoc)),
                DBResult::Void => return Some(Vec::new()),
                _ => {}
            }
        }
        None
    }

    #[cfg(feature = "row-native")]
    pub async fn query(&mut self, query: &str, params: QueryParam<'_>) -> Option<Vec<Row>> {
        match MsSql::query_raw(&mut self.client, query, params).await {
            DBResult::Vec(rows) => return Some(rows),
            // DBResult::Stream(_) => return Some(Vec::new()),
            DBResult::Void => return Some(Vec::new()),
            DBResult::ErrQuery(_e) => {
                log!(warning, 0, "{} error={}", query, _e);
                return None;
            }
            DBResult::NoClient => log!(warning, 0),
            DBResult::ErrConnect(_e) => log!(warning, 0, "{}", _e),
        };
        self.client = None;
        if self.try_connect().await {
            match MsSql::query_raw(&mut self.client, query, params).await {
                DBResult::Vec(rows) => return Some(rows),
                DBResult::Void => return Some(Vec::new()),
                _ => {}
            }
        }
        None
    }

    // #[cfg(feature = "row-native")]
    // pub async fn query_stream<'a, 'b>(&'a mut self, query: &'b str, params: QueryParam<'b>) -> Option<RowStream<'a>> {
    //     match MsSql::query_raw(&mut self.client, query, params).await {
    //         DBResult::Stream(res) => return Some(res),
    //         DBResult::Vec(_) => return None,
    //         DBResult::Void => return None,
    //         DBResult::ErrQuery(_e) => {
    //             log!(warning, 0, "{} error={}", query, _e);
    //             return None;
    //         }
    //         DBResult::NoClient => log!(warning, 0),
    //         DBResult::ErrConnect(_e) => log!(warning, 0, "{}", _e),
    //     };
    //     self.client = None;
    //     if self.try_connect().await {
    //         if let DBResult::Stream(res) = MsSql::query_raw(&mut self.client, query, params).await {
    //             return Some(res);
    //         }
    //     }
    //     None
    // }

    #[cfg(any(
        feature = "session-db",
        feature = "redirect-db",
        feature = "route-db",
        feature = "access-db",
        feature = "setting-db",
        feature = "mail-db"
    ))]
    pub(crate) async fn query_prepare(&mut self, query: i64, params: QueryParam<'_>) -> Option<Vec<Row>> {
        let stat = match self.prepare.get(&query) {
            Some(s) => s,
            None => {
                log!(warning, 0, "{:?}", query);
                return None;
            }
        };
        let mut sql = format!("EXEC sp_execute {}", stat);
        sql.reserve(20 + 6 * params.len());
        for i in 0..params.len() {
            sql.push_str(", @P");
            sql.push_str(&i.to_string());
        }

        match MsSql::query_raw(&mut self.client, sql, params).await {
            DBResult::Vec(rows) => return Some(rows),
            DBResult::Stream(_) => return Some(Vec::new()),
            DBResult::Void => return Some(Vec::new()),
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
            let mut sql = format!("EXEC sp_execute {}", stat);
            sql.reserve(20 + 6 * params.len());
            for i in 0..params.len() {
                sql.push_str(", @P");
                sql.push_str(&i.to_string());
            }
            match MsSql::query_raw(&mut self.client, sql, params).await {
                DBResult::Vec(rows) => return Some(rows),
                DBResult::Void => return Some(Vec::new()),
                _ => {}
            }
        }
        None
    }

    /// Execute query to database without a result
    pub async fn execute(&mut self, query: &str, params: &[&dyn ToSql]) -> Option<()> {
        match MsSql::execute_raw(&mut self.client, query, params).await {
            DBResult::Void | DBResult::Vec(_) => return Some(()),
            // DBResult::Stream(_) => return Some(()),
            DBResult::ErrQuery(_e) => {
                log!(warning, 0, "{} error={}", query, _e);
                return None;
            }
            DBResult::NoClient => log!(warning, 0),
            DBResult::ErrConnect(_e) => log!(warning, 0, "{}", _e),
        };
        self.client = None;
        if self.try_connect().await {
            match MsSql::execute_raw(&mut self.client, query, params).await {
                DBResult::Void | DBResult::Vec(_) => return Some(()),
                _ => {}
            }
        }
        None
    }

    /// Execute query to database without a result
    #[cfg(any(feature = "session-db", feature = "mail-db"))]
    pub(crate) async fn execute_prepare(&mut self, query: i64, params: &[&dyn ToSql]) -> Option<()> {
        let stat = match self.prepare.get(&query) {
            Some(s) => s,
            None => {
                log!(warning, 0, "{:?}", query);
                return None;
            }
        };
        let mut sql = format!("EXEC sp_execute {}", stat);
        sql.reserve(20 + 6 * params.len());
        for i in 0..params.len() {
            sql.push_str(", @P");
            sql.push_str(&i.to_string());
        }

        match MsSql::execute_raw(&mut self.client, sql, params).await {
            DBResult::Void | DBResult::Vec(_) => return Some(()),
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
            let mut sql = format!("EXEC sp_execute {}", stat);
            sql.reserve(20 + 6 * params.len());
            for i in 0..params.len() {
                sql.push_str(", @P");
                sql.push_str(&i.to_string());
            }
            match MsSql::execute_raw(&mut self.client, sql, params).await {
                DBResult::Void | DBResult::Vec(_) => return Some(()),
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
    fn get_column_type<'a>(&self, cols: &'a [Column]) -> Vec<fn(&Row, usize) -> Data> {
        let mut columns = Vec::with_capacity(cols.len());
        for col in cols {
            let func = match col.column_type() {
                tiberius::ColumnType::Bit => Self::get_bool,
                tiberius::ColumnType::Int1 => Self::get_i8,
                tiberius::ColumnType::Int2 => Self::get_i16,
                tiberius::ColumnType::Int4 => Self::get_i32,
                tiberius::ColumnType::Int8 => Self::get_i64,
                tiberius::ColumnType::Float4 => Self::get_f32,
                tiberius::ColumnType::Float8 => Self::get_f64,
                tiberius::ColumnType::DatetimeOffsetn => Self::get_date,
                tiberius::ColumnType::Guid => Self::get_uuid,
                tiberius::ColumnType::BigVarBin => Self::get_bytea,
                tiberius::ColumnType::BigVarChar => Self::get_string,
                tiberius::ColumnType::BigBinary => Self::get_bytea,
                tiberius::ColumnType::BigChar => Self::get_string,
                tiberius::ColumnType::NVarchar => Self::get_string,
                tiberius::ColumnType::Text => Self::get_string,
                _u => {
                    log!(warning, 0, "Type: {:?}", _u);
                    Self::get_unknown
                }
            };
            columns.push(func);
        }
        columns
    }

    /// Detect columns' type with columns' name
    #[cfg(feature = "row-data")]
    fn get_column_type_name(&self, cols: &[Column]) -> HashMap<i64, MsColumnName> {
        let mut columns = HashMap::with_capacity(cols.len());
        for (idx, col) in cols.iter().enumerate() {
            let func = match col.column_type() {
                tiberius::ColumnType::Bit => Self::get_bool,
                tiberius::ColumnType::Int1 => Self::get_i8,
                tiberius::ColumnType::Int2 => Self::get_i16,
                tiberius::ColumnType::Int4 => Self::get_i32,
                tiberius::ColumnType::Int8 => Self::get_i64,
                tiberius::ColumnType::Float4 => Self::get_f32,
                tiberius::ColumnType::Float8 => Self::get_f64,
                tiberius::ColumnType::DatetimeOffsetn => Self::get_date,
                tiberius::ColumnType::Guid => Self::get_uuid,
                tiberius::ColumnType::BigVarBin => Self::get_bytea,
                tiberius::ColumnType::BigVarChar => Self::get_string,
                tiberius::ColumnType::BigBinary => Self::get_bytea,
                tiberius::ColumnType::BigChar => Self::get_string,
                tiberius::ColumnType::NVarchar => Self::get_string,
                tiberius::ColumnType::Text => Self::get_string,
                _u => {
                    log!(warning, 0, "Type: {:?}", _u);
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

    /// Row::i8 to Data::I16
    #[cfg(feature = "row-data")]
    #[inline]
    fn get_i8(row: &Row, idx: usize) -> Data {
        let i: Option<i16> = row.get(idx);
        match i {
            Some(i) => Data::I16(i),
            None => Data::None,
        }
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
        let s: Result<Option<DateTime<Utc>>, Error> = row.try_get(idx);
        match s {
            Ok(Some(s)) => Data::Date(s),
            Ok(None) => Data::None,
            Err(_) => Data::None,
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

    /// Row::String to Data::String
    #[cfg(feature = "row-data")]
    #[inline]
    fn get_string(row: &Row, idx: usize) -> Data {
        let s: Result<Option<&str>, Error> = row.try_get(idx);
        match s {
            Ok(Some(s)) => Data::String(s.to_owned()),
            Ok(None) => Data::None,
            Err(_) => Data::None,
        }
    }

    /// Row::Vec<u8> to Data::Raw
    #[cfg(feature = "row-data")]
    #[inline]
    fn get_bytea(row: &Row, idx: usize) -> Data {
        let s: Result<Option<&[u8]>, Error> = row.try_get(idx);
        match s {
            Ok(Some(s)) => Data::Raw(s.to_vec()),
            Ok(None) => Data::None,
            Err(_) => Data::None,
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

// pub struct QueryStream<'a> {
//     pub(crate) permit: SemaphorePermit<'a>,
//     pub(crate) db: MutexGuard<'a, MsSql>,
//     pub(crate) stream: Pin<Box<RowStream<'a>>>,
//     #[cfg(any(feature = "debug-v", feature = "debug-vv", feature = "debug-vvv"))]
//     pub(crate) sql: &'a str,
//     #[cfg(feature = "row-data")]
//     pub(crate) cols: PgColumn,
// }

// impl<'a> QueryStream<'a> {
//     #[cfg(feature = "row-data")]
//     pub async fn next(&mut self) -> Option<Data> {
//         match self.stream.try_next().await {
//             Ok(row) => Some(self.convert(row?)),
//             Err(_e) => {
//                 log!(warning, 0, "{} error={}", self.sql, _e);
//                 None
//             }
//         }
//     }

//     #[cfg(feature = "row-data")]
//     fn convert(&mut self, row: Row) -> Data {
//         match self.cols {
//             PgColumn::Vec(ref mut vec) => {
//                 let cols = vec.get_or_insert_with(|| self.db.get_column_type(row.columns()));
//                 let mut v = Vec::with_capacity(cols.len());
//                 let mut func;
//                 for idx in 0..cols.len() {
//                     func = unsafe { cols.get_unchecked(idx) };
//                     v.push(func(&row, idx))
//                 }
//                 Data::Vec(v)
//             }
//             PgColumn::Map(ref mut map) => {
//                 let cols = map.get_or_insert_with(|| self.db.get_column_type_name(row.columns()));
//                 let mut t = HashMap::with_capacity(cols.len());
//                 for (name, turple) in cols {
//                     t.insert(*name, turple.1(&row, turple.0));
//                 }
//                 Data::Map(t)
//             }
//         }
//     }
// }

// impl Drop for QueryStream<'_> {
//     fn drop(&mut self) {
//         let _ = &mut self.stream;
//         let _ = &mut self.db;
//         let _ = &mut self.permit;
//     }
// }
