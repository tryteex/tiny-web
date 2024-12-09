#[cfg(any(feature = "session-memory", feature = "session-file"))]
use std::path::PathBuf;
use std::{
    fs::read_to_string,
    io::{Error, ErrorKind},
    net::{IpAddr, SocketAddr},
    path::Path,
    sync::Arc,
};

use toml::Table;

use crate::{fnv1a_64, sys::net::stream::Socket};

#[cfg(any(feature = "debug-v", feature = "debug-vv", feature = "debug-vvv"))]
use crate::sys::log::{InitLog, Log};

/// Час очикування для сигналів
pub(crate) const SIGNAL_TIMEOUT: u64 = 2000;
/// Час очикування для завершення роботи
pub(crate) const SIGNAL_TIMEOUT_WAIT: u64 = 30000;

#[derive(Debug, Clone)]
pub(crate) enum AutoCount<T>
where
    T: Ord + Copy + Default,
{
    Auto,
    Count(T),
}

impl<T> AutoCount<T>
where
    T: Ord + Copy + Default,
{
    pub fn value(&self) -> T {
        match &self {
            AutoCount::Auto => T::default(),
            AutoCount::Count(value) => *value,
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct Async {
    pub worker_threads: AutoCount<usize>,
    pub event_interval: AutoCount<u32>,
    pub global_queue_interval: AutoCount<u32>,
    pub max_blocking_threads: AutoCount<usize>,
    pub max_io_events_per_tick: AutoCount<usize>,
    pub thread_keep_alive: AutoCount<u32>,
    pub thread_stack_size: AutoCount<usize>,
}

#[cfg(any(feature = "pgsql", feature = "mssql"))]
#[derive(Debug, Clone)]
pub(crate) struct DBConfig {
    pub host: String,
    pub port: Option<u16>,
    pub name: String,
    pub user: Option<String>,
    pub pwd: Option<String>,
    pub ssl: bool,
    pub max: AutoCount<usize>,
}

#[derive(Debug)]
pub(crate) struct Web {
    #[cfg(any(feature = "lang-static", feature = "lang-reload"))]
    pub lang: Arc<String>,
    pub salt: Arc<String>,
    #[cfg(any(feature = "session-memory", feature = "session-file", feature = "session-db"))]
    pub session_key: Arc<String>,
    #[cfg(any(feature = "session-memory", feature = "session-file"))]
    pub session_path: Arc<PathBuf>,
    pub index: Arc<[i64; 3]>,
    pub not_found: Option<Arc<[i64; 3]>>,
}

#[derive(Debug)]
pub(crate) struct Net {
    pub bind: Socket,
    pub bind_from: IpAddr,
    pub rpc: Socket,
    pub rpc_from: IpAddr,
}

#[cfg(feature = "mail-smtp")]
#[derive(Debug)]
pub(crate) enum Tls {
    None,
    /// STARTTLS
    Start,
    /// SSL/TLS
    Ssl,
}

#[cfg(feature = "mail-smtp")]
#[derive(Debug, PartialEq)]
pub(crate) enum Auth {
    None,
    /// PLAIN
    Plain,
    /// LOGIN
    Login,
    /// XOAUTH2
    XOAuth2,
}

#[cfg(any(feature = "mail-sendmail", feature = "mail-smtp", feature = "mail-file"))]
#[derive(Debug)]
pub(crate) struct MailConfig {
    #[cfg(feature = "mail-sendmail")]
    pub sendmail: String,
    #[cfg(feature = "mail-file")]
    pub path: String,
    #[cfg(feature = "mail-smtp")]
    pub server: String,
    #[cfg(feature = "mail-smtp")]
    pub port: u16,
    #[cfg(feature = "mail-smtp")]
    pub tls: Tls,
    #[cfg(feature = "mail-smtp")]
    pub auth: Auth,
    #[cfg(feature = "mail-smtp")]
    pub user: Option<String>,
    #[cfg(feature = "mail-smtp")]
    pub pwd: Option<String>,
}

#[derive(Debug)]
pub(crate) struct Init {
    pub name: String,
    pub version: String,
    pub desc: String,
    pub web: Web,
    pub net: Net,
    pub proc: Async,
    #[cfg(any(feature = "pgsql", feature = "mssql"))]
    pub db: Arc<DBConfig>,
    #[cfg(any(feature = "mail-sendmail", feature = "mail-smtp", feature = "mail-file"))]
    pub mail: Arc<MailConfig>,
}

impl Init {
    pub(crate) fn parse(name: String, version: String, desc: String, root: &Path) -> Result<Init, Error> {
        let mut path = root.to_path_buf();
        path.push("init.toml");
        let content = match read_to_string(path) {
            Ok(content) => content,
            Err(e) => {
                #[cfg(any(feature = "debug-v", feature = "debug-vv", feature = "debug-vvv"))]
                Log::init(InitLog::Path(root.to_owned()));
                return Err(e);
            }
        };
        let res: Table = match toml::from_str(&content) {
            Ok(res) => res,
            Err(e) => {
                #[cfg(any(feature = "debug-v", feature = "debug-vv", feature = "debug-vvv"))]
                Log::init(InitLog::Path(root.to_owned()));
                return Err(Error::new(ErrorKind::InvalidData, e));
            }
        };

        #[cfg(any(feature = "debug-v", feature = "debug-vv", feature = "debug-vvv"))]
        match res.get("log") {
            Some(val) => match val.as_str() {
                Some(v) => Log::init(InitLog::File(v.to_string())),
                None => Log::init(InitLog::None),
            },
            None => Log::init(InitLog::None),
        };
        let mut web = None;
        let mut net = None;
        let mut proc = None;
        #[cfg(any(feature = "pgsql", feature = "mssql"))]
        let mut db = None;
        #[cfg(any(feature = "mail-sendmail", feature = "mail-smtp", feature = "mail-file"))]
        let mut mail = None;

        for (key, val) in res {
            match key.as_str() {
                "web" => {
                    if let Some(list) = val.as_table() {
                        #[cfg(any(feature = "lang-static", feature = "lang-reload"))]
                        let mut lang = None;
                        let mut salt = None;
                        #[cfg(any(feature = "session-db", feature = "session-file", feature = "session-memory"))]
                        let mut session_key = None;
                        let mut index = None;
                        let mut not_found = None;
                        #[cfg(any(feature = "session-memory", feature = "session-file"))]
                        let mut session_path = None;

                        for (key, val) in list {
                            match key.as_str() {
                                #[cfg(any(feature = "lang-static", feature = "lang-reload"))]
                                "lang" => lang = val.as_str(),
                                "salt" => salt = val.as_str(),
                                #[cfg(any(feature = "session-db", feature = "session-file", feature = "session-memory"))]
                                "session" => session_key = val.as_str(),
                                #[cfg(any(feature = "session-memory", feature = "session-file"))]
                                "session_path" => session_path = val.as_str(),
                                "index" => {
                                    if let Some(vec) = val.as_array() {
                                        let module = match unsafe { vec.get_unchecked(0) }.as_str() {
                                            Some(s) if !s.is_empty() => s.to_owned(),
                                            _ => continue,
                                        };

                                        let class = match unsafe { vec.get_unchecked(1) }.as_str() {
                                            Some(s) if !s.is_empty() => s.to_owned(),
                                            _ => continue,
                                        };

                                        let action = match unsafe { vec.get_unchecked(2) }.as_str() {
                                            Some(s) if !s.is_empty() => s.to_owned(),
                                            _ => continue,
                                        };

                                        index =
                                            Some([fnv1a_64(module.as_bytes()), fnv1a_64(class.as_bytes()), fnv1a_64(action.as_bytes())]);
                                    }
                                }
                                "not_found" => {
                                    if let Some(vec) = val.as_array() {
                                        let module = match unsafe { vec.get_unchecked(0) }.as_str() {
                                            Some(s) if !s.is_empty() => s.to_owned(),
                                            _ => continue,
                                        };

                                        let class = match unsafe { vec.get_unchecked(1) }.as_str() {
                                            Some(s) if !s.is_empty() => s.to_owned(),
                                            _ => continue,
                                        };

                                        let action = match unsafe { vec.get_unchecked(2) }.as_str() {
                                            Some(s) if !s.is_empty() => s.to_owned(),
                                            _ => continue,
                                        };

                                        not_found = Some(Arc::new([
                                            fnv1a_64(module.as_bytes()),
                                            fnv1a_64(class.as_bytes()),
                                            fnv1a_64(action.as_bytes()),
                                        ]));
                                    }
                                }
                                _ => {}
                            }
                        }
                        #[cfg(any(feature = "lang-static", feature = "lang-reload"))]
                        let lang = lang
                            .filter(|l| !l.is_empty())
                            .ok_or_else(|| {
                                Error::new(
                                    ErrorKind::InvalidData,
                                    "Параметр [web] lang обов'язковий. Must consist of two characters according to ISO 639-1.",
                                )
                            })?
                            .to_owned();
                        let salt = salt
                            .filter(|s| !s.is_empty())
                            .ok_or_else(|| {
                                Error::new(ErrorKind::InvalidData, "Параметр [web] salt обов'язковий. Повинен бути не пустим рядком.")
                            })?
                            .to_owned();
                        #[cfg(any(feature = "session-db", feature = "session-file", feature = "session-memory"))]
                        let session_key = session_key
                            .filter(|s| !s.is_empty())
                            .ok_or_else(|| {
                                Error::new(ErrorKind::InvalidData, "Параметр [web] session обов'язковий. Повинен бути не пустим рядком.")
                            })?
                            .to_owned();
                        #[cfg(any(feature = "session-memory", feature = "session-file"))]
                        let session_path = session_path
                            .filter(|s| !s.is_empty())
                            .ok_or_else(|| {
                                Error::new(
                                    ErrorKind::InvalidData,
                                    "Параметр [web] session_path обов'язковий. Повинен бути не пустим рядком.",
                                )
                            })?
                            .to_owned();
                        let index = index.ok_or_else(|| {
                            Error::new(
                                ErrorKind::InvalidData,
                                r#"Параметр [web] index. Повинен бути масив із трьох рядків ["module", "class", "action"]"#,
                            )
                        })?;

                        web = Some(Web {
                            #[cfg(any(feature = "lang-static", feature = "lang-reload"))]
                            lang: Arc::new(lang),
                            salt: Arc::new(salt),
                            #[cfg(any(feature = "session-db", feature = "session-file", feature = "session-memory"))]
                            session_key: Arc::new(session_key),
                            #[cfg(any(feature = "session-memory", feature = "session-file"))]
                            session_path: Arc::new(PathBuf::from(session_path)),
                            index: Arc::new(index),
                            not_found,
                        });
                    }
                }
                "net" => {
                    if let Some(list) = val.as_table() {
                        let mut bind = None;
                        let mut bind_from = None;
                        let mut rpc = None;
                        let mut rpc_from = None;
                        for (key, val) in list {
                            match key.as_str() {
                                "bind" => {
                                    if let Some(addr) = val.as_str() {
                                        bind = if addr.starts_with('/') {
                                            Some(Socket::Unix(addr.to_owned()))
                                        } else {
                                            addr.parse::<SocketAddr>().map(Socket::Inet).ok()
                                        };
                                    }
                                }
                                "bind_from" => {
                                    if let Some(addr) = val.as_str() {
                                        bind_from = addr.parse::<IpAddr>().ok();
                                    }
                                }
                                "rpc" => {
                                    if let Some(addr) = val.as_str() {
                                        rpc = if addr.starts_with('/') {
                                            Some(Socket::Unix(addr.to_owned()))
                                        } else {
                                            addr.parse::<SocketAddr>().map(Socket::Inet).ok()
                                        };
                                    }
                                }
                                "rpc_from" => {
                                    if let Some(addr) = val.as_str() {
                                        rpc_from = addr.parse::<IpAddr>().ok();
                                    }
                                }
                                _ => {}
                            }
                        }
                        let bind = bind.ok_or_else(|| {
                            Error::new(
                                ErrorKind::InvalidData,
                                r#"Параметр [net] bind. Повинен починатися з "/" якщо це UDS чи "ip:port" для звичайного сокета"#,
                            )
                        })?;
                        let bind_from = bind_from
                            .ok_or_else(|| Error::new(ErrorKind::InvalidData, r#"Параметр [net] bind_from. Повинена бути IP адреса"#))?;
                        let rpc = rpc.ok_or_else(|| {
                            Error::new(
                                ErrorKind::InvalidData,
                                r#"Параметр [net] rpc. Повинен починатися з "/" якщо це UDS чи "ip:port" для звичайного сокета"#,
                            )
                        })?;
                        let rpc_from = rpc_from
                            .ok_or_else(|| Error::new(ErrorKind::InvalidData, r#"Параметр [net] rpc_from. Повинена бути IP адреса"#))?;
                        net = Some(Net { bind, bind_from, rpc, rpc_from })
                    }
                }
                "async" => {
                    if let Some(list) = val.as_table() {
                        let mut worker_threads = None;
                        let mut event_interval = None;
                        let mut global_queue_interval = None;
                        let mut max_blocking_threads = None;
                        let mut max_io_events_per_tick = None;
                        let mut thread_keep_alive = None;
                        let mut thread_stack_size = None;
                        for (key, val) in list {
                            match key.as_str() {
                                "worker_threads" => {
                                    val.as_str()
                                        .map(|v| {
                                            if v == "auto" {
                                                worker_threads = Some(AutoCount::Auto);
                                            }
                                        })
                                        .or_else(|| {
                                            val.as_integer().and_then(|v| {
                                                usize::try_from(v).ok().map(|s| {
                                                    worker_threads = Some(AutoCount::Count(s));
                                                })
                                            })
                                        });
                                }
                                "event_interval" => {
                                    val.as_str()
                                        .map(|v| {
                                            if v == "auto" {
                                                event_interval = Some(AutoCount::Auto);
                                            }
                                        })
                                        .or_else(|| {
                                            val.as_integer().and_then(|v| {
                                                u32::try_from(v).ok().map(|s| {
                                                    event_interval = Some(AutoCount::Count(s));
                                                })
                                            })
                                        });
                                }
                                "global_queue_interval" => {
                                    val.as_str()
                                        .map(|v| {
                                            if v == "auto" {
                                                global_queue_interval = Some(AutoCount::Auto);
                                            }
                                        })
                                        .or_else(|| {
                                            val.as_integer().and_then(|v| {
                                                u32::try_from(v).ok().map(|s| {
                                                    global_queue_interval = Some(AutoCount::Count(s));
                                                })
                                            })
                                        });
                                }
                                "max_blocking_threads" => {
                                    val.as_str()
                                        .map(|v| {
                                            if v == "auto" {
                                                max_blocking_threads = Some(AutoCount::Auto);
                                            }
                                        })
                                        .or_else(|| {
                                            val.as_integer().and_then(|v| {
                                                usize::try_from(v).ok().map(|s| {
                                                    max_blocking_threads = Some(AutoCount::Count(s));
                                                })
                                            })
                                        });
                                }
                                "max_io_events_per_tick" => {
                                    val.as_str()
                                        .map(|v| {
                                            if v == "auto" {
                                                max_io_events_per_tick = Some(AutoCount::Auto);
                                            }
                                        })
                                        .or_else(|| {
                                            val.as_integer().and_then(|v| {
                                                usize::try_from(v).ok().map(|s| {
                                                    max_io_events_per_tick = Some(AutoCount::Count(s));
                                                })
                                            })
                                        });
                                }
                                "thread_keep_alive" => {
                                    val.as_str()
                                        .map(|v| {
                                            if v == "auto" {
                                                thread_keep_alive = Some(AutoCount::Auto);
                                            }
                                        })
                                        .or_else(|| {
                                            val.as_integer().and_then(|v| {
                                                u32::try_from(v).ok().map(|s| {
                                                    thread_keep_alive = Some(AutoCount::Count(s));
                                                })
                                            })
                                        });
                                }
                                "thread_stack_size" => {
                                    val.as_str()
                                        .map(|v| {
                                            if v == "auto" {
                                                thread_stack_size = Some(AutoCount::Auto);
                                            }
                                        })
                                        .or_else(|| {
                                            val.as_integer().and_then(|v| {
                                                usize::try_from(v).ok().map(|s| {
                                                    thread_stack_size = Some(AutoCount::Count(s));
                                                })
                                            })
                                        });
                                }
                                _ => {}
                            }
                        }
                        let worker_threads = worker_threads.ok_or_else(|| {
                            Error::new(
                                ErrorKind::InvalidData,
                                r#"Параметр [async] worker_threads обов'язковий. Повинен бути рядок "auto" чи значення usize"#,
                            )
                        })?;
                        let event_interval = event_interval.ok_or_else(|| {
                            Error::new(
                                ErrorKind::InvalidData,
                                r#"Параметр [async] event_interval обов'язковий. Повинен бути рядок "auto" чи значення u32"#,
                            )
                        })?;
                        let global_queue_interval = global_queue_interval.ok_or_else(|| {
                            Error::new(
                                ErrorKind::InvalidData,
                                r#"Параметр [async] global_queue_interval обов'язковий. Повинен бути рядок "auto" чи значення u32"#,
                            )
                        })?;
                        let max_blocking_threads = max_blocking_threads.ok_or_else(|| {
                            Error::new(
                                ErrorKind::InvalidData,
                                r#"Параметр [async] max_blocking_threads обов'язковий. Повинен бути рядок "auto" чи значення usize"#,
                            )
                        })?;
                        let max_io_events_per_tick = max_io_events_per_tick.ok_or_else(|| {
                            Error::new(
                                ErrorKind::InvalidData,
                                r#"Параметр [async] max_io_events_per_tick обов'язковий. Повинен бути рядок "auto" чи значення usize"#,
                            )
                        })?;
                        let thread_keep_alive = thread_keep_alive.ok_or_else(|| {
                            Error::new(
                                ErrorKind::InvalidData,
                                r#"Параметр [async] thread_keep_alive обов'язковий. Повинен бути рядок "auto" чи значення u32"#,
                            )
                        })?;
                        let thread_stack_size = thread_stack_size.ok_or_else(|| {
                            Error::new(
                                ErrorKind::InvalidData,
                                r#"Параметр [async] thread_stack_size обов'язковий. Повинен бути рядок "auto" чи значення usize"#,
                            )
                        })?;
                        proc = Some(Async {
                            worker_threads,
                            event_interval,
                            global_queue_interval,
                            max_blocking_threads,
                            max_io_events_per_tick,
                            thread_keep_alive,
                            thread_stack_size,
                        })
                    }
                }
                #[cfg(any(feature = "pgsql", feature = "mssql"))]
                "db" => {
                    if let Some(list) = val.as_table() {
                        let mut host = None;
                        let mut port = None;
                        let mut name = None;
                        let mut user = None;
                        let mut pwd = None;
                        let mut ssl = None;
                        let mut max = None;
                        for (key, val) in list {
                            match key.as_str() {
                                "host" => host = val.as_str(),
                                "port" => port = val.as_integer(),
                                "name" => name = val.as_str(),
                                "user" => user = val.as_str(),
                                "pwd" => pwd = val.as_str(),
                                "ssl" => ssl = val.as_bool(),
                                "max" => {
                                    val.as_str()
                                        .map(|v| {
                                            if v == "auto" {
                                                max = Some(AutoCount::Auto);
                                            }
                                        })
                                        .or_else(|| {
                                            val.as_integer().and_then(|v| {
                                                usize::try_from(v).ok().map(|s| {
                                                    max = Some(AutoCount::Count(s));
                                                })
                                            })
                                        });
                                }
                                _ => {}
                            }
                        }
                        let host = host
                            .filter(|s| !s.is_empty())
                            .ok_or_else(|| {
                                Error::new(ErrorKind::InvalidData, "Параметр [db] host обов'язковий. Повинен бути не пустим рядком.")
                            })?
                            .to_owned();
                        let port = match port {
                            Some(v) => match u16::try_from(v) {
                                Ok(v) => Some(v),
                                Err(_) => return Err(Error::new(ErrorKind::InvalidData, "Параметр [db] port. Повинен бути u16.")),
                            },
                            None => None,
                        };
                        let name = name
                            .filter(|s| !s.is_empty())
                            .ok_or_else(|| {
                                Error::new(ErrorKind::InvalidData, "Параметр [db] name обов'язковий. Повинен бути не пустим рядком.")
                            })?
                            .to_owned();
                        let user = user.filter(|v| !v.is_empty()).map(|v| v.to_owned());
                        let pwd = pwd.filter(|v| !v.is_empty()).map(|v| v.to_owned());
                        let ssl = ssl.ok_or_else(|| {
                            Error::new(ErrorKind::InvalidData, "Параметр [db] ssl обов'язковий. Повинен бути true чи false")
                        })?;
                        let max = max.ok_or_else(|| {
                            Error::new(
                                ErrorKind::InvalidData,
                                r#"Параметр [db] max обов'язковий. Повинен бути рядок "auto" чи значення usize"#,
                            )
                        })?;
                        db = Some(DBConfig { host, port, name, user, pwd, ssl, max });
                    }
                }
                #[cfg(any(feature = "mail-sendmail", feature = "mail-smtp", feature = "mail-file"))]
                "mail" => {
                    if let Some(list) = val.as_table() {
                        #[cfg(feature = "mail-sendmail")]
                        let mut sendmail = None;
                        #[cfg(feature = "mail-file")]
                        let mut path = None;
                        #[cfg(feature = "mail-smtp")]
                        let mut server = None;
                        #[cfg(feature = "mail-smtp")]
                        let mut port = None;
                        #[cfg(feature = "mail-smtp")]
                        let mut tls = None;
                        #[cfg(feature = "mail-smtp")]
                        let mut auth = None;
                        #[cfg(feature = "mail-smtp")]
                        let mut user = None;
                        #[cfg(feature = "mail-smtp")]
                        let mut pwd = None;
                        for (key, val) in list {
                            match key.as_str() {
                                #[cfg(feature = "mail-sendmail")]
                                "sendmail" => sendmail = val.as_str(),
                                #[cfg(feature = "mail-file")]
                                "path" => path = val.as_str(),
                                #[cfg(feature = "mail-smtp")]
                                "server" => server = val.as_str(),
                                #[cfg(feature = "mail-smtp")]
                                "port" => port = val.as_integer(),
                                #[cfg(feature = "mail-smtp")]
                                "tls" => tls = val.as_str(),
                                #[cfg(feature = "mail-smtp")]
                                "auth" => auth = val.as_str(),
                                #[cfg(feature = "mail-smtp")]
                                "user" => user = val.as_str(),
                                #[cfg(feature = "mail-smtp")]
                                "pwd" => pwd = val.as_str(),
                                _ => {}
                            }
                        }
                        #[cfg(feature = "mail-sendmail")]
                        let sendmail = sendmail
                            .filter(|s| !s.is_empty())
                            .ok_or_else(|| {
                                Error::new(ErrorKind::InvalidData, "Параметр [mail] sendmail обов'язковий. Повинен бути не пустим рядком.")
                            })?
                            .to_owned();
                        #[cfg(feature = "mail-file")]
                        let path = path
                            .filter(|s| !s.is_empty())
                            .ok_or_else(|| {
                                Error::new(ErrorKind::InvalidData, "Параметр [mail] path обов'язковий. Повинен бути не пустим рядком.")
                            })?
                            .to_owned();
                        #[cfg(feature = "mail-smtp")]
                        let server = server
                            .filter(|s| !s.is_empty())
                            .ok_or_else(|| {
                                Error::new(ErrorKind::InvalidData, "Параметр [mail] server обов'язковий. Повинен бути не пустим рядком.")
                            })?
                            .to_owned();
                        #[cfg(feature = "mail-smtp")]
                        let port = port
                            .and_then(|p| u16::try_from(p).ok())
                            .ok_or_else(|| Error::new(ErrorKind::InvalidData, "Параметр [mail] port обов'язковий. Повинен бути u16."))?;
                        #[cfg(feature = "mail-smtp")]
                        let tls = tls
                            .and_then(|tls| match tls {
                                "NONE" => Some(Tls::None),
                                "STARTTLS" => Some(Tls::Start),
                                "SSL/TLS" => Some(Tls::Ssl),
                                _ => None,
                            })
                            .ok_or_else(|| {
                                Error::new(
                                    ErrorKind::InvalidData,
                                    "Параметр [mail] tls обов'язковий. Може бути тільки: NONE, STARTTLS, SSL/TLS.",
                                )
                            })?;
                        #[cfg(feature = "mail-smtp")]
                        let auth = auth
                            .and_then(|auth: &str| match auth {
                                "NONE" => Some(Auth::None),
                                "PLAIN" => Some(Auth::Plain),
                                "LOGIN" => Some(Auth::Login),
                                "XOAUTH2" => Some(Auth::XOAuth2),
                                _ => None,
                            })
                            .ok_or_else(|| {
                                Error::new(
                                    ErrorKind::InvalidData,
                                    "Параметр [mail] auth обов'язковий. Може бути тільки: NONE, PLAIN, LOGIN, XOAUTH2.",
                                )
                            })?;
                        #[cfg(feature = "mail-smtp")]
                        let user = user.filter(|v| !v.is_empty()).map(|v| v.to_owned());
                        #[cfg(feature = "mail-smtp")]
                        let pwd = pwd.filter(|v| !v.is_empty()).map(|v| v.to_owned());

                        mail = Some(MailConfig {
                            #[cfg(feature = "mail-sendmail")]
                            sendmail,
                            #[cfg(feature = "mail-file")]
                            path,
                            #[cfg(feature = "mail-smtp")]
                            server,
                            #[cfg(feature = "mail-smtp")]
                            port,
                            #[cfg(feature = "mail-smtp")]
                            tls,
                            #[cfg(feature = "mail-smtp")]
                            auth,
                            #[cfg(feature = "mail-smtp")]
                            user,
                            #[cfg(feature = "mail-smtp")]
                            pwd,
                        });
                    }
                }
                _ => {}
            }
        }
        let web = match web {
            Some(web) => web,
            None => return Err(Error::new(ErrorKind::InvalidData, "Секція [web] не знайдена.")),
        };
        let net = match net {
            Some(net) => net,
            None => return Err(Error::new(ErrorKind::InvalidData, "Секція [net] не знайдена.")),
        };
        let proc = match proc {
            Some(proc) => proc,
            None => return Err(Error::new(ErrorKind::InvalidData, "Секція [async] не знайдена.")),
        };
        #[cfg(any(feature = "pgsql", feature = "mssql"))]
        let db = match db {
            Some(db) => Arc::new(db),
            None => return Err(Error::new(ErrorKind::InvalidData, "Секція [db] не знайдена.")),
        };
        #[cfg(any(feature = "mail-sendmail", feature = "mail-smtp", feature = "mail-file"))]
        let mail = match mail {
            Some(mail) => Arc::new(mail),
            None => return Err(Error::new(ErrorKind::InvalidData, "Секція [mail] не знайдена.")),
        };

        Ok(Init {
            name,
            version,
            desc,
            web,
            net,
            proc,
            #[cfg(any(feature = "pgsql", feature = "mssql"))]
            db,
            #[cfg(any(feature = "mail-sendmail", feature = "mail-smtp", feature = "mail-file"))]
            mail,
        })
    }
}
