use std::{
    collections::HashMap,
    io::ErrorKind,
    process,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};

use tokio::{
    fs::remove_file,
    net::{TcpListener, TcpStream, UnixListener, UnixStream},
    runtime::Builder,
    sync::{oneshot, Mutex},
    task::JoinHandle,
    time,
};

#[cfg(any(feature = "html-reload", feature = "lang-reload"))]
use tokio::sync::RwLock;

use crate::{
    fnv1a_64, log,
    sys::{
        net::{
            stream::{Listener, Socket},
            worker::{Worker, WorkerData},
        },
        stat::stat::Stat,
        web::action::ModuleMap,
    },
};

#[cfg(any(feature = "html-static", feature = "html-reload"))]
use crate::sys::web::html::Html;

#[cfg(any(feature = "lang-static", feature = "lang-reload"))]
use crate::sys::web::{lang::Lang, lang::LangParam};

#[cfg(any(feature = "session-memory", feature = "session-file", feature = "session-db"))]
use crate::sys::web::session::{SessionArg, SessionLoader};

#[cfg(any(feature = "pgsql", feature = "mssql"))]
use crate::sys::db::adapter::DB;

use super::{
    arg::Arg,
    init::{AutoCount, Init, SIGNAL_TIMEOUT},
};

pub(crate) struct Run;

#[cfg(feature = "cache")]
use crate::sys::web::cache::Cache;

impl Run {
    pub(crate) fn start(args: Arg, init: Init, engine: ModuleMap) -> Result<(), ()> {
        let mut builder = Builder::new_multi_thread();
        builder.thread_name(format!("{} {}", init.name, init.version));
        if let AutoCount::Count(worker_threads) = init.proc.worker_threads {
            builder.worker_threads(worker_threads);
        }
        if let AutoCount::Count(event_interval) = init.proc.event_interval {
            builder.event_interval(event_interval);
        }
        if let AutoCount::Count(global_queue_interval) = init.proc.global_queue_interval {
            builder.global_queue_interval(global_queue_interval);
        }
        if let AutoCount::Count(max_blocking_threads) = init.proc.max_blocking_threads {
            builder.max_blocking_threads(max_blocking_threads);
        }
        if let AutoCount::Count(max_io_events_per_tick) = init.proc.max_io_events_per_tick {
            builder.max_io_events_per_tick(max_io_events_per_tick);
        }
        if let AutoCount::Count(thread_keep_alive) = init.proc.thread_keep_alive {
            builder.thread_keep_alive(Duration::from_millis(thread_keep_alive as u64));
        }
        if let AutoCount::Count(thread_stack_size) = init.proc.thread_stack_size {
            builder.thread_stack_size(thread_stack_size);
        }

        let runtime = match builder.enable_all().build() {
            Ok(r) => r,
            Err(_e) => {
                log!(stop, 0, "{}", _e);
                return Err(());
            }
        };
        // Start runtime
        runtime.block_on(async move {
            let mon = Arc::new(Stat::new());
            let stop = Arc::new(AtomicBool::new(false));
            let init = Arc::new(init);

            let mon_clone = Arc::clone(&mon);
            let stop_clone = Arc::clone(&stop);
            let init_clone = Arc::clone(&init);

            let mut res = Ok(());
            if let Ok(listener) = Run::listen(stop_clone, mon_clone, init_clone, args, engine).await {
                if Run::listen_rpc(stop, listener, mon, Arc::clone(&init)).await.is_ok() {
                    #[cfg(not(target_family = "windows"))]
                    if let Socket::Unix(uds) = &init.net.rpc {
                        if let Err(e) = remove_file(uds).await {
                            if e.kind() != ErrorKind::NotFound {
                                log!(stop, 0, "{}", e);
                                res = Err(());
                            }
                        }
                    }
                };

                #[cfg(not(target_family = "windows"))]
                if let Socket::Unix(uds) = &init.net.bind {
                    if let Err(e) = remove_file(uds).await {
                        if e.kind() != ErrorKind::NotFound {
                            log!(stop, 0, "{}", e);
                            res = Err(());
                        }
                    }
                }
            }
            res
        })
    }

    async fn listen(stop: Arc<AtomicBool>, mon: Arc<Stat>, init: Arc<Init>, _args: Arg, engine: ModuleMap) -> Result<JoinHandle<()>, ()> {
        let bind = match &init.net.bind {
            Socket::Inet(addr) => match TcpListener::bind(addr).await {
                Ok(i) => Listener::TcpListener(i),
                Err(_e) => {
                    log!(stop, 0, "{}", _e);
                    return Err(());
                }
            },
            #[cfg(not(target_family = "windows"))]
            Socket::Unix(uds) => match UnixListener::bind(uds) {
                Ok(i) => Listener::UnixListener(i),
                Err(_e) => {
                    log!(stop, 0, "{}", _e);
                    return Err(());
                }
            },
        };
        Ok(tokio::spawn(async move {
            let ip = init.net.bind_from;
            let workers: Arc<Mutex<HashMap<u64, JoinHandle<()>>>> =
                Arc::new(Mutex::new(HashMap::with_capacity(init.proc.worker_threads.value() + 1)));
            let engine = Arc::new(engine);
            #[cfg(any(feature = "pgsql", feature = "mssql"))]
            let db = match DB::new(Arc::clone(&init.db)).await {
                Ok(db) => Arc::new(db),
                Err(_) => return,
            };
            #[cfg(any(feature = "html-static", feature = "html-reload"))]
            let html = match Html::new(Arc::clone(&_args.root)).await {
                Ok(html) => {
                    #[cfg(feature = "html-static")]
                    {
                        Arc::new(html)
                    }
                    #[cfg(feature = "html-reload")]
                    {
                        Arc::new(RwLock::new(html))
                    }
                }
                Err(_) => {
                    log!(stop, 0);
                    return;
                }
            };

            #[cfg(feature = "cache")]
            let cache = Arc::new(Cache::new());

            #[cfg(any(feature = "lang-static", feature = "lang-reload"))]
            let param = LangParam {
                root: Arc::clone(&_args.root),
                default_lang: Arc::clone(&init.web.lang),
                #[cfg(feature = "session-db")]
                db: Arc::clone(&db),
            };

            #[cfg(any(feature = "lang-static", feature = "lang-reload"))]
            let lang = match Lang::new(param).await {
                Ok(lang) => {
                    #[cfg(feature = "lang-reload")]
                    {
                        Arc::new(RwLock::new(lang))
                    }
                    #[cfg(feature = "lang-static")]
                    {
                        Arc::new(lang)
                    }
                }
                Err(_) => {
                    log!(stop, 0);
                    return;
                }
            };

            #[cfg(any(feature = "session-memory", feature = "session-file", feature = "session-db"))]
            let sess_arg = SessionArg {
                session_key: Arc::clone(&init.web.session_key),
                #[cfg(any(feature = "session-memory", feature = "session-file"))]
                session_path: Arc::clone(&init.web.session_path),
                #[cfg(feature = "session-db")]
                db: Arc::clone(&db),
            };
            #[cfg(any(feature = "session-memory", feature = "session-file", feature = "session-db"))]
            let session = match SessionLoader::start(sess_arg).await {
                Ok(session) => Arc::new(session),
                Err(_) => {
                    log!(stop, 0);
                    return;
                }
            };

            #[cfg(feature = "https")]
            let acceptor = match Worker::load_cert(Arc::clone(&_args.root)) {
                Ok(acceptor) => acceptor,
                Err(_e) => {
                    log!(stop, 507, "{}", _e);
                    return;
                }
            };
            loop {
                let (stream, _ip) = match bind.accept(&ip).await {
                    Ok(stream) => stream,
                    Err(_e) => {
                        log!(stop, 0, "{}", _e);
                        continue;
                    }
                };
                if stop.load(Ordering::SeqCst) {
                    break;
                }
                let id = mon.worker.fetch_add(1, Ordering::SeqCst);
                let (tx, rx) = oneshot::channel();
                let mon = Arc::clone(&mon);
                let engine = Arc::clone(&engine);
                #[cfg(any(feature = "http", feature = "https"))]
                let root = Arc::clone(&_args.root);
                let salt = Arc::clone(&init.web.salt);
                let index = Arc::clone(&init.web.index);
                let not_found = init.web.not_found.clone();
                #[cfg(feature = "https")]
                let acceptor = Arc::clone(&acceptor);
                #[cfg(any(feature = "html-static", feature = "html-reload"))]
                let html = Arc::clone(&html);
                #[cfg(any(feature = "session-memory", feature = "session-file", feature = "session-db"))]
                let session = Arc::clone(&session);
                #[cfg(any(feature = "lang-static", feature = "lang-reload"))]
                let lang = Arc::clone(&lang);
                #[cfg(any(feature = "pgsql", feature = "mssql"))]
                let db = Arc::clone(&db);
                #[cfg(any(feature = "mail-sendmail", feature = "mail-smtp", feature = "mail-file"))]
                let mail = Arc::clone(&init.mail);
                #[cfg(feature = "cache")]
                let cache = Arc::clone(&cache);

                let worker = tokio::spawn(async move {
                    let data = WorkerData {
                        #[cfg(feature = "debug-vvv")]
                        id,
                        mon,
                        engine,
                        #[cfg(any(feature = "http", feature = "https"))]
                        root,
                        salt,
                        #[cfg(any(feature = "http", feature = "https"))]
                        ip: _ip,
                        index,
                        not_found,
                        #[cfg(any(feature = "pgsql", feature = "mssql"))]
                        db,
                        #[cfg(feature = "https")]
                        acceptor,
                        #[cfg(any(feature = "html-static", feature = "html-reload"))]
                        html,
                        #[cfg(any(feature = "session-memory", feature = "session-file", feature = "session-db"))]
                        session,
                        #[cfg(any(feature = "lang-static", feature = "lang-reload"))]
                        lang,
                        #[cfg(any(feature = "mail-sendmail", feature = "mail-smtp", feature = "mail-file"))]
                        mail,
                        #[cfg(feature = "cache")]
                        cache,
                    };
                    Worker::run(stream, data).await;
                    if let Err(_i) = tx.send(id) {
                        log!(error, 0, "{}", _i);
                    }
                });
                let workers_clone = Arc::clone(&workers);
                tokio::spawn(async move {
                    workers_clone.lock().await.insert(id, worker);
                    if let Ok(id) = rx.await {
                        workers_clone.lock().await.remove(&id);
                    };
                });
                if stop.load(Ordering::Relaxed) {
                    break;
                }
            }
            for (_, handle) in workers.lock().await.iter() {
                handle.abort()
            }
            #[cfg(any(feature = "session-memory", feature = "session-file", feature = "session-db"))]
            let _ = session.stop().await;
            for (_, handle) in workers.lock().await.iter_mut() {
                if let Err(e) = handle.await {
                    if !e.is_cancelled() {
                        log!(stop, 0, "{}", e);
                    }
                }
            }
        }))
    }

    async fn listen_rpc(stop: Arc<AtomicBool>, listener: JoinHandle<()>, mon: Arc<Stat>, init: Arc<Init>) -> Result<(), ()> {
        let rpc = match init.net.rpc.bind().await {
            Ok(listener) => listener,
            Err(_e) => {
                log!(stop, 0, "{}", _e);
                Run::send_stop(stop, listener, init).await;
                return Err(());
            }
        };
        let stop_signal = fnv1a_64(format!("stop{}", init.web.salt).as_bytes());
        let status_signal = fnv1a_64(format!("status{}", init.web.salt).as_bytes());

        loop {
            let (mut stream, _) = match rpc.accept(&init.net.rpc_from).await {
                Ok(stream) => stream,
                Err(_e) => {
                    log!(warning, 0, "{}", _e);
                    continue;
                }
            };

            if stop.load(Ordering::SeqCst) {
                break;
            }

            // read key
            let signal = match stream.signal_read_i64().await {
                Ok(signal) => signal,
                Err(_e) => {
                    log!(warning, 0, "{}", _e);
                    continue;
                }
            };
            if signal == status_signal {
                log!(info, 0);
                let pid = process::id() as u64;
                if let Err(_e) = stream.signal_write_u64(pid).await {
                    log!(stop, 0, "{}", _e);
                }

                let len = mon.get_number();
                let last = mon.get_last();
                let last = if last > 0 { last.to_string() } else { "empty".to_owned() };
                let online = mon.get_online();
                let total = mon.get_total();
                let status = format!(
                    r#"
The system is working ...
Number of workers: {}.
Last worker id: {}.
Number of online requests: {}.
Number of total requests: {}.
"#,
                    len, last, online, total
                );
                if let Err(_e) = stream.signal_write_str(&status).await {
                    log!(stop, 0, "{}", _e);
                }
            } else if signal == stop_signal {
                log!(info, 0);
                Run::send_stop(stop, listener, init).await;
                let pid = process::id() as u64;
                if let Err(_e) = stream.signal_write_u64(pid).await {
                    log!(stop, 0, "{}", _e);
                }
                break;
            } else {
                log!(warning, 0, "{}", signal.to_string());
            }
        }
        Ok(())
    }

    async fn send_stop(stop: Arc<AtomicBool>, listener: JoinHandle<()>, init: Arc<Init>) {
        stop.store(true, Ordering::SeqCst);
        match &init.net.bind {
            Socket::Inet(addr) => {
                if let Ok(Err(res)) = time::timeout(Duration::from_millis(SIGNAL_TIMEOUT), TcpStream::connect(addr)).await {
                    if res.kind() == ErrorKind::ConnectionRefused {
                    } else {
                        log!(warning, 0);
                    }
                }
            }
            Socket::Unix(uds) => {
                if let Ok(Err(res)) = time::timeout(Duration::from_millis(SIGNAL_TIMEOUT), UnixStream::connect(uds)).await {
                    if res.kind() == ErrorKind::ConnectionRefused {
                    } else {
                        log!(warning, 0);
                    }
                }
            }
        }
        if let Err(_e) = listener.await {
            log!(stop, 0, "{}", _e);
        }
    }
}
