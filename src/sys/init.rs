use std::{
    env,
    fs::read_to_string,
    io::ErrorKind,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    str::FromStr,
};

use crate::fnv1a_64;

use super::log::Log;

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
/// * `host: String` - Host of database;
/// * `port: Option<u16>` - Port of database;
/// * `name: String` - Name of database;
/// * `user: Option<String>` - Database user;
/// * `pwd: Option<String>` - Password of database user;
/// * `sslmode: bool` - Use for sslmode=require when connecting to the database;
/// * `max: SysCount` - The number of connections that will be used in the pool;
/// * `zone: String` - Time zone to init database.
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
    /// Time zone to init database.
    pub zone: Option<String>,
}

/// Describes the server configuration.
///
/// # Values
///
/// * `version: String` - Server version from env!("CARGO_PKG_VERSION") primary project;
/// * `lang: String` - Default language;
/// * `log: String` - Path to log file;
/// * `max: SysCount` - Number of work processes in async operations;
/// * `bind_accept: AcceptAddr` - The address from which we accept working connections;
/// * `bind_ip: Addr` - The address of the server that binds clients;
/// * `rpc_accept: AcceptAddr` - The address from which we accept connections for managing the server;
/// * `rpc_ip: Addr` - IP address from which to bind connections for managing the server;
/// * `salt: String` - Salt for a crypto functions.
/// * `db: Option<DBConfig>` - Database configuration.
/// * `stop: u64` - Stop signal.
#[derive(Debug, Clone)]
pub struct Config {
    /// Name server from env!("CARGO_PKG_NAME") primary project.
    pub name: String,
    /// Description server from env!("CARGO_PKG_DESCRIPTION") primary project.
    pub desc: String,
    /// Server version from env!("CARGO_PKG_VERSION") primary project.
    pub version: String,
    /// Default language.
    pub lang: String,
    /// Path to log file.
    pub log: String,
    /// Number of work processes in async operations.
    pub max: usize,
    /// The address from which we accept working connections.
    pub bind_accept: AcceptAddr,
    /// The address of the server that binds clients.
    pub bind: Addr,
    /// The address from which we accept connections for managing the server.
    pub rpc_accept: AcceptAddr,
    /// IP address from which to bind connections for managing the server.
    pub rpc: Addr,
    /// Salt for a crypto functions.
    pub salt: String,
    /// Database configuration.
    pub db: DBConfig,
    /// Stop signal
    pub stop: i64,
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
    pub root_path: String,
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
                                        let file = format!("{}/tiny.conf", p);
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
                    conf = "".to_owned();
                    conf_file = "".to_owned();
                    root_path = "".to_owned();
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
            root_path,
        })
    }

    /// Get the path to the current executable
    pub fn get_current_exe() -> Option<String> {
        let exe = match env::current_exe() {
            Ok(e) => match e.to_str() {
                Some(e) => {
                    if &e[..2] == "\\\\" {
                        if &e[..4] == "\\\\?\\" {
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
        let file = format!("{}/tiny.conf", path);
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
                            Some(s) => format!("{}/tiny.conf", s.replace('\\', "/")),
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
        let num_cpus = num_cpus::get();
        let mut num_connections = num_cpus * 3;
        let mut conf = Config {
            name: name.to_owned(),
            desc: desc.to_owned(),
            version: version.to_owned(),
            lang: "ua".to_owned(),
            log: "tiny.log".to_owned(),
            max: num_cpus,
            bind_accept: AcceptAddr::Any,
            bind: Addr::SocketAddr(SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 12500)),
            rpc_accept: AcceptAddr::IpAddr(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1))),
            rpc: Addr::SocketAddr(SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 12501)),
            salt: "".to_owned(),
            db: DBConfig {
                host: "".to_owned(),
                port: None,
                name: "".to_owned(),
                user: None,
                pwd: None,
                sslmode: false,
                max: num_connections,
                zone: None,
            },
            stop: 0,
        };
        if !text.is_empty() {
            for part in text.split('\n') {
                let line = part.trim();
                if !line.is_empty() && &line[..1] != "#" && line.contains('=') {
                    let vals: Vec<&str> = line.splitn(2, '=').collect();
                    if vals.len() == 2 {
                        let param = vals[0].trim();
                        let val = vals[1].trim();
                        match param {
                            "lang" => {
                                if val.len() != 2 {
                                    Log::warning(51, Some(val.to_owned()));
                                } else {
                                    conf.lang = val.to_owned();
                                }
                            }
                            "log" => conf.log = val.to_owned(),
                            "max" => {
                                if val != "auto" {
                                    match val.parse::<usize>() {
                                        Ok(v) => {
                                            if v > 0 {
                                                conf.max = v;
                                                num_connections = v * 3;
                                            } else {
                                                Log::warning(52, Some(val.to_owned()));
                                            }
                                        }
                                        Err(e) => Log::warning(52, Some(format!("{} ({})", e, val))),
                                    }
                                }
                            }
                            "bind_from" => {
                                if val.is_empty() {
                                    #[cfg(not(target_family = "windows"))]
                                    {
                                        conf.bind_accept = AcceptAddr::UDS;
                                    }
                                    #[cfg(target_family = "windows")]
                                    {
                                        conf.bind_accept = AcceptAddr::Any;
                                    }
                                } else if val == "any" {
                                    conf.bind_accept = AcceptAddr::Any;
                                } else {
                                    match IpAddr::from_str(val) {
                                        Ok(ip) => conf.bind_accept = AcceptAddr::IpAddr(ip),
                                        Err(e) => Log::warning(53, Some(format!("{} ({})", e, val))),
                                    };
                                }
                            }
                            "bind" => {
                                if val.contains(':') {
                                    match SocketAddr::from_str(val) {
                                        Ok(s) => conf.bind = Addr::SocketAddr(s),
                                        Err(e) => Log::warning(54, Some(format!("{} ({})", e, val))),
                                    }
                                } else {
                                    #[cfg(target_family = "windows")]
                                    {
                                        Log::warning(54, Some(val.to_owned()));
                                    }
                                    #[cfg(not(target_family = "windows"))]
                                    if val.is_empty() || &val[..1] != "/" {
                                        Log::warning(54, None);
                                    } else {
                                        conf.bind = Addr::UDS(val.to_owned());
                                    }
                                }
                            }
                            "rpc_from" => {
                                if val.is_empty() {
                                    #[cfg(not(target_family = "windows"))]
                                    {
                                        conf.rpc_accept = AcceptAddr::UDS;
                                    }
                                    #[cfg(target_family = "windows")]
                                    {
                                        conf.rpc_accept = AcceptAddr::Any;
                                    }
                                } else if val == "any" {
                                    conf.rpc_accept = AcceptAddr::Any;
                                } else {
                                    match IpAddr::from_str(val) {
                                        Ok(ip) => conf.rpc_accept = AcceptAddr::IpAddr(ip),
                                        Err(e) => Log::warning(55, Some(format!("{} ({})", e, val))),
                                    };
                                }
                            }
                            "rpc" => {
                                if val.contains(':') {
                                    match SocketAddr::from_str(val) {
                                        Ok(s) => conf.rpc = Addr::SocketAddr(s),
                                        Err(e) => Log::warning(56, Some(format!("{} ({})", e, val))),
                                    }
                                } else {
                                    #[cfg(target_family = "windows")]
                                    {
                                        Log::warning(56, Some(val.to_owned()));
                                    }
                                    #[cfg(not(target_family = "windows"))]
                                    if val.is_empty() || &val[..1] != "/" {
                                        Log::warning(56, None);
                                    } else {
                                        conf.rpc = Addr::UDS(val.to_owned());
                                    }
                                }
                            }
                            "salt" => {
                                conf.salt = val.to_owned();
                                conf.stop = fnv1a_64(&format!("stop{}", &conf.salt));
                            }
                            "db_host" => {
                                if !val.is_empty() {
                                    conf.db.host = val.to_owned();
                                }
                            }
                            "db_port" => match val.parse::<u16>() {
                                Ok(v) => {
                                    if v > 0 {
                                        conf.db.port = Some(v);
                                    } else {
                                        Log::warning(57, Some(val.to_owned()));
                                    }
                                }
                                Err(e) => Log::warning(57, Some(format!("{} ({})", e, val))),
                            },
                            "db_name" => conf.db.name = val.to_owned(),
                            "db_user" => conf.db.user = Some(val.to_owned()),
                            "db_pwd" => conf.db.pwd = Some(val.to_owned()),
                            "sslmode" => {
                                if val == "require" {
                                    conf.db.sslmode = true;
                                }
                            }
                            "max_db" => {
                                if val == "auto" {
                                    conf.db.max = num_connections;
                                } else {
                                    match val.parse::<usize>() {
                                        Ok(v) => {
                                            if v > 0 {
                                                conf.db.max = v;
                                            } else {
                                                Log::warning(58, Some(val.to_owned()));
                                            }
                                        }
                                        Err(e) => Log::warning(58, Some(format!("{} ({})", e, val))),
                                    }
                                }
                            }
                            "zone" => {
                                if !val.is_empty() {
                                    conf.db.zone = Some(val.to_owned())
                                }
                            }
                            _ => {}
                        };
                    }
                }
            }
        }
        if conf.db.host.is_empty() {
            Log::stop(59, None);
            return None;
        }
        if check_salt && conf.salt.is_empty() {
            Log::stop(50, None);
            return None;
        }
        Log::set_path(conf.log.clone());
        Some(conf)
    }
}
