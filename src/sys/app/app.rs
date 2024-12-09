use std::{
    io::{Read, Write},
    net::TcpStream,
    time::Duration,
};

#[cfg(not(target_family = "windows"))]
use std::os::unix::net::UnixStream;

use crate::{
    fnv1a_64,
    help::Help,
    log,
    sys::{net::stream::Socket, web::action::ModuleMap},
};

#[cfg(any(feature = "debug-v", feature = "debug-vv", feature = "debug-vvv"))]
use crate::sys::log::{InitLog, Log};

use super::{
    arg::{Arg, Mode},
    init::{Init, SIGNAL_TIMEOUT, SIGNAL_TIMEOUT_WAIT},
    run::Run,
};

/// Application
#[derive(Debug)]
pub(crate) struct App {}

impl App {
    pub(crate) fn run(name: &str, version: &str, desc: &str, engine: ModuleMap) -> Result<(), ()> {
        let args = match Arg::get() {
            Ok(args) => args,
            Err(_e) => {
                #[cfg(any(feature = "debug-v", feature = "debug-vv", feature = "debug-vvv"))]
                Log::init(InitLog::None);
                log!(stop, 0, "Неможливо прочитати параметри запуска. Помилка: {}", _e);
                return Err(());
            }
        };
        let init = match Init::parse(name.to_owned(), version.to_owned(), desc.to_owned(), &args.root) {
            Ok(init) => init,
            Err(_e) => {
                log!(stop, 0, "Неможливо прочитати файл з налаштуваннями чи файл неправильного формату. Помилка: {}", _e);
                return Err(());
            }
        };

        match args.mode {
            Mode::Help => Help::show(init),
            Mode::Start => App::start(args),
            Mode::Stop => App::stop(init),
            Mode::Status => App::status(init),
            Mode::Run => return Run::start(args, init, engine),
        }
        Ok(())
    }

    /// Starting a new instance of the application in server mode for Windows
    #[cfg(target_family = "windows")]
    fn start(args: Arg) {
        const DETACHED_PROCESS: u32 = 0x00000008;
        const CREATE_NEW_PROCESS_GROUP: u32 = 0x00000200;
        const CREATE_NO_WINDOW: u32 = 0x08000000;

        use std::{os::windows::process::CommandExt, process::Command};

        match Command::new(&args.exe)
            .arg("run")
            .arg("-r")
            .arg(&*args.root)
            .current_dir(&*args.root)
            .creation_flags(DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP | CREATE_NO_WINDOW)
            .spawn()
        {
            Ok(_c) => log!(info, 0, "{:?} run -r {:?}. PID: {}", args.exe, args.root, _c.id()),
            Err(_e) => log!(stop, 0, "{:?} run -r {:?}. Error: {}", args.exe, args.root, _e),
        };
    }

    /// Starting a new instance of the application in server mode for not Windows
    #[cfg(not(target_family = "windows"))]
    fn start(args: Arg) {
        use std::process::Command;

        match Command::new(&args.exe).arg("run").arg("-r").arg(&*args.root).current_dir(&*args.root).spawn() {
            Ok(_c) => log!(info, 0, "{:?} run -r {:?}. PID: {}", args.exe, args.root, _c.id()),
            Err(_e) => log!(stop, 0, "{:?} run -r {:?}. Error: {}", args.exe, args.root, _e),
        };
    }

    fn stop(init: Init) {
        let stop = fnv1a_64(format!("stop{}", init.web.salt).as_bytes()).to_be_bytes();
        match init.net.rpc {
            Socket::Inet(socket) => {
                let mut tcp = match TcpStream::connect_timeout(&socket, Duration::from_millis(SIGNAL_TIMEOUT)) {
                    Ok(tcp) => tcp,
                    Err(_e) => {
                        log!(stop, 0, "Неможливо відправити сигнал stop. Помилка: {}", _e);
                        return;
                    }
                };

                if let Err(_e) = tcp.write(&stop) {
                    log!(stop, 0, "Неможливо відправити сигнал stop. Помилка: {}", _e);
                    return;
                };
                if let Err(_e) = tcp.set_read_timeout(Some(Duration::from_millis(SIGNAL_TIMEOUT_WAIT))) {
                    log!(stop, 0, "Неможливо відправити сигнал stop. Помилка: {}", _e);
                    return;
                }

                let mut buf: [u8; 8] = [0; 8];
                if let Err(_e) = tcp.read_exact(&mut buf) {
                    log!(stop, 0, "Неможливо відправити сигнал stop. Помилка: {}", _e);
                    return;
                };
                let _pid = u64::from_be_bytes(buf);
                log!(info, 0, "Answer PID={}", _pid);
            }
            #[cfg(not(target_family = "windows"))]
            Socket::Unix(path) => {
                let mut tcp = match UnixStream::connect(path) {
                    Ok(tcp) => tcp,
                    Err(_e) => {
                        log!(stop, 0, "Неможливо відправити сигнал stop. Помилка: {}", _e);
                        return;
                    }
                };
                if let Err(_e) = tcp.set_write_timeout(Some(Duration::from_millis(SIGNAL_TIMEOUT))) {
                    log!(stop, 0, "Неможливо відправити сигнал stop. Помилка: {}", _e);
                    return;
                }
                if let Err(_e) = tcp.write(&stop) {
                    log!(stop, 0, "Неможливо відправити сигнал stop. Помилка: {}", _e);
                    return;
                }
                if let Err(_e) = tcp.set_read_timeout(Some(Duration::from_millis(SIGNAL_TIMEOUT_WAIT))) {
                    log!(stop, 0, "Неможливо відправити сигнал stop. Помилка: {}", _e);
                    return;
                }

                let mut buf: [u8; 8] = [0; 8];
                if let Err(_e) = tcp.read_exact(&mut buf) {
                    log!(stop, 0, "Неможливо відправити сигнал stop. Помилка: {}", _e);
                    return;
                };
                let _pid = u64::from_be_bytes(buf);
                log!(info, 0, "Answer PID={}", _pid);
            }
        }
    }

    fn status(init: Init) {
        let status = fnv1a_64(format!("status{}", init.web.salt).as_bytes()).to_be_bytes();
        match init.net.rpc {
            Socket::Inet(socket) => {
                let mut tcp = match TcpStream::connect_timeout(&socket, Duration::from_millis(SIGNAL_TIMEOUT)) {
                    Ok(tcp) => tcp,
                    Err(_e) => {
                        log!(stop, 0, "Неможливо відправити сигнал status. Помилка: {}", _e);
                        return;
                    }
                };
                if let Err(_e) = tcp.write(&status) {
                    log!(stop, 0, "Неможливо відправити сигнал status. Помилка: {}", _e);
                    return;
                };
                if let Err(_e) = tcp.set_read_timeout(Some(Duration::from_millis(SIGNAL_TIMEOUT))) {
                    log!(stop, 0, "Неможливо відправити сигнал status. Помилка: {}", _e);
                    return;
                }

                let mut buf: [u8; 8] = [0; 8];
                if let Err(_e) = tcp.read_exact(&mut buf) {
                    log!(stop, 0, "Неможливо відправити сигнал status. Помилка: {}", _e);
                    return;
                };

                let pid = u64::from_be_bytes(buf);

                let mut status = Vec::with_capacity(1024);
                if let Err(_e) = tcp.read_to_end(&mut status) {
                    log!(stop, 0, "Неможливо відправити сигнал status. Помилка: {}", _e);
                    return;
                };
                let answer = match String::from_utf8(status) {
                    Ok(a) => a,
                    Err(_e) => {
                        log!(stop, 0, "Неможливо відправити сигнал status. Помилка: {}", _e);
                        return;
                    }
                };
                log!(info, 0, "Answer PID={}", pid);
                println!("Answer PID={}\n{}", pid, answer);
            }
            #[cfg(not(target_family = "windows"))]
            Socket::Unix(path) => {
                let mut tcp = match UnixStream::connect(path) {
                    Ok(tcp) => tcp,
                    Err(_e) => {
                        log!(stop, 0, "Неможливо відправити сигнал status. Помилка: {}", _e);
                        return;
                    }
                };
                if let Err(_e) = tcp.set_write_timeout(Some(Duration::from_millis(SIGNAL_TIMEOUT))) {
                    log!(stop, 0, "Неможливо відправити сигнал status. Помилка: {}", _e);
                    return;
                }
                if let Err(_e) = tcp.write(&status) {
                    log!(stop, 0, "Неможливо відправити сигнал status. Помилка: {}", _e);
                    return;
                }
                if let Err(_e) = tcp.set_read_timeout(Some(Duration::from_millis(SIGNAL_TIMEOUT))) {
                    log!(stop, 0, "Неможливо відправити сигнал status. Помилка: {}", _e);
                    return;
                }

                let mut buf: [u8; 8] = [0; 8];
                if let Err(_e) = tcp.read_exact(&mut buf) {
                    log!(stop, 0, "Неможливо відправити сигнал status. Помилка: {}", _e);
                    return;
                };
                if let Err(_e) = tcp.set_read_timeout(Some(Duration::from_millis(SIGNAL_TIMEOUT))) {
                    log!(stop, 0, "Неможливо відправити сигнал status. Помилка: {}", _e);
                    return;
                }

                let pid = u64::from_be_bytes(buf);

                let mut status = Vec::with_capacity(1024);
                if let Err(_e) = tcp.read_to_end(&mut status) {
                    log!(stop, 0, "Неможливо відправити сигнал status. Помилка: {}", _e);
                    return;
                };
                let answer = match String::from_utf8(status) {
                    Ok(a) => a,
                    Err(_e) => {
                        log!(stop, 0, "Неможливо відправити сигнал status. Помилка: {}", _e);
                        return;
                    }
                };
                log!(info, 0, "Answer PID={}", pid);
                println!("Answer PID={}\n{}", pid, answer);
            }
        }
    }
}
