use std::{
    borrow::Cow,
    collections::{btree_map::Entry, BTreeMap},
    sync::Arc,
};

use chrono::{DateTime, Utc};

use futures_util::TryStreamExt;
use tiberius::{error::Error, AuthMethod, Client, Column, Config, EncryptionLevel, QueryItem, Row, ToSql};
use tiny_web_macro::fnv1a_64;
use tokio::net::TcpStream;
use tokio_util::compat::{Compat, TokioAsyncWriteCompatExt};

use crate::sys::{action::Data, init::DBConfig, log::Log};

use super::adapter::{DBFieldType, DBPrepare, KeyOrQuery, StrOrI64OrUSize};

/// Response to the result of the query
enum DBResult {
    /// The request was completed successfully.
    Vec(Vec<Row>),
    /// The request was completed successfully without result.
    Void,
    /// Connection is empty.
    NoClient,
    /// Query execution error.
    ErrQuery(String),
    /// Connection is lost.
    ErrConnect(String),
    /// No prepare query
    ErrPrepare,
}

/// Responsible for working with MsSql database
pub struct MsSql {
    config: Config,
    client: Option<Client<Compat<TcpStream>>>,
    prepare: BTreeMap<i64, MsStatement>,
    external: Arc<BTreeMap<i64, DBPrepare>>,
}

/// Statement to database
pub struct MsStatement {
    /// Statement to database
    statement: i64,
    /// Sql query to database
    sql: String,
}

/// Names of columns
type MsColumnName = (usize, fn(&Row, usize) -> Data);

impl MsSql {
    /// Initializes a new object `PgSql`
    pub fn new(config: Arc<DBConfig>, prepare: Arc<BTreeMap<i64, DBPrepare>>) -> Option<MsSql> {
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
        if config.sslmode {
            cfg.encryption(EncryptionLevel::Required);
        }

        Some(MsSql {
            config: cfg,
            client: None,
            prepare: BTreeMap::new(),
            external: prepare,
        })
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
        let tcp = match TcpStream::connect(self.config.get_addr()).await {
            Ok(tcp) => tcp,
            Err(e) => {
                Log::stop(601, Some(e.to_string()));
                return false;
            }
        };
        if let Err(e) = tcp.set_nodelay(true) {
            Log::stop(616, Some(e.to_string()));
            return false;
        };
        let client = match Client::connect(self.config.clone(), tcp.compat_write()).await {
            Ok(client) => client,
            Err(e) => {
                Log::stop(604, Some(e.to_string()));
                return false;
            }
        };
        self.client = Some(client);

        self.prepare().await
    }

    /// Prepare sql statement
    async fn prepare(&mut self) -> bool {
        self.prepare.clear();
        match self.client.as_mut() {
            Some(client) => {
                let mut map = BTreeMap::new();
                // Get lang
                let sql = r#"
                    SELECT [lang_id], [lang], [name], [index]
                    FROM [lang]
                    WHERE [enable]=1
                    ORDER BY [sort]
                "#;
                map.insert(fnv1a_64!("lib_get_langs"), ("".to_owned(), sql.to_owned()));
                // Get session
                let sql = r#"
                    UPDATE [session] 
                    SET
                        [last] = CURRENT_TIMESTAMP
                    OUTPUT INSERTED.[session_id], INSERTED.[user_id], u.[role_id], INSERTED.[data], INSERTED.[lang_id]
                    FROM [session] s
                    INNER JOIN [user] u ON u.[user_id]=s.[user_id] 
                    WHERE
                        s.[session] = @P1
                "#;
                map.insert(fnv1a_64!("lib_get_session"), ("'@P1 VARCHAR(512)".to_owned(), sql.to_owned()));

                // Update session
                let sql = r#"
                    UPDATE [session]
                    SET
                        [user_id] = @P1,
                        [lang_id] = @P2,
                        [data] = @P3,
                        [last] = CURRENT_TIMESTAMP,
                        [ip] = @P4,
                        [user_agent] = @P5
                    WHERE
                        [session_id] = @P6
                "#;
                map.insert(
                    fnv1a_64!("lib_set_session"),
                    (
                        "@P1 BIGINT, @P2 BIGINT, @P3 VARBINARY(MAX), @P4 VARCHAR(255), @P5 VARCHAR(MAX), @P6 BIGINT]".to_owned(),
                        sql.to_owned(),
                    ),
                );

                // Insert session
                let sql = r#"
                    INSERT INTO [session] ([user_id], [lang_id], [session], [data], [created], [last], [ip], [user_agent])
                    SELECT @P1, @P2, @P3, @P4, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP, @P5, @P6
                "#;
                map.insert(
                    fnv1a_64!("lib_add_session"),
                    (
                        "@P1 BIGINT, @P2 BIGINT, @P3 VARCHAR(512), @P4 VARBINARY(MAX), @P5 VARCHAR(255), @P6 VARCHAR(MAX)"
                            .to_owned(),
                        sql.to_owned(),
                    ),
                );

                // Get redirect
                let sql = r#"
                    SELECT [redirect], [permanently] FROM [redirect] WHERE [url]=@P1
                "#;
                map.insert(fnv1a_64!("lib_get_redirect"), ("@P1 VARCHAR(4000)".to_owned(), sql.to_owned()));

                // Get route
                let sql = r#"
                    SELECT
                        c.[module], c.[class], c.[action],
                        c.[module_id], c.[class_id], c.[action_id],
                        r.[params], r.[lang_id]
                    FROM
                        [route] r
                        INNER JOIN [controller] c ON c.[controller_id]=r.[controller_id]
                    WHERE r.[url]=@P1
                "#;
                map.insert(fnv1a_64!("lib_get_route"), ("@P1 VARCHAR(4000)".to_owned(), sql.to_owned()));

                // Get route from module/class/action
                let sql = r#"
                    SELECT r.[url]
                    FROM
                        [controller] c
                        INNER JOIN [route] r ON
                            r.[controller_id]=c.[controller_id] AND r.[lang_id]=@P5 AND r.[params] = @P4
                    WHERE
                        c.[module_id]=@P1 AND c.[class_id]=@P2 AND c.[action_id]=@P3
                "#;
                map.insert(
                    fnv1a_64!("lib_get_url"),
                    ("@P1 BIGINT, @P2 BIGINT, @P3 BIGINT, @P4 BIGINT, @P5 VARCHAR(255)".to_owned(), sql.to_owned()),
                );

                // Get auth permissions
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
                    (
                        "@P1 BIGINT, @P2 BIGINT, @P3 BIGINT, @P4 BIGINT, @P5 BIGINT, @P6 BIGINT, @P7 BIGINT".to_owned(),
                        sql.to_owned(),
                    ),
                );

                // Get not found
                let sql = r#"
                    SELECT [url]
                    FROM [route]
                    WHERE [controller_id]=3 AND [lang_id]=@P1
                "#;
                map.insert(fnv1a_64!("lib_get_not_found"), ("@P1 BIGINT".to_owned(), sql.to_owned()));

                // Get settings
                let sql = r#"
                    SELECT [data] FROM [setting] WHERE [key]=@P1
                "#;
                map.insert(fnv1a_64!("lib_get_setting"), ("@P1 BIGINT".to_owned(), sql.to_owned()));

                // Insert email
                let sql = r#"
                    INSERT INTO [mail]([user_id], [mail], [create], [err], [transport])
                    OUTPUT INSERTED.[mail_id]
                    VALUES (@P1, @P2, CURRENT_TIMESTAMP, 0, @P3)
                "#;
                map.insert(
                    fnv1a_64!("lib_mail_new"),
                    ("@P1 BIGINT, @P2 NVARCHAR(MAX), @P3 VARCHAR(255)".to_owned(), sql.to_owned()),
                );

                // Insert email without provider
                let sql = r#"
                    INSERT INTO [mail]([user_id], [mail], [create], [send], [err], [transport])
                    VALUES (@P1, @P2, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP, 0, 'None')
                "#;
                map.insert(fnv1a_64!("lib_mail_add"), ("@P1 BIGINT, @P2 NVARCHAR(MAX)".to_owned(), sql.to_owned()));

                // Insert error send email
                let sql = r#"
                    UPDATE [mail]
                    SET [err]=1, [send]=CURRENT_TIMESTAMP, [err_text]=@P1
                    WHERE [mail_id]=@P2
                "#;
                map.insert(fnv1a_64!("lib_mail_err"), ("@P1 NVARCHAR(MAX), @P2 BIGINT".to_owned(), sql.to_owned()));

                // Insert success send email
                let sql = r#"
                    UPDATE [mail]
                    SET [err]=0, [send]=CURRENT_TIMESTAMP
                    WHERE [mail_id]=@P1
                "#;
                map.insert(fnv1a_64!("lib_mail_ok"), ("@P1 BIGINT".to_owned(), sql.to_owned()));

                // Add config prepare
                for (key, sql) in self.external.as_ref() {
                    if let DBFieldType::Mssql(vec) = &sql.types {
                        let res = vec
                            .iter()
                            .enumerate()
                            .map(|(index, s)| format!("@P{} {}", index, s))
                            .collect::<Vec<String>>()
                            .join(", ");
                        map.insert(*key, (res, sql.query.to_owned()));
                    }
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
                                    self.prepare.insert(key, MsStatement { statement, sql });
                                } else {
                                    Log::stop(613, Some(format!("Error=No handle in prepare. sql={}", sql)));
                                    return false;
                                }
                            }
                            Err(e) => {
                                Log::stop(613, Some(format!("Error={}. sql={}", e, sql)));
                                return false;
                            }
                            _ => {
                                Log::stop(613, Some(format!("Error=No handle in prepare. sql={}", sql)));
                                return false;
                            }
                        },
                        Err(e) => {
                            Log::stop(613, Some(format!("Error={}. sql={}", e, sql)));
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

    /// Executes a statement in database, without results
    async fn execute_raw<'a>(
        client: &mut Option<Client<Compat<TcpStream>>>,
        query: impl Into<Cow<'a, str>>,
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
    fn get_error(e: Error) -> DBResult {
        match e {
            Error::Io { kind: _, message } => DBResult::ErrConnect(message),
            Error::Tls(e) => DBResult::ErrConnect(e),
            tiberius::error::Error::Routing { host, port } => DBResult::ErrConnect(format!("Erro route: {}:{}", host, port)),
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
    async fn query_statement(&mut self, query: &impl KeyOrQuery, params: &[&dyn ToSql]) -> DBResult {
        if query.is_key() {
            let stat = match self.prepare.get(&query.to_i64()) {
                Some(s) => s,
                None => return DBResult::ErrPrepare,
            };
            let mut sql = format!("EXEC sp_execute {}", stat.statement);
            sql.reserve(20 + 6 * params.len());
            for i in 0..params.len() {
                sql.push_str(", @P");
                sql.push_str(&i.to_string());
            }
            MsSql::query_raw(&mut self.client, sql, params).await
        } else {
            MsSql::query_raw(&mut self.client, query.to_str(), params).await
        }
    }

    /// Execute query to database without a result
    async fn execute_statement(&mut self, query: &impl KeyOrQuery, params: &[&(dyn ToSql)]) -> DBResult {
        if query.is_key() {
            let stat = match self.prepare.get(&query.to_i64()) {
                Some(s) => s,
                None => return DBResult::ErrPrepare,
            };
            let mut sql = format!("EXEC sp_execute {}", stat.statement);
            sql.reserve(20 + 6 * params.len());
            for i in 0..params.len() {
                sql.push_str(", @P");
                sql.push_str(&i.to_string());
            }
            MsSql::execute_raw(&mut self.client, sql, params).await
        } else {
            MsSql::execute_raw(&mut self.client, query.to_str(), params).await
        }
    }

    /// Execute query to database and return a result
    pub async fn query(&mut self, query: &impl KeyOrQuery, params: &[&dyn ToSql], assoc: bool) -> Option<Vec<Data>> {
        match self.query_statement(query, params).await {
            DBResult::Vec(rows) => return Some(self.convert(rows, assoc)),
            DBResult::Void => return Some(Vec::new()),
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
            match self.query_statement(query, params).await {
                DBResult::Vec(rows) => return Some(self.convert(rows, assoc)),
                DBResult::Void => return Some(Vec::new()),
                _ => {}
            }
        }
        None
    }

    /// Execute query to database without a result
    pub async fn execute(&mut self, query: &impl KeyOrQuery, params: &[&dyn ToSql]) -> Option<()> {
        match self.execute_statement(query, params).await {
            DBResult::Void | DBResult::Vec(_) => return Some(()),
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
            match self.execute_statement(query, params).await {
                DBResult::Void | DBResult::Vec(_) => return Some(()),
                _ => {}
            }
        }
        None
    }

    /// Execute query to database and return a result,  
    /// and grouping tabular data according to specified conditions.
    pub async fn query_group(
        &mut self,
        query: &impl KeyOrQuery,
        params: &[&dyn ToSql],
        assoc: bool,
        conds: &[&[impl StrOrI64OrUSize]],
    ) -> Option<Data> {
        if conds.is_empty() {
            return None;
        }
        match self.query_statement(query, params).await {
            DBResult::Vec(rows) => {
                if rows.is_empty() {
                    return Some(Data::Map(BTreeMap::new()));
                }
                if assoc {
                    return Some(self.convert_map(rows, conds));
                } else {
                    return Some(self.convert_vec(rows, conds));
                }
            }
            DBResult::Void => return Some(Data::Map(BTreeMap::new())),
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
            match self.query_statement(query, params).await {
                DBResult::Vec(rows) => {
                    if rows.is_empty() {
                        return Some(Data::Map(BTreeMap::new()));
                    }
                    if assoc {
                        return Some(self.convert_map(rows, conds));
                    } else {
                        return Some(self.convert_vec(rows, conds));
                    }
                }
                DBResult::Void => return Some(Data::Map(BTreeMap::new())),
                _ => {}
            }
        }
        None
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
        columns: &BTreeMap<i64, MsColumnName>,
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
                u => {
                    Log::warning(614, Some(format!("Type: {:?}", u)));
                    Self::get_unknown
                }
            };
            columns.push(func);
        }
        columns
    }

    /// Detect columns' type with columns' name
    fn get_column_type_name(&self, cols: &[Column]) -> BTreeMap<i64, MsColumnName> {
        let mut columns = BTreeMap::new();
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
                u => {
                    Log::warning(614, Some(format!("Type: {:?}", u)));
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

    /// Row::i8 to Data::I16
    #[inline]
    fn get_i8(row: &Row, idx: usize) -> Data {
        let i: Option<i16> = row.get(idx);
        match i {
            Some(i) => Data::I16(i),
            None => Data::None,
        }
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

    /// Row::i64 to Data::I64
    #[inline]
    fn get_i64(row: &Row, idx: usize) -> Data {
        let i: Option<i64> = row.get(idx);
        match i {
            Some(i) => Data::I64(i),
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

    /// Row::f64 to Data::F64
    #[inline]
    fn get_f64(row: &Row, idx: usize) -> Data {
        let f: Option<f64> = row.get(idx);
        match f {
            Some(f) => Data::F64(f),
            None => Data::None,
        }
    }

    /// Row::DateTime<Utc> to Data::DateTime<Utc>
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
    #[inline]
    fn get_uuid(row: &Row, idx: usize) -> Data {
        let u: Option<uuid::Uuid> = row.get(idx);
        match u {
            Some(u) => Data::String(u.to_string()),
            None => Data::None,
        }
    }

    /// Row::String to Data::String
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
    #[inline]
    fn get_bool(row: &Row, idx: usize) -> Data {
        let b: Option<bool> = row.get(idx);
        match b {
            Some(b) => Data::Bool(b),
            None => Data::None,
        }
    }
}

impl std::fmt::Debug for MsSql {
    /// Formats the value using the given formatter.
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        let MsSql { client, prepare, external, config } = self;
        f.debug_struct("DB")
            .field("client", &client)
            .field("config", &config)
            .field("prepare", &prepare)
            .field("external", &external)
            .finish()
    }
}

impl std::fmt::Debug for MsStatement {
    /// Formats the value using the given formatter.
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        let MsStatement { statement, sql } = self;
        f.debug_struct("PgStatement").field("sql", &sql).field("statement", &statement).finish()
    }
}
