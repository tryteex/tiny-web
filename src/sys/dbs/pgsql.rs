use std::{
    collections::{btree_map::Entry, BTreeMap},
    sync::Arc,
};

use chrono::{DateTime, Utc};
use futures_util::{pin_mut, TryStreamExt};
use postgres::{types::ToSql, NoTls, Row, Statement, ToStatement};
use rustls::{ClientConfig, RootCertStore};
use serde_json::Value;
use tokio_postgres::{types::Type, Client, Column};

use tiny_web_macro::fnv1a_64;

use crate::sys::{action::Data, init::DBConfig, log::Log};

use super::adapter::{KeyOrQuery, MakeTinyTlsConnect, StrOrI64OrUSize};

/// Response to the result of the query
#[derive(Debug)]
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

/// Responsible for working with postgresql database
pub(crate) struct PgSql {
    /// Client for connection to database.
    client: Option<Client>,
    /// Connection config.
    sql_conn: tokio_postgres::Config,
    /// Use tls for connection when sslmode=require.
    tls: Option<MakeTinyTlsConnect>,
    /// Prepare statements to database.
    prepare: BTreeMap<i64, PgStatement>,
}

/// Statement to database
pub(crate) struct PgStatement {
    /// Statement to database
    statement: Statement,
    /// Sql query to database
    sql: String,
}

/// Names of columns
type PgColumnName = (usize, fn(&Row, usize) -> Data);

impl PgSql {
    /// Initializes a new object `PgSql`
    pub fn new(config: Arc<DBConfig>) -> Option<PgSql> {
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
            let config = ClientConfig::builder().with_root_certificates(RootCertStore::empty()).with_no_client_auth();
            Some(MakeTinyTlsConnect::new(config))
        } else {
            None
        };
        Some(PgSql {
            client: None,
            sql_conn,
            tls,
            prepare: BTreeMap::new(),
        })
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
        self.prepare().await
    }

    /// Prepare sql statement
    async fn prepare(&mut self) -> bool {
        self.prepare.clear();
        match &self.client {
            Some(client) => {
                let mut map = BTreeMap::new();

                // Get lang
                let sql = r#"
                    SELECT lang_id, lang, name, index
                    FROM lang
                    WHERE enable
                    ORDER BY sort
                "#;
                map.insert(fnv1a_64!("lib_get_langs"), (client.prepare_typed(sql, &[]), sql.to_owned()));

                // Get session
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
                        s.session_id, s.user_id, u.role_id, s.data, s.lang_id 
                    FROM 
                        upd s
                        INNER JOIN "user" u ON u.user_id=s.user_id
                "#;
                map.insert(fnv1a_64!("lib_get_session"), (client.prepare_typed(sql, &[Type::TEXT]), sql.to_owned()));

                // Update session
                let sql = r#"
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
                "#;
                map.insert(
                    fnv1a_64!("lib_set_session"),
                    (
                        client.prepare_typed(sql, &[Type::INT8, Type::INT8, Type::BYTEA, Type::TEXT, Type::TEXT, Type::INT8]),
                        sql.to_owned(),
                    ),
                );

                // Insert session
                let sql = r#"
                    INSERT INTO session (user_id, lang_id, session, data, created, last, ip, user_agent)
                    SELECT $1, $2, $3, $4, now(), now(), $5, $6
                "#;
                map.insert(
                    fnv1a_64!("lib_add_session"),
                    (
                        client.prepare_typed(sql, &[Type::INT8, Type::INT8, Type::TEXT, Type::BYTEA, Type::TEXT, Type::TEXT]),
                        sql.to_owned(),
                    ),
                );

                // Get redirect
                let sql = r#"
                    SELECT redirect, permanently FROM redirect WHERE url=$1
                "#;
                map.insert(fnv1a_64!("lib_get_redirect"), (client.prepare_typed(sql, &[Type::TEXT]), sql.to_owned()));

                // Get route
                let sql = r#"
                    SELECT 
                        c.module, c.class, c.action,
                        c.module_id, c.class_id, c.action_id,
                        r.params, r.lang_id
                    FROM 
                        route r
                        INNER JOIN controller c ON c.controller_id=r.controller_id
                    WHERE r.url=$1
                "#;
                map.insert(fnv1a_64!("lib_get_route"), (client.prepare_typed(sql, &[Type::TEXT]), sql.to_owned()));
                // Get route from module/class/action
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

                // Get auth permissions
                let sql = r#"
                    SELECT ISNULL(MAX(a.access), 0) AS access
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
                // Get not found
                let sql = r#"
                    SELECT url
                    FROM route
                    WHERE controller_id=3 AND lang_id=$1
                "#;
                map.insert(fnv1a_64!("lib_get_not_found"), (client.prepare_typed(sql, &[Type::INT8]), sql.to_owned()));
                // Get settings
                let sql = r#"
                    SELECT data FROM setting WHERE key=$1
                "#;
                map.insert(fnv1a_64!("lib_get_setting"), (client.prepare_typed(sql, &[Type::INT8]), sql.to_owned()));
                // Insert email
                let sql = r#"
                    INSERT INTO mail(user_id, mail, "create", err, transport)
                    VALUES ($1, $2, now(), false, $3)
                    RETURNING mail_id;
                "#;
                map.insert(
                    fnv1a_64!("lib_mail_new"),
                    (client.prepare_typed(sql, &[Type::INT8, Type::TEXT, Type::TEXT]), sql.to_owned()),
                );
                // Insert email without provider
                let sql = r#"
                    INSERT INTO mail(user_id, mail, "create", send, err, transport)
                    VALUES ($1, $2, now(), now(), false, 'None')
                "#;
                map.insert(fnv1a_64!("lib_mail_add"), (client.prepare_typed(sql, &[Type::INT8, Type::TEXT]), sql.to_owned()));
                // Insert error send email
                let sql = r#"
                    UPDATE mail
                    SET err=true, send=now(), err_text=$1
                    WHERE mail_id=$2
                "#;
                map.insert(fnv1a_64!("lib_mail_err"), (client.prepare_typed(sql, &[Type::TEXT, Type::INT8]), sql.to_owned()));
                // Insert success send email
                let sql = r#"
                    UPDATE mail
                    SET err=false, send=now()
                    WHERE mail_id=$1
                "#;
                map.insert(fnv1a_64!("lib_mail_ok"), (client.prepare_typed(sql, &[Type::INT8]), sql.to_owned()));

                // Prepare statements
                for (key, (prepare, sql)) in map {
                    match prepare.await {
                        Ok(s) => {
                            self.prepare.insert(key, PgStatement { statement: s, sql });
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

    /// Execute query to database and return a result
    async fn query_statement(&self, query: &impl KeyOrQuery, params: &[&(dyn ToSql + Sync)]) -> DBResult {
        if query.is_key() {
            let stat = match self.prepare.get(&query.to_i64()) {
                Some(s) => s,
                None => return DBResult::ErrPrepare,
            };
            PgSql::query_raw(&self.client, &stat.statement, params).await
        } else {
            PgSql::query_raw(&self.client, query.to_str(), params).await
        }
    }

    /// Execute query to database without a result
    async fn execute_statement(&self, query: &impl KeyOrQuery, params: &[&(dyn ToSql + Sync)]) -> DBResult {
        if query.is_key() {
            let stat = match self.prepare.get(&query.to_i64()) {
                Some(s) => s,
                None => return DBResult::ErrPrepare,
            };
            PgSql::execute_raw(&self.client, &stat.statement, params).await
        } else {
            PgSql::execute_raw(&self.client, query.to_str(), params).await
        }
    }

    /// Execute query to database without a result
    pub async fn execute(&mut self, query: &impl KeyOrQuery, params: &[&(dyn ToSql + Sync)]) -> Option<()> {
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

    /// Execute query to database and return a result
    pub async fn query(&mut self, query: &impl KeyOrQuery, params: &[&(dyn ToSql + Sync)], assoc: bool) -> Option<Vec<Data>> {
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

    /// Execute query to database and return a result,  
    /// and grouping tabular data according to specified conditions.
    pub async fn query_group(
        &mut self,
        query: &impl KeyOrQuery,
        params: &[&(dyn ToSql + Sync)],
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
        Some(Data::Map(BTreeMap::new()))
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
        columns: &BTreeMap<i64, PgColumnName>,
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
    fn get_column_type_name(&self, cols: &[Column]) -> BTreeMap<i64, PgColumnName> {
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
        let d: Option<DateTime<Utc>> = row.get(idx);
        match d {
            Some(d) => Data::Date(d),
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

impl std::fmt::Debug for PgSql {
    /// Formats the value using the given formatter.
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        let tls = self.tls.clone().map(|_| "TlsConnector");
        let PgSql { client, sql_conn, tls: _, prepare } = self;
        f.debug_struct("DB")
            .field("client", &client)
            .field("sql_conn", &sql_conn)
            .field("tls", &tls)
            .field("prepare", &prepare)
            .finish()
    }
}

impl std::fmt::Debug for PgStatement {
    /// Formats the value using the given formatter.
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        let PgStatement { statement, sql } = self;
        f.debug_struct("PgStatement")
            .field("sql", &sql)
            .field("columns", &statement.columns())
            .field("params", &statement.params())
            .finish()
    }
}
