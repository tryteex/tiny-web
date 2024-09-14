use std::{
    collections::BTreeMap,
    io::ErrorKind,
    process,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    runtime::Builder,
    sync::{oneshot, Mutex},
    task::JoinHandle,
    time,
};

#[cfg(debug_assertions)]
use tokio::sync::RwLock;

use super::{
    action::ActMap,
    cache::CacheSys,
    dbs::adapter::DB,
    html::Html,
    init::{AcceptAddr, Addr, Config, Init},
    lang::Lang,
    log::Log,
    mail::Mail,
    worker::{Worker, WorkerData},
};

/// Server management
pub(crate) struct Go;

impl Go {
    /// Run server in tokio runtime
    pub fn run(init: &Init, func: &impl Fn() -> ActMap) {
        let runtime = match Builder::new_multi_thread().worker_threads(init.conf.max).enable_all().build() {
            Ok(r) => r,
            Err(e) => {
                Log::stop(1, Some(e.to_string()));
                return;
            }
        };

        // Start tokio runtime
        runtime.block_on(async move {
            let stop = Arc::new(AtomicBool::new(false));

            // Start listening to incoming clients
            if let Some(main) = Go::listen(init, Arc::clone(&stop), func).await {
                if !main.is_finished() {
                    Go::listen_rpc(&init.conf, stop, main).await;
                }
            };
        });
    }

    /// Listens for clients on the bind port
    ///
    /// # Return
    ///
    /// `None` - The server cannot listen on the bind port
    /// `Some(JoinHandle)` - Handler for main tokio thread
    async fn listen(init: &Init, stop: Arc<AtomicBool>, func: &impl Fn() -> ActMap) -> Option<JoinHandle<()>> {
        // Open bind port
        let bind = match &init.conf.bind {
            Addr::SocketAddr(a) => TcpListener::bind(a).await,
            #[cfg(not(target_family = "windows"))]
            Addr::Uds(s) => TcpListener::bind(s).await,
        };
        let bind = match bind {
            Ok(i) => i,
            Err(e) => {
                Log::stop(500, Some(e.to_string()));
                return None;
            }
        };
        let root_path = Arc::clone(&init.root_path);
        let db = Arc::clone(&init.conf.db);
        let lang = Arc::clone(&init.conf.lang);
        let bind_accept = Arc::clone(&init.conf.bind_accept);
        let session_key = Arc::clone(&init.conf.session);
        let salt = Arc::clone(&init.conf.salt);
        let engine_data = func();
        let protocol = init.conf.protocol.clone();

        let action_index = Arc::clone(&init.conf.action_index);
        let action_not_found = Arc::clone(&init.conf.action_not_found);
        let action_err = Arc::clone(&init.conf.action_err);

        let max = db.max;
        let mut db = DB::new(max, db).await?;

        let signal_stop =
            if db.in_use() { None } else { Some((Arc::clone(&init.conf.rpc), init.conf.stop_signal, Arc::clone(&init.exe_path))) };

        let main = tokio::spawn(async move {
            #[cfg(not(debug_assertions))]
            let lang = Arc::new(Lang::new(&root_path, &lang, &mut db).await);

            #[cfg(debug_assertions)]
            let lang = Arc::new(RwLock::new(Lang::new(Arc::clone(&root_path), &lang, &mut db).await));

            #[cfg(not(debug_assertions))]
            let html = Arc::new(Html::new(&root_path).await);
            #[cfg(debug_assertions)]
            let html = Arc::new(RwLock::new(Html::new(&root_path).await));

            let cache = CacheSys::new().await;
            let engine = Arc::new(engine_data);

            let db = Arc::new(db);
            let session_key = Arc::clone(&session_key);
            let salt = Arc::clone(&salt);
            let mail = Arc::new(Mutex::new(Mail::new(Arc::clone(&db)).await));
            let protocol = Arc::new(protocol);

            let action_index = Arc::clone(&action_index);
            let action_not_found = Arc::clone(&action_not_found);
            let action_err = Arc::clone(&action_err);

            let signal_stop = match signal_stop {
                Some((ref rpc, stop, ref path)) => Some((Arc::clone(rpc), stop, Arc::clone(path))),
                None => None,
            };

            // Started (accepted) threads
            let handles = Arc::new(Mutex::new(BTreeMap::new()));
            let mut counter: u64 = 0;
            loop {
                let (stream, addr) = match bind.accept().await {
                    Ok((stream, addr)) => (stream, addr),
                    Err(e) => {
                        // Check no critical error
                        match e.kind() {
                            ErrorKind::ConnectionRefused
                            | ErrorKind::ConnectionReset
                            | ErrorKind::Interrupted
                            | ErrorKind::TimedOut
                            | ErrorKind::WouldBlock
                            | ErrorKind::UnexpectedEof => continue,
                            _ => {
                                Log::stop(504, Some(e.to_string()));
                                break;
                            }
                        }
                    }
                };
                // Check stop signal
                if stop.load(Ordering::Relaxed) {
                    break;
                }

                let (tx, rx) = oneshot::channel();

                let lang = Arc::clone(&lang);
                let html = Arc::clone(&html);
                let cache = Arc::clone(&cache);
                let engine = Arc::clone(&engine);
                let db = Arc::clone(&db);
                let bind_accept = Arc::clone(&bind_accept);
                let session_key = Arc::clone(&session_key);
                let salt = Arc::clone(&salt);
                let mail = Arc::clone(&mail);
                let protocol = Arc::clone(&protocol);
                let action_index = Arc::clone(&action_index);
                let action_not_found = Arc::clone(&action_not_found);
                let action_err = Arc::clone(&action_err);
                let signal_stop = match signal_stop {
                    Some((ref rpc, stop, ref path)) => Some((Arc::clone(rpc), stop, Arc::clone(path))),
                    None => None,
                };

                let handle = tokio::spawn(async move {
                    let id = counter;
                    if let Err(e) = stream.set_nodelay(true) {
                        Log::warning(506, Some(e.to_string()));
                        return;
                    }
                    // Check accept ip
                    if let AcceptAddr::IpAddr(ip) = &*bind_accept {
                        if &addr.ip() != ip {
                            Log::warning(501, Some(addr.ip().to_string()));
                            return;
                        }
                    }

                    // Starting one main thread from the client connection
                    let data = WorkerData {
                        engine,
                        lang,
                        html,
                        cache,
                        db,
                        session_key,
                        salt,
                        mail,
                        action_index,
                        action_not_found,
                        action_err,
                        stop: signal_stop,
                    };
                    Worker::run(stream, data, protocol).await;
                    if let Err(i) = tx.send(id) {
                        Log::error(502, Some(i.to_string()));
                    }
                });
                let handles_clone = Arc::clone(&handles);
                // Handle the termination of the main thread from the client connection
                tokio::spawn(async move {
                    handles_clone.lock().await.insert(counter, handle);
                    if let Ok(id) = rx.await {
                        handles_clone.lock().await.remove(&id);
                    };
                });
                counter += 1;
                // Check stop signal
                if stop.load(Ordering::Relaxed) {
                    break;
                }
            }

            for (_, handle) in handles.lock().await.iter() {
                handle.abort()
            }
            for (_, handle) in handles.lock().await.iter_mut() {
                if let Err(e) = handle.await {
                    Log::stop(505, Some(e.to_string()));
                }
            }
        });
        Some(main)
    }

    /// Listens for rcp connection
    async fn listen_rpc(conf: &Config, stop: Arc<AtomicBool>, main: JoinHandle<()>) {
        // Open rpc port
        let rpc = match conf.rpc.as_ref() {
            Addr::SocketAddr(a) => TcpListener::bind(a).await,
            #[cfg(not(target_family = "windows"))]
            Addr::Uds(s) => TcpListener::bind(s).await,
        };
        let rpc = match rpc {
            Ok(i) => i,
            Err(e) => {
                Log::stop(202, Some(e.to_string()));
                return;
            }
        };
        loop {
            // accept rpc
            let (mut stream, addr) = rpc.accept().await.unwrap();
            if let AcceptAddr::IpAddr(ip) = conf.rpc_accept {
                if addr.ip() != ip {
                    Log::warning(203, Some(addr.ip().to_string()));
                    continue;
                }
            }
            if let Err(e) = stream.set_nodelay(true) {
                Log::warning(219, Some(e.to_string()));
                continue;
            }
            // read stop key
            let signal = stream.read_i64();
            let signal = match time::timeout(Duration::from_secs(2), signal).await {
                Ok(signal) => match signal {
                    Ok(signal) => signal,
                    Err(e) => {
                        Log::warning(205, Some(e.to_string()));
                        continue;
                    }
                },
                Err(_) => {
                    Log::warning(204, None);
                    continue;
                }
            };
            if signal == conf.stop_signal {
                // set stop
                stop.store(true, Ordering::Relaxed);
                // push current thread id
                Log::info(207, None);
                let pid = process::id() as u64;
                if let Err(e) = stream.write_u64(pid).await {
                    Log::warning(215, Some(e.to_string()));
                }
                // send stop signal
                Go::send_stop(&conf.bind).await;
                // wait all threads stop
                if let Err(e) = main.await {
                    Log::stop(220, Some(e.to_string()));
                }
                break;
            } else if signal == conf.status_signal {
                Log::info(227, None);
                let pid = process::id() as u64;
                if let Err(e) = stream.write_u64(pid).await {
                    Log::warning(215, Some(e.to_string()));
                } else if let Err(e) = stream.write_all("Working...".as_bytes()).await {
                    Log::warning(215, Some(e.to_string()));
                }
            } else {
                Log::warning(206, Some(signal.to_string()));
            }
        }
    }

    /// Send stop signal to bind port
    async fn send_stop(addr: &Addr) {
        #[allow(clippy::infallible_destructuring_match)]
        match addr {
            Addr::SocketAddr(s) => match time::timeout(Duration::from_secs(1), TcpStream::connect(s)).await {
                Ok(stream) => {
                    if let Err(e) = stream {
                        Log::warning(222, Some(e.to_string()));
                    }
                }
                Err(_) => {
                    Log::warning(221, None);
                }
            },
            #[cfg(not(target_family = "windows"))]
            Addr::Uds(s) => match time::timeout(Duration::from_secs(1), TcpStream::connect(s)).await {
                Ok(stream) => {
                    if let Err(e) = stream {
                        Log::warning(222, Some(e.to_string()));
                    }
                }
                Err(_) => {
                    Log::warning(221, None);
                }
            },
        }
    }
}
