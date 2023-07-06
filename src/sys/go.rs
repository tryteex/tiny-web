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

use super::{
    action::ActMap,
    cache::Cache,
    html::Html,
    init::{AcceptAddr, Addr, Config, Init},
    lang::{Lang, LangItem},
    log::Log,
    pool::DBPool,
    worker::{Worker, WorkerData},
};

/// Server management
pub struct Go;

impl Go {
    /// Run server in tokio runtime
    pub fn run(init: &Init, func: impl Fn() -> ActMap) {
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
    async fn listen(init: &Init, stop: Arc<AtomicBool>, func: impl Fn() -> ActMap) -> Option<JoinHandle<()>> {
        // Open bind port
        let bind = match init.conf.bind {
            Addr::SocketAddr(a) => TcpListener::bind(a),
            #[cfg(not(target_family = "windows"))]
            Addr::UDS(s) => TcpListener::bind(s),
        };
        let bind = match bind.await {
            Ok(i) => i,
            Err(e) => {
                Log::stop(500, Some(e.to_string()));
                return None;
            }
        };
        let root_path = Arc::new(init.root_path.clone());
        let db = Arc::new(init.conf.db.clone());
        let lang = Arc::new(init.conf.lang.clone());
        let bind_accept = Arc::new(init.conf.bind_accept.clone());
        let salt = Arc::new(init.conf.salt.clone());
        let bind_addr = init.conf.bind.clone();
        let engine_data = func();

        let main = tokio::spawn(async move {
            // Create pool database connector
            let max = db.max;
            let mut db = DBPool::new(max, db).await;
            if max != db.size {
                stop.store(true, Ordering::Relaxed);
                Log::stop(610, None);
                // send stop signal
                Go::send_stop(&bind_addr).await;
                return;
            }
            let langs = Go::get_langs(&mut db).await;
            let lang = Arc::new(Lang::new(&root_path, &lang, langs));
            let html = Arc::new(Html::new(&root_path));
            let cache = Cache::new().await;
            let engine = Arc::new(engine_data);

            let db = Arc::new(db);
            let salt = Arc::clone(&salt);

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
                let salt = Arc::clone(&salt);

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
                    let data = WorkerData { engine, lang, html, cache, db, salt };
                    Worker::run(stream, data).await;
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
        let rpc = match conf.rpc {
            Addr::SocketAddr(a) => TcpListener::bind(a),
            #[cfg(not(target_family = "windows"))]
            Addr::UDS(s) => TcpListener::bind(s),
        };
        let rpc = match rpc.await {
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
            if signal != conf.stop {
                Log::warning(206, Some(signal.to_string()));
                continue;
            }
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
        }
    }

    /// Send stop signal to bind port
    async fn send_stop(addr: &Addr) {
        #[allow(clippy::infallible_destructuring_match)]
        let socket = match addr {
            Addr::SocketAddr(s) => s,
            #[cfg(not(target_family = "windows"))]
            UDS(s) => SocketAddr::from(s),
        };
        match time::timeout(Duration::from_secs(1), TcpStream::connect(socket)).await {
            Ok(stream) => {
                if let Err(e) = stream {
                    Log::warning(222, Some(e.to_string()))
                }
            }
            Err(_) => Log::warning(221, None),
        }
    }

    /// Get list of enabled langs from database
    async fn get_langs(db: &mut DBPool) -> Vec<LangItem> {
        let sql = "
            SELECT lang_id, lang, code, name
            FROM lang
            WHERE enable
            ORDER BY sort
        ";
        let res = match db.query(sql).await {
            Some(res) => res,
            None => {
                Log::warning(1150, None);
                return Vec::new();
            }
        };
        let mut vec = Vec::with_capacity(res.len());
        for row in res {
            let id = row.get::<usize, i64>(0);
            vec.push(LangItem {
                id,
                code: row.get(1),
                lang: row.get(2),
                name: row.get(3),
            });
        }
        vec.shrink_to_fit();
        vec
    }
}
