use std::{
    collections::BTreeMap,
    env,
    fs::read_to_string,
    io::ErrorKind,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    str::FromStr,
    sync::Arc,
};

use toml::{map::Map, Table, Value};

use crate::fnv1a_64;

use super::{
    dbs::adapter::{DBEngine, DBFieldType, DBPrepare},
    log::Log,
    worker::WorkerType,
};

/// Responsible for the IP address that should be accepted.
///
/// # Values
///
/// * `Any` - Accepts any IP address;
/// * `IpAddr: IpAddr` - Accepts a specific IP address;
/// * `UDS` - Accepts only from Unix Domain Socket.
#[derive(Debug, Clone)]
pub enum AcceptAddr {
    /// Accepts any IP address.
    Any,
    /// Accepts a specific IP address;
    IpAddr(IpAddr),
    /// Accepts only from Unix Domain Socket.
    #[cfg(not(target_family = "windows"))]
    UDS,
}

/// Responsible for the IP address.
///
/// # Values
///
/// * `SocketAddr` - Accepts any socket address;
/// * `UDS: String` - Accepts only from Unix Domain Socket.
#[derive(Debug, Clone)]
pub enum Addr {
    /// Accepts any IP address.
    SocketAddr(SocketAddr),
    /// Accepts only from Unix Domain Socket.
    #[cfg(not(target_family = "windows"))]
    UDS(String),
}

/// Responsible for database configuration data.
///
/// # Values
///
/// * `engine: DBEngine` - Engine of database;
/// * `host: String` - Host of database;
/// * `port: Option<u16>` - Port of database;
/// * `name: String` - Name of database;
/// * `user: Option<String>` - Database user;
/// * `pwd: Option<String>` - Password of database user;
/// * `sslmode: bool` - Use for sslmode=require when connecting to the database;
/// * `max: SysCount` - The number of connections that will be used in the pool;
#[derive(Debug, Clone)]
pub struct DBConfig {
    /// Engine of database.
    pub engine: DBEngine,
    /// Host of database.
    pub host: String,
    /// Port of database.
    pub port: Option<u16>,
    /// Name of database.
    pub name: String,
    /// Database user.
    pub user: Option<String>,
    /// Password of database user.
    pub pwd: Option<String>,
    /// Use for sslmode=require when connecting to the database
    pub sslmode: bool,
    /// The number of connections that will be used in the pool
    pub max: usize,
}

/// Describes the server configuration.
#[derive(Debug, Clone)]
pub struct Config {
    /// Name server from env!("CARGO_PKG_NAME") primary project.
    pub name: String,
    /// Description server from env!("CARGO_PKG_DESCRIPTION") primary project.
    pub desc: String,
    /// Server version from env!("CARGO_PKG_VERSION") primary project.
    pub version: String,
    /// Default language.
    pub lang: Arc<String>,
    /// Path to log file.
    pub log: String,
    /// Number of work processes in async operations.
    pub max: usize,
    /// The address from which we accept working connections.
    pub bind_accept: Arc<AcceptAddr>,
    /// The address of the server that binds clients.
    pub bind: Addr,
    /// The address from which we accept connections for managing the server.
    pub rpc_accept: AcceptAddr,
    /// IP address from which to bind connections for managing the server.
    pub rpc: Addr,
    /// Session key
    pub session: Arc<String>,
    /// Salt for a crypto functions.
    pub salt: Arc<String>,
    /// Database configuration.
    pub db: Arc<DBConfig>,
    /// Stop signal
    pub stop: i64,
    /// Status signal
    pub status: i64,
    /// Protocol
    pub protocol: WorkerType,
    /// Prepare sql queries
    pub prepare: Arc<BTreeMap<i64, DBPrepare>>,
}

/// Responsible for running mode of server.
///
/// # Values
///
/// * `Start` - Start the server;
/// * `Stop` - Stop the server;
/// * `Status` - Get status from server;
/// * `Help` - Display a short help on starting the server;
/// * `Go` - Start the server in interactive mode.
#[derive(Debug, Clone, PartialEq)]
pub enum Mode {
    /// Start the server.
    Start,
    /// Stop the server.
    Stop,
    /// Get status from server.
    Status,
    /// Display a short help on starting the server.
    Help,
    /// Start the server in interactive mode.
    Go,
}

/// Describes the server configuration:
///
/// # Values
///
/// * `mode: Mode` - Running mode of server;
/// * `conf: Config` - Server configuration;
/// * `exe_path: String` - The full path to the folder where this executable file is located;
/// * `exe_file: String` - The full path to this executable file;
/// * `conf_file: String` - The full path to configuration file;
/// * `root_path: String` - The full path to the folder where the server was started.
#[derive(Debug, Clone)]
pub struct Init {
    /// Running mode of server.
    pub mode: Mode,
    /// Server configuration.
    pub conf: Config,
    /// The full path to the folder where this executable file is located.
    pub exe_path: String,
    /// The full path to this executable file.
    pub exe_file: String,
    /// The full path to configuration file.
    pub conf_file: String,
    /// The full path to the folder where the server was started.
    pub root_path: Arc<String>,
}

impl Init {
    /// Initializes the server configuration
    pub fn new(name: &str, version: &str, desc: &str) -> Option<Init> {
        let exe_file = Init::get_current_exe()?;

        let exe_path = match exe_file.rfind('/') {
            Some(i) => exe_file[..i].to_owned(),
            None => {
                Log::stop(16, Some(exe_file));
                return None;
            }
        };

        let mut args = env::args();

        let mode;
        let conf;
        let conf_file;
        let root_path;

        args.next();
        // Check first parameter
        // Can be mode Or root Or empty
        match args.next() {
            // first parameter is empty
            None => {
                mode = Mode::Help;
                (conf_file, conf, root_path) = Init::check_path(&exe_path)?;
            }
            // first parameter is not empty
            Some(arg) => {
                match arg.as_str() {
                    "start" => mode = Mode::Start,
                    "stop" => mode = Mode::Stop,
                    "status" => mode = Mode::Status,
                    "go" => mode = Mode::Go,
                    _ => mode = Mode::Help,
                };
                if mode != Mode::Help {
                    // check second parameter
                    // if second parameter is root try to read configuration file
                    match args.next() {
                        // second parameter is not empty
                        Some(c) => {
                            // second parameter is root
                            if c.as_str() == "-r" {
                                match args.next() {
                                    Some(p) => {
                                        let file = format!("{}/tiny.toml", p);
                                        match read_to_string(&file) {
                                            Ok(s) => {
                                                conf_file = file;
                                                conf = s;
                                                root_path = match conf_file.rfind('/') {
                                                    Some(i) => conf_file[..i].to_owned(),
                                                    None => {
                                                        Log::stop(16, Some(conf_file));
                                                        return None;
                                                    }
                                                }
                                            }
                                            Err(e) => {
                                                Log::stop(14, Some(format!("{}. Error: {}", &p, e)));
                                                return None;
                                            }
                                        }
                                    }
                                    None => {
                                        Log::stop(13, None);
                                        return None;
                                    }
                                }
                            } else {
                                (conf_file, conf, root_path) = Init::check_path(&exe_path)?;
                            }
                        }
                        // second parameter is empty
                        None => (conf_file, conf, root_path) = Init::check_path(&exe_path)?,
                    };
                } else {
                    conf = String::new();
                    conf_file = String::new();
                    root_path = String::new();
                }
            }
        };

        let conf = Init::load_conf(conf, mode != Mode::Help, name, version, desc)?;

        Some(Init {
            mode,
            conf,
            exe_file,
            exe_path,
            conf_file,
            root_path: Arc::new(root_path),
        })
    }

    /// Get the path to the current executable
    pub fn get_current_exe() -> Option<String> {
        let exe = match env::current_exe() {
            Ok(e) => match e.to_str() {
                Some(e) => {
                    if &e[..2] == r"\\" {
                        if &e[..4] == r"\\?\" {
                            e[4..].replace('\\', "/")
                        } else {
                            Log::stop(12, Some(e.to_string()));
                            return None;
                        }
                    } else {
                        e.replace('\\', "/")
                    }
                }
                None => {
                    Log::stop(11, Some(e.to_string_lossy().to_string()));
                    return None;
                }
            },
            Err(e) => {
                Log::stop(10, Some(e.to_string()));
                return None;
            }
        };
        Some(exe)
    }

    /// Try to read configuration file
    ///
    /// # Parameters
    ///
    /// * `path: &str` - path to file
    ///
    /// # Return
    ///
    /// * `Option::None` - file not found;
    /// * `Option::Some((String, String, String))` - success read configuration file:
    ///   * `turple.0` - path to configuration file
    ///   * `turple.1` - file contents
    ///   * `turple.2` - root folder
    fn check_path(path: &str) -> Option<(String, String, String)> {
        // configuration file was not found,
        // so we look for it in the folder with the current program
        let file = format!("{}/tiny.toml", path);
        match read_to_string(&file) {
            Ok(s) => {
                let root = match file.rfind('/') {
                    Some(i) => file[..i].to_owned(),
                    None => {
                        Log::stop(16, Some(file));
                        return None;
                    }
                };
                Some((file, s, root))
            }
            Err(e) => match e.kind() {
                ErrorKind::NotFound => {
                    // configuration file was not found,
                    // so we look for it in env::current_dir()
                    let file = match env::current_dir() {
                        Ok(f) => match f.to_str() {
                            Some(s) => format!("{}/tiny.toml", s.replace('\\', "/")),
                            None => {
                                Log::stop(15, None);
                                return None;
                            }
                        },
                        Err(_) => {
                            Log::stop(15, None);
                            return None;
                        }
                    };
                    match read_to_string(&file) {
                        Ok(s) => {
                            let root = match file.rfind('/') {
                                Some(i) => file[..i].to_owned(),
                                None => {
                                    Log::stop(16, Some(file));
                                    return None;
                                }
                            };
                            Some((file, s, root))
                        }
                        Err(_) => {
                            Log::stop(15, None);
                            None
                        }
                    }
                }
                _ => {
                    Log::stop(14, Some(format!("{}. Error: {}", &file, e)));
                    None
                }
            },
        }
    }

    /// Responsable for parsing data from configuration file.
    ///
    /// # Parameters
    ///
    /// * `text: String` - Configuration string;
    /// * `check_salt: bool` - Check salt for empty.
    /// * `name: &str` - Name of app.
    /// * `version: &str` - Version of app.
    /// * `desc: &str` - Description of app.
    ///
    /// # Return
    ///
    /// `Option<Config>` - Option of parsed configuration:
    ///   * `None` - Configuration contains errors;
    ///   * `Some(Config)` - is ok.
    fn load_conf(text: String, check_salt: bool, name: &str, version: &str, desc: &str) -> Option<Config> {
        let text = match text.parse::<Table>() {
            Ok(v) => v,
            Err(e) => {
                Log::stop(18, Some(e.to_string()));
                return None;
            }
        };

        let num_cpus = num_cpus::get();
        let mut num_connections = num_cpus * 3;
        let mut lang = "ua".to_owned();
        let mut log = "tiny.log".to_owned();
        let mut max = num_cpus;
        let mut bind_accept = AcceptAddr::Any;
        let mut bind = Addr::SocketAddr(SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 12500));
        let mut rpc_accept = AcceptAddr::IpAddr(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)));
        let mut rpc = Addr::SocketAddr(SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 12501));
        let mut session = "tinysession".to_owned();
        let mut salt = String::new();
        let mut stop = 0;
        let mut status = 0;
        let mut protocol = WorkerType::FastCGI;
        let mut prepare = BTreeMap::new();
        let mut db = DBConfig {
            engine: DBEngine::Pgsql,
            host: String::new(),
            port: None,
            name: String::new(),
            user: None,
            pwd: None,
            sslmode: false,
            max: num_connections,
        };
        if !text.is_empty() {
            for (key, value) in text {
                match &key[..] {
                    "lang" => {
                        if let Value::String(val) = value {
                            if val.len() != 2 {
                                Log::warning(51, Some(val));
                            } else {
                                lang = val;
                            }
                        } else {
                            Log::warning(51, Some(value.to_string()));
                        }
                    }
                    "log" => {
                        if let Value::String(val) = value {
                            log = val
                        } else {
                            Log::warning(61, Some(value.to_string()));
                        }
                    }
                    "max" => match value {
                        Value::String(s) => {
                            if &s != "auto" {
                                Log::warning(52, Some(s));
                            }
                        }
                        Value::Integer(i) => match usize::try_from(i) {
                            Ok(v) => {
                                if v > 0 {
                                    max = v;
                                    num_connections = v * 3;
                                } else {
                                    Log::warning(52, Some(v.to_string()));
                                }
                            }
                            Err(e) => {
                                Log::warning(52, Some(format!("{} {}", i, e)));
                            }
                        },
                        _ => {
                            Log::warning(52, Some(value.to_string()));
                        }
                    },
                    "bind_from" => {
                        if let Value::String(val) = value {
                            if val.is_empty() {
                                #[cfg(not(target_family = "windows"))]
                                {
                                    bind_accept = AcceptAddr::UDS;
                                }
                                #[cfg(target_family = "windows")]
                                {
                                    Log::warning(53, None);
                                }
                            } else if val == "any" {
                                bind_accept = AcceptAddr::Any;
                            } else {
                                match IpAddr::from_str(&val) {
                                    Ok(ip) => bind_accept = AcceptAddr::IpAddr(ip),
                                    Err(e) => {
                                        Log::warning(53, Some(format!("{} ({})", e, val)));
                                    }
                                };
                            }
                        } else {
                            Log::warning(53, Some(value.to_string()));
                        }
                    }
                    "bind" => {
                        if let Value::String(val) = value {
                            if val.contains(':') {
                                match SocketAddr::from_str(&val) {
                                    Ok(s) => bind = Addr::SocketAddr(s),
                                    Err(e) => {
                                        Log::warning(54, Some(format!("{} ({})", e, val)));
                                    }
                                }
                            } else {
                                #[cfg(target_family = "windows")]
                                {
                                    Log::warning(54, Some(val));
                                }
                                #[cfg(not(target_family = "windows"))]
                                if val.is_empty() || &val[..1] != "/" {
                                    Log::warning(54, None);
                                } else {
                                    bind = Addr::UDS(val);
                                }
                            }
                        } else {
                            Log::warning(54, Some(value.to_string()));
                        }
                    }
                    "rpc_from" => {
                        if let Value::String(val) = value {
                            if val.is_empty() {
                                #[cfg(not(target_family = "windows"))]
                                {
                                    rpc_accept = AcceptAddr::UDS;
                                }
                                #[cfg(target_family = "windows")]
                                {
                                    Log::warning(53, None);
                                }
                            } else if val == "any" {
                                rpc_accept = AcceptAddr::Any;
                            } else {
                                match IpAddr::from_str(&val) {
                                    Ok(ip) => rpc_accept = AcceptAddr::IpAddr(ip),
                                    Err(e) => {
                                        Log::warning(55, Some(format!("{} ({})", e, val)));
                                    }
                                };
                            }
                        } else {
                            Log::warning(55, Some(value.to_string()));
                        }
                    }
                    "rpc" => {
                        if let Value::String(val) = value {
                            if val.contains(':') {
                                match SocketAddr::from_str(&val) {
                                    Ok(s) => rpc = Addr::SocketAddr(s),
                                    Err(e) => {
                                        Log::warning(56, Some(format!("{} ({})", e, val)));
                                    }
                                }
                            } else {
                                #[cfg(target_family = "windows")]
                                {
                                    Log::warning(56, Some(val));
                                }
                                #[cfg(not(target_family = "windows"))]
                                if val.is_empty() || &val[..1] != "/" {
                                    Log::warning(56, None);
                                } else {
                                    rpc = Addr::UDS(val);
                                }
                            }
                        } else {
                            Log::warning(56, Some(value.to_string()));
                        }
                    }
                    "session" => {
                        if let Value::String(val) = value {
                            session = val;
                        } else {
                            Log::warning(71, Some(value.to_string()));
                        }
                    }
                    "salt" => {
                        if let Value::String(val) = value {
                            salt = val;
                            stop = fnv1a_64(format!("stop{}", &salt).as_bytes());
                            status = fnv1a_64(format!("status{}", &salt).as_bytes());
                        } else {
                            Log::warning(62, Some(value.to_string()));
                        }
                    }
                    "db_type" => {
                        if let Value::String(val) = value {
                            match val.as_bytes() {
                                b"postgresql" => db.engine = DBEngine::Pgsql,
                                b"mssql" => db.engine = DBEngine::Mssql,
                                _ => {
                                    Log::warning(73, Some(val.to_string()));
                                }
                            }
                        } else {
                            Log::warning(72, Some(value.to_string()));
                        }
                    }
                    "db_host" => {
                        if let Value::String(val) = value {
                            if !val.is_empty() {
                                db.host = val;
                            }
                        } else {
                            Log::warning(63, Some(value.to_string()));
                        }
                    }
                    "db_port" => {
                        if let Value::Integer(i) = value {
                            match u16::try_from(i) {
                                Ok(v) => {
                                    if v > 0 {
                                        db.port = Some(v);
                                    } else {
                                        Log::warning(57, Some(v.to_string()));
                                    }
                                }
                                Err(e) => {
                                    Log::warning(57, Some(format!("{} ({})", e, i)));
                                }
                            }
                        } else {
                            Log::warning(57, Some(value.to_string()));
                        }
                    }
                    "db_name" => {
                        if let Value::String(val) = value {
                            db.name = val;
                        } else {
                            Log::warning(64, Some(value.to_string()));
                        }
                    }
                    "db_user" => {
                        if let Value::String(val) = value {
                            db.user = Some(val);
                        } else {
                            Log::warning(65, Some(value.to_string()));
                        }
                    }
                    "db_pwd" => {
                        if let Value::String(val) = value {
                            db.pwd = Some(val);
                        } else {
                            Log::warning(66, Some(value.to_string()));
                        }
                    }
                    "sslmode" => {
                        if let Value::Boolean(val) = value {
                            db.sslmode = val;
                        } else {
                            Log::warning(67, Some(value.to_string()));
                        }
                    }
                    "db_max" => match value {
                        Value::String(s) => {
                            if &s == "auto" {
                                db.max = num_connections;
                            } else {
                                Log::warning(58, Some(s));
                            }
                        }
                        Value::Integer(i) => match usize::try_from(i) {
                            Ok(v) => {
                                if v > 0 {
                                    db.max = v;
                                } else {
                                    Log::warning(58, Some(v.to_string()));
                                }
                            }
                            Err(e) => {
                                Log::warning(58, Some(format!("{} {}", i, e)));
                            }
                        },
                        _ => {
                            Log::warning(58, Some(value.to_string()));
                        }
                    },
                    "protokol" => {
                        if let Value::String(val) = value {
                            protocol = match &val[..] {
                                "FastCGI" => WorkerType::FastCGI,
                                "SCGI" => WorkerType::Scgi,
                                "uWSGI" => WorkerType::Uwsgi,
                                "FastgRPCCGI" => WorkerType::Grpc,
                                "HTTP" => WorkerType::Http,
                                "WebSocket" => WorkerType::WebSocket,
                                _ => {
                                    Log::warning(60, Some(val.to_owned()));
                                    WorkerType::FastCGI
                                }
                            }
                        } else {
                            Log::warning(68, Some(value.to_string()));
                        }
                    }
                    "prepare" => {
                        if let Value::Table(list) = &value {
                            match db.engine {
                                DBEngine::Pgsql => Init::load_pgsql_prepare(list, &value, &mut prepare),
                                DBEngine::Mssql => Init::load_mssql_prepare(list, &value, &mut prepare),
                            }
                        } else {
                            Log::warning(69, Some(value.to_string()));
                        }
                    }
                    _ => {}
                }
            }
        }
        if db.host.is_empty() {
            Log::stop(59, None);
            return None;
        }
        if check_salt && salt.is_empty() {
            Log::stop(50, None);
            return None;
        }
        Log::set_path(log.clone());
        let conf = Config {
            name: name.to_owned(),
            desc: desc.to_owned(),
            version: version.to_owned(),
            lang: Arc::new(lang),
            log,
            max,
            bind_accept: Arc::new(bind_accept),
            bind,
            rpc_accept,
            rpc,
            session: Arc::new(session),
            salt: Arc::new(salt),
            db: Arc::new(db),
            stop,
            status,
            protocol,
            prepare: Arc::new(prepare),
        };
        Some(conf)
    }

    /// Load prepare for Postgresql
    fn load_pgsql_prepare(list: &Map<String, Value>, value: &Value, prepare: &mut BTreeMap<i64, DBPrepare>) {
        for (key, val) in list {
            if let Value::Table(item) = val {
                if let Some(sql) = item.get("query") {
                    let query = if let Value::String(q) = sql {
                        q
                    } else {
                        Log::warning(70, Some(value.to_string()));
                        continue;
                    };
                    let types = if let Some(types) = item.get("types") {
                        if let Value::Array(types) = types {
                            let mut vec = Vec::with_capacity(types.len());
                            for t in types {
                                if let Value::String(v) = t {
                                    match v.as_str() {
                                        "BOOL" => vec.push(tokio_postgres::types::Type::BOOL),
                                        "INT8" => vec.push(tokio_postgres::types::Type::INT8),
                                        "INT4" => vec.push(tokio_postgres::types::Type::INT4),
                                        "INT2" => vec.push(tokio_postgres::types::Type::INT2),
                                        "TEXT" => vec.push(tokio_postgres::types::Type::TEXT),
                                        "VARCHAR" => vec.push(tokio_postgres::types::Type::VARCHAR),
                                        "FLOAT4" => vec.push(tokio_postgres::types::Type::FLOAT4),
                                        "FLOAT8" => vec.push(tokio_postgres::types::Type::FLOAT8),
                                        "JSON" => vec.push(tokio_postgres::types::Type::JSON),
                                        "TIMESTAMPTZ" => vec.push(tokio_postgres::types::Type::TIMESTAMPTZ),
                                        "UUID" => vec.push(tokio_postgres::types::Type::UUID),
                                        "BYTEA" => vec.push(tokio_postgres::types::Type::BYTEA),
                                        _ => {
                                            Log::warning(70, Some(value.to_string()));
                                        }
                                    }
                                } else {
                                    Log::warning(70, Some(value.to_string()));
                                }
                            }
                            vec
                        } else {
                            Log::warning(70, Some(value.to_string()));
                            continue;
                        }
                    } else {
                        Vec::new()
                    };
                    prepare.insert(
                        fnv1a_64(key.as_bytes()),
                        DBPrepare {
                            query: query.to_owned(),
                            types: DBFieldType::Pgsql(types),
                        },
                    );
                } else {
                    Log::warning(70, Some(val.to_string()));
                }
            } else {
                Log::warning(70, Some(val.to_string()));
            }
        }
    }

    /// Load prepare for MsSql
    /// Load prepare for MsSql
    fn load_mssql_prepare(list: &Map<String, Value>, value: &Value, prepare: &mut BTreeMap<i64, DBPrepare>) {
        for (key, val) in list {
            if let Value::Table(item) = val {
                if let Some(sql) = item.get("query") {
                    let query = if let Value::String(q) = sql {
                        q
                    } else {
                        Log::warning(70, Some(value.to_string()));
                        continue;
                    };
                    let types = if let Some(types) = item.get("types") {
                        if let Value::Array(types) = types {
                            let mut vec = Vec::with_capacity(types.len());
                            for t in types {
                                if let Value::String(v) = t {
                                    //NVARCHAR(N_int), VARCHAR(N_int), VARBINARY(N_int)

                                    match v.as_str() {
                                        "TINYINT" | "BIGINT" | "INT" | "SMALLINT" | "NVARCHAR(MAX)" | "VARCHAR(MAX)"
                                        | "FLOAT" | "REAL" | "DATETIMEOFFSET" | "UNIQUEIDENTIFIER" | "VARBINARY(MAX)" => {
                                            vec.push(v.to_owned())
                                        }
                                        t => {
                                            if &t[t.len() - 1..t.len()] == ")" {
                                                if t.len() > 10 && &t[..8] == "VARCHAR(" {
                                                    match &t[8..t.len() - 1].parse::<i16>() {
                                                        Ok(i) => {
                                                            if *i <= 4000 {
                                                                vec.push(v.to_owned());
                                                            } else {
                                                                Log::warning(70, Some(value.to_string()));
                                                            }
                                                        }
                                                        Err(_) => {
                                                            Log::warning(70, Some(value.to_string()));
                                                        }
                                                    }
                                                } else if t.len() > 11 && &t[..9] == "NVARCHAR(" {
                                                    match &t[9..t.len() - 1].parse::<i16>() {
                                                        Ok(i) => {
                                                            if *i <= 4000 {
                                                                vec.push(v.to_owned());
                                                            } else {
                                                                Log::warning(70, Some(value.to_string()));
                                                            }
                                                        }
                                                        Err(_) => {
                                                            Log::warning(70, Some(value.to_string()));
                                                        }
                                                    }
                                                } else if t.len() > 12 && &t[..10] == "VARBINARY(" {
                                                    match &t[10..t.len() - 1].parse::<i16>() {
                                                        Ok(i) => {
                                                            if *i <= 4000 {
                                                                vec.push(v.to_owned());
                                                            } else {
                                                                Log::warning(70, Some(value.to_string()));
                                                            }
                                                        }
                                                        Err(_) => {
                                                            Log::warning(70, Some(value.to_string()));
                                                        }
                                                    }
                                                } else {
                                                    Log::warning(70, Some(value.to_string()));
                                                }
                                            } else {
                                                Log::warning(70, Some(value.to_string()));
                                            }
                                        }
                                    }
                                } else {
                                    Log::warning(70, Some(value.to_string()));
                                }
                            }
                            vec
                        } else {
                            Log::warning(70, Some(value.to_string()));
                            continue;
                        }
                    } else {
                        Vec::new()
                    };
                    prepare.insert(
                        fnv1a_64(key.as_bytes()),
                        DBPrepare {
                            query: query.to_owned(),
                            types: DBFieldType::Mssql(types),
                        },
                    );
                } else {
                    Log::warning(70, Some(val.to_string()));
                }
            } else {
                Log::warning(70, Some(val.to_string()));
            }
        }
    }
}
