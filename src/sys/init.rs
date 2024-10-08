use std::{
    env,
    fs::read_to_string,
    io::ErrorKind,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    str::FromStr,
    sync::Arc,
};

use toml::{Table, Value};

use crate::fnv1a_64;
use tiny_web_macro::fnv1a_64 as m_fnv1a_64;

use super::{log::Log, route::Route};

/// Responsible for the IP address that should be accepted.
///
/// # Values
///
/// * `Any` - Accepts any IP address;
/// * `IpAddr: IpAddr` - Accepts a specific IP address;
/// * `UDS` - Accepts only from Unix Domain Socket.
#[derive(Debug, Clone)]
pub(crate) enum AcceptAddr {
    /// Accepts any IP address.
    Any,
    /// Accepts a specific IP address;
    IpAddr(IpAddr),
    /// Accepts only from Unix Domain Socket.
    #[cfg(not(target_family = "windows"))]
    Uds,
}

/// Responsible for the IP address.
///
/// # Values
///
/// * `SocketAddr` - Accepts any socket address;
/// * `UDS: String` - Accepts only from Unix Domain Socket.
#[derive(Debug, Clone)]
pub(crate) enum Addr {
    /// Accepts any IP address.
    SocketAddr(SocketAddr),
    /// Accepts only from Unix Domain Socket.
    #[cfg(not(target_family = "windows"))]
    Uds(String),
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
pub(crate) struct Config {
    pub is_default: bool,
    /// Name server from env!("CARGO_PKG_NAME") primary project.
    pub name: String,
    /// Description server from env!("CARGO_PKG_DESCRIPTION") primary project.
    pub desc: String,
    /// Server version from env!("CARGO_PKG_VERSION") primary project.
    pub version: String,
    /// Default language.
    pub lang: Arc<String>,
    /// Number of work processes in async operations.
    pub max: usize,
    /// The address from which we accept working connections.
    pub bind_accept: Arc<AcceptAddr>,
    /// The address of the server that binds clients.
    pub bind: Addr,
    /// The address from which we accept connections for managing the server.
    pub rpc_accept: AcceptAddr,
    /// IP address from which to bind connections for managing the server.
    pub rpc: Arc<Addr>,
    /// Session key
    pub session: Arc<String>,
    /// Salt for a crypto functions.
    pub salt: Arc<String>,
    /// Database configuration.
    pub db: Arc<DBConfig>,
    /// Stop signal
    pub stop_signal: i64,
    /// Status signal
    pub status_signal: i64,
    /// Default controller for request "/" or default class or default action
    pub action_index: Arc<Route>,
    /// Default controller for 404 Not Found
    pub action_not_found: Arc<Route>,
    /// Default controller for error_route
    pub action_err: Arc<Route>,
}

impl Config {
    fn default(args: InitArgs) -> Config {
        let num_cpus = num_cpus::get();
        let bind_accept = AcceptAddr::Any;
        let bind = Addr::SocketAddr(SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 12500));
        let rpc_accept = AcceptAddr::IpAddr(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)));
        let rpc = Addr::SocketAddr(SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 12501));

        let db = DBConfig {
            host: String::new(),
            port: None,
            name: String::new(),
            user: None,
            pwd: None,
            sslmode: false,
            max: 0,
        };
        let log = "tiny.log".to_owned();
        Log::set_path(log.clone());

        let lang = if let Some(lang) = args.lang { lang } else { "en".to_owned() };

        Config {
            is_default: true,
            name: args.name.to_owned(),
            desc: args.desc.to_owned(),
            version: args.version.to_owned(),
            lang: Arc::new(lang),
            max: num_cpus,
            bind_accept: Arc::new(bind_accept),
            bind,
            rpc_accept,
            rpc: Arc::new(rpc),
            session: Arc::new("tinysession".to_owned()),
            salt: Arc::new("salt".to_owned()),
            db: Arc::new(db),
            stop_signal: m_fnv1a_64!("stopsalt"),
            status_signal: m_fnv1a_64!("statussalt"),
            action_index: Arc::new(Route::default_index()),
            action_not_found: Arc::new(Route::default_not_found()),
            action_err: Arc::new(Route::default_err()),
        }
    }
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
pub(crate) enum Mode {
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
pub(crate) struct Init {
    /// Running mode of server.
    pub mode: Mode,
    /// Server configuration.
    pub conf: Config,
    /// The full path to the folder where this executable file is located.
    pub exe_path: Arc<String>,
    /// The full path to this executable file.
    pub exe_file: String,
    /// The full path to the folder where the server was started.
    pub root_path: Arc<String>,
}

/// Startup parameters
#[derive(Debug, Clone)]
struct InitArgs<'a> {
    path: Option<String>, // -r start path for searching log file
    lang: Option<String>, // -l Default lang

    file: Option<&'a str>,
    check_salt: bool,
    name: &'a str,
    version: &'a str,
    desc: &'a str,
    allow_no_config: bool,
}

impl Init {
    /// Initializes the server configuration
    /// If the server is running in release mode, the configuration file must be in the same folder as the program.
    /// If the server is running in debug mode, the configuration file must be in the user's current folder.
    pub fn new(name: &str, version: &str, desc: &str, allow_no_config: bool) -> Option<Init> {
        let exe_file = Init::get_current_exe()?;

        #[cfg(not(debug_assertions))]
        let exe_path = match exe_file.rfind('/') {
            Some(i) => exe_file[..i].to_owned(),
            None => {
                Log::stop(16, Some(exe_file));
                return None;
            }
        };
        #[cfg(debug_assertions)]
        let exe_path = match env::current_dir() {
            Ok(path) => match path.to_str() {
                Some(path) => path.to_owned(),
                None => {
                    Log::stop(16, Some(format!("{:?}", path)));
                    return None;
                }
            },
            Err(e) => {
                Log::stop(16, Some(e.to_string()));
                return None;
            }
        };

        let mut args = env::args();

        let conf;
        let mode;
        let conf_file;
        let root_path;

        args.next();

        let mut iar = InitArgs {
            path: None,
            lang: None,
            file: None,
            check_salt: false,
            name,
            version,
            desc,
            allow_no_config,
        };

        // Check first parameter
        match args.next() {
            // first parameter is empty
            None => {
                mode = Mode::Help;
                (conf_file, root_path) = Init::check_path(&exe_path, allow_no_config)?;
                iar.file = conf_file.as_deref();

                conf = Init::load_conf(iar)?;
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
                    iar.check_salt = true;

                    while let Some(command) = args.next() {
                        if command == "-r" {
                            match args.next() {
                                Some(value) => {
                                    iar.path = Some(value);
                                }
                                None => break,
                            }
                        } else if command == "-l" {
                            match args.next() {
                                Some(value) => {
                                    if value.len() != 2 {
                                        Log::warning(81, Some(value));
                                    } else {
                                        iar.lang = Some(value);
                                    }
                                }
                                None => break,
                            }
                        }
                    }

                    if let Some(path) = &iar.path {
                        let file = format!("{}/tiny.toml", path);
                        root_path = match file.rfind('/') {
                            Some(i) => file[..i].to_owned(),
                            None => {
                                Log::stop(16, Some(file));
                                return None;
                            }
                        };
                        iar.file = Some(&root_path);
                    } else {
                        (conf_file, root_path) = Init::check_path(&exe_path, allow_no_config)?;
                        iar.file = conf_file.as_deref();
                    }
                    conf = Init::load_conf(iar)?;
                } else {
                    root_path = String::new();
                    conf = Config::default(iar);
                }
            }
        };

        Some(Init {
            mode,
            conf,
            exe_file,
            exe_path: Arc::new(exe_path),
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
    ///   * `turple.1` - root folder
    fn check_path(path: &str, allow_no_config: bool) -> Option<(Option<String>, String)> {
        let file = format!("{}/tiny.toml", path);
        match read_to_string(&file) {
            Ok(_) => Some((Some(file), path.to_owned())),
            Err(e) => match e.kind() {
                ErrorKind::NotFound => {
                    if allow_no_config {
                        Some((None, path.to_owned()))
                    } else {
                        Log::stop(15, Some(format!("{}. Error: {}", &file, e)));
                        None
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
    /// * `iar: InitArgs` - Startup parameters
    ///
    /// # Return
    ///
    /// `Option<Config>` - Option of parsed configuration:
    ///   * `None` - Configuration contains errors;
    ///   * `Some(Config)` - is ok.
    fn load_conf(args: InitArgs) -> Option<Config> {
        let file = match args.file {
            Some(file) => file,
            None => {
                if args.allow_no_config {
                    return Some(Config::default(args));
                } else {
                    Log::stop(14, None);
                    return None;
                }
            }
        };

        let text = match read_to_string(file) {
            Ok(text) => text,
            Err(e) => {
                Log::stop(14, Some(format!("{}. Error: {}", &file, e)));
                return None;
            }
        };

        let text = match text.parse::<Table>() {
            Ok(v) => v,
            Err(e) => {
                Log::stop(18, Some(e.to_string()));
                return None;
            }
        };

        let num_cpus = num_cpus::get();
        let mut num_connections = num_cpus * 3;
        let mut lang = "en".to_owned();
        let mut max = num_cpus;
        let mut bind_accept = AcceptAddr::Any;
        let mut bind = Addr::SocketAddr(SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 12500));
        let mut rpc_accept = AcceptAddr::IpAddr(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)));
        let mut rpc = Addr::SocketAddr(SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 12501));
        let mut session = "tinysession".to_owned();
        let mut salt = String::new();
        let mut stop_signal = 0;
        let mut status_signal = 0;

        let mut db = DBConfig {
            host: String::new(),
            port: None,
            name: String::new(),
            user: None,
            pwd: None,
            sslmode: false,
            max: num_connections,
        };
        let mut action_index = Route::default_index();
        let mut action_not_found = Route::default_not_found();
        let mut action_err = Route::default_err();

        if let Some(value) = text.get("log") {
            if let Value::String(val) = value {
                Log::set_path(val.clone());
            } else {
                Log::warning(61, Some(value.to_string()));
            }
        };

        for (key, value) in text {
            match key.as_str() {
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
                                bind_accept = AcceptAddr::Uds;
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
                                bind = Addr::Uds(val);
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
                                rpc_accept = AcceptAddr::Uds;
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
                                rpc = Addr::Uds(val);
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
                        stop_signal = fnv1a_64(format!("stop{}", &salt).as_bytes());
                        status_signal = fnv1a_64(format!("status{}", &salt).as_bytes());
                    } else {
                        Log::warning(62, Some(value.to_string()));
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
                "action_index" => {
                    if let Value::String(val) = &value {
                        if let Some(route) = Route::parse(val) {
                            action_index = route;
                        } else {
                            Log::warning(75, Some(value.to_string()));
                        }
                    } else {
                        Log::warning(74, Some(value.to_string()));
                    }
                }
                "action_not_found" => {
                    if let Value::String(val) = &value {
                        if let Some(route) = Route::parse(val) {
                            action_not_found = route;
                        } else {
                            Log::warning(77, Some(value.to_string()));
                        }
                    } else {
                        Log::warning(76, Some(value.to_string()));
                    }
                }
                "action_err" => {
                    if let Value::String(val) = &value {
                        if let Some(route) = Route::parse(val) {
                            action_err = route;
                        } else {
                            Log::warning(79, Some(value.to_string()));
                        }
                    } else {
                        Log::warning(78, Some(value.to_string()));
                    }
                }
                _ => {}
            }
        }

        if db.host.is_empty() {
            Log::stop(59, None);
            return None;
        }
        if args.check_salt && salt.is_empty() {
            Log::stop(50, None);
            return None;
        }
        if let Some(l) = args.lang {
            lang = l;
        }

        let conf = Config {
            is_default: false,
            name: args.name.to_owned(),
            desc: args.desc.to_owned(),
            version: args.version.to_owned(),
            lang: Arc::new(lang),
            max,
            bind_accept: Arc::new(bind_accept),
            bind,
            rpc_accept,
            rpc: Arc::new(rpc),
            session: Arc::new(session),
            salt: Arc::new(salt),
            db: Arc::new(db),
            stop_signal,
            status_signal,
            action_index: Arc::new(action_index),
            action_not_found: Arc::new(action_not_found),
            action_err: Arc::new(action_err),
        };
        Some(conf)
    }
}
