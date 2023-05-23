use std::{
    io::{Read, Write},
    net::TcpStream,
    process::Command,
    time::Duration,
};

use crate::{help::Help, sys::log::Log};

use super::{
    go::Go,
    init::{Addr, Init, Mode},
};

/// Application information
///
/// # Values
///
/// * `init: Init` - Server configuration.
#[derive(Debug)]
pub struct App {
    /// Server configuration.
    pub init: Init,
}

impl App {
    /// Initializes application
    pub fn new() -> Option<App> {
        let init = Init::new()?;
        Some(App { init })
    }

    /// Run application
    pub fn run(&self) {
        Log::info(17, Some(format!("{:?}", self.init.mode)));
        match self.init.mode {
            Mode::Start => self.start(),
            Mode::Stop => self.stop(),
            Mode::Help => Help::show(&self.init.conf.version),
            Mode::Go => Go::run(&self.init),
            Mode::Status => todo!(),
        };
    }

    /// Send stop signal
    fn stop(&self) {
        #[allow(clippy::infallible_destructuring_match)]
        let socket = match self.init.conf.rpc {
            Addr::SocketAddr(s) => s,
            #[cfg(not(target_family = "windows"))]
            UDS(s) => SocketAddr::from(s),
        };
        let mut tcp = match TcpStream::connect_timeout(&socket, Duration::from_secs(2)) {
            Ok(t) => t,
            Err(e) => {
                Log::stop(213, Some(e.to_string()));
                return;
            }
        };
        // Send stop to server into rpc channal
        if let Err(e) = tcp.write(&self.init.conf.stop.to_be_bytes()) {
            Log::stop(214, Some(e.to_string()));
            return;
        };
        if let Err(e) = tcp.set_read_timeout(Some(Duration::from_secs(30))) {
            Log::stop(216, Some(e.to_string()));
            return;
        };

        // Reads answer
        let mut buf: [u8; 8] = [0; 8];
        if let Err(e) = tcp.read_exact(&mut buf) {
            Log::stop(217, Some(e.to_string()));
            return;
        };
        let pid = u64::from_be_bytes(buf);
        Log::info(218, Some(format!("Answer PID={}", pid)));
    }

    /// Starting a new instance of the application in server mode for Windows
    #[cfg(target_family = "windows")]
    fn start(&self) {
        let path = App::to_win_path(&self.init.exe_path);
        let exe = App::to_win_path(&self.init.exe_file);
        const DETACHED_PROCESS: u32 = 0x00000008;
        const CREATE_NEW_PROCESS_GROUP: u32 = 0x00000200;
        const CREATE_NO_WINDOW: u32 = 0x08000000;

        let args = ["go", "-r", &self.init.root_path];
        use std::os::windows::process::CommandExt;

        match Command::new(&exe)
            .args(args)
            .current_dir(&path)
            .creation_flags(DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP | CREATE_NO_WINDOW)
            .spawn()
        {
            Ok(c) => Log::info(
                211,
                Some(format!("{} {}. PID: {}", &exe, args.join(" "), c.id())),
            ),
            Err(e) => Log::stop(
                212,
                Some(format!("{} {}. Error: {}", &exe, args.join(" "), e)),
            ),
        };
    }

    /// Starting a new instance of the application in server mode for not Windows
    #[cfg(not(target_family = "windows"))]
    fn start(&self) {
        let path = &self.init.exe_path;
        let exe = &self.init.exe_file;

        let args = vec!["go", "-r", &self.init.root_path];
        match Command::new(&exe)
            .args(&args[..])
            .current_dir(&path)
            .spawn()
        {
            Ok(c) => Log::info(
                211,
                Some(format!("{} {}. PID: {}", &exe, args.join(" "), c.id())),
            ),
            Err(e) => Log::stop(
                212,
                Some(format!("{} {}. Error: {}", &exe, args.join(" "), e)),
            ),
        };
    }

    /// Convecrt windows path to unix
    #[cfg(target_family = "windows")]
    fn to_win_path(text: &str) -> String {
        text.replace('/', "\\")
    }
}
