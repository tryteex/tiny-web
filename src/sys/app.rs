use std::{
    env,
    fs::{remove_file, OpenOptions},
    io::{stdout, Read, Write},
    net::TcpStream,
    process::Command,
    time::Duration,
};

#[cfg(not(target_family = "windows"))]
use std::os::unix::net::UnixStream;

use chrono::Local;
use rand;
use rand::seq::SliceRandom;
use sha3::{Digest, Sha3_512};

use crate::{help::Help, sys::log::Log};

use super::{
    action::ActMap,
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
    pub fn new(name: &str, version: &str, desc: &str) -> Option<App> {
        let init = match Init::new(name, version, desc) {
            Some(i) => i,
            None => App::install(name, version, desc)?,
        };
        Some(App { init })
    }

    /// Run application
    pub fn run(&self, func: impl Fn() -> ActMap) {
        Log::info(17, Some(format!("{:?}", self.init.mode)));
        match self.init.mode {
            Mode::Start => self.start(),
            Mode::Stop => self.stop(),
            Mode::Help => Help::show(&self.init.conf.name, &self.init.conf.version, &self.init.conf.desc),
            Mode::Go => Go::run(&self.init, func),
            Mode::Status => todo!(),
        };
    }

    /// Send stop signal
    fn stop(&self) {
        let mut buf: [u8; 8] = [0; 8];
        #[allow(clippy::infallible_destructuring_match)]
        match &self.init.conf.rpc {
            Addr::SocketAddr(socket) => {
                let mut tcp = match TcpStream::connect_timeout(socket, Duration::from_secs(2)) {
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
                if let Err(e) = tcp.read_exact(&mut buf) {
                    Log::stop(217, Some(e.to_string()));
                    return;
                };
            }
            #[cfg(not(target_family = "windows"))]
            Addr::UDS(path) => {
                let mut tcp = match UnixStream::connect(path) {
                    Ok(t) => t,
                    Err(e) => {
                        Log::stop(213, Some(e.to_string()));
                        return;
                    }
                };
                if let Err(e) = tcp.set_write_timeout(Some(Duration::new(2, 0))) {
                    Log::stop(223, Some(e.to_string()));
                    return;
                };
                if let Err(e) = tcp.write(&self.init.conf.stop.to_be_bytes()) {
                    Log::stop(214, Some(e.to_string()));
                    return;
                };
                if let Err(e) = tcp.set_read_timeout(Some(Duration::from_secs(30))) {
                    Log::stop(216, Some(e.to_string()));
                    return;
                };

                // Reads answer
                if let Err(e) = tcp.read_exact(&mut buf) {
                    Log::stop(217, Some(e.to_string()));
                    return;
                };
            }
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
            Ok(c) => Log::info(211, Some(format!("{} {}. PID: {}", &exe, args.join(" "), c.id()))),
            Err(e) => Log::stop(212, Some(format!("{} {}. Error: {}", &exe, args.join(" "), e))),
        };
    }

    /// Starting a new instance of the application in server mode for not Windows
    #[cfg(not(target_family = "windows"))]
    fn start(&self) {
        let path = &self.init.exe_path;
        let exe = &self.init.exe_file;

        let args = vec!["go", "-r", &self.init.root_path];
        match Command::new(exe).args(&args[..]).current_dir(path).spawn() {
            Ok(c) => Log::info(211, Some(format!("{} {}. PID: {}", &exe, args.join(" "), c.id()))),
            Err(e) => Log::stop(212, Some(format!("{} {}. Error: {}", &exe, args.join(" "), e))),
        };
    }

    /// Convecrt unix path to windows
    #[cfg(target_family = "windows")]
    fn to_win_path(text: &str) -> String {
        text.replace('/', r"\")
    }

    /// Create config file
    fn install(name: &str, version: &str, desc: &str) -> Option<Init> {
        println!("Файл конфігурації не знайдено.");
        print!("Запустити поміщника створення такого файлу [y]? ");
        stdout().flush().ok()?;
        let mut buf = String::new();
        std::io::stdin().read_line(&mut buf).ok()?;
        buf = buf.trim().to_lowercase();
        if !buf.is_empty() && buf != "y" {
            return None;
        }
        println!("Зупинити поміщника можна за допомогою клавіш Ctrl+С.");
        let dir_path = env::current_dir().ok()?.to_str()?.replace('\\', "/");
        let mut path_path = dir_path.clone();

        // Path to the configuration file
        let config_path = loop {
            print!("Введіть шлях де буде розміщуватися файл конфігурації [{}]: ", dir_path);
            stdout().flush().ok()?;
            buf.clear();
            if let Err(e) = std::io::stdin().read_line(&mut buf) {
                println!("Виникла помилка: {}", e);
                continue;
            };
            #[cfg(target_family = "windows")]
            {
                buf = App::to_win_path(buf.trim());
            }
            #[cfg(not(target_family = "windows"))]
            {
                buf = buf.trim().to_owned();
            }
            let file_path = if buf.is_empty() {
                format!("{}/tiny.conf", dir_path)
            } else if buf.ends_with('/') {
                path_path = buf[..buf.len() - 1].to_owned();
                format!("{}tiny.conf", buf)
            } else {
                path_path = buf.clone();
                format!("{}/tiny.conf", buf)
            };
            let file = match OpenOptions::new().write(true).create_new(true).open(&file_path) {
                Ok(f) => f,
                Err(e) => {
                    println!("Виникла помилка: {}", e);
                    continue;
                }
            };
            if let Err(e) = file.sync_all() {
                println!("Виникла помилка: {}", e);
                continue;
            };
            drop(file);
            if let Err(e) = remove_file(&file_path) {
                println!("Виникла помилка під час перевірки файлової системи: {}", e);
                continue;
            };
            break file_path;
        };

        let mut config = Vec::with_capacity(16);
        // Set default lang
        loop {
            print!("Введіть мову за умовчанням використовуючи ISO 639-1 [en]: ");
            stdout().flush().ok()?;
            buf.clear();
            if let Err(e) = std::io::stdin().read_line(&mut buf) {
                println!("Виникла помилка: {}", e);
                continue;
            };
            buf = buf.trim().to_lowercase();
            if buf.is_empty() {
                buf = "en".to_string();
            } else if buf.len() != 2 {
                println!("Мова повинна бути не більше двох символів");
                continue;
            }
            if !&buf[..1].is_ascii() || !&buf[1..].is_ascii() {
                println!("Символи мови повинні бути символами ASCII");
                continue;
            }
            config.push(format!("lang = {}", buf));
            break;
        }

        // Set log file
        loop {
            print!("Введіть назву файла для запису логів [{}/tiny.log]: ", &path_path);
            stdout().flush().ok()?;
            buf.clear();
            if let Err(e) = std::io::stdin().read_line(&mut buf) {
                println!("Виникла помилка: {}", e);
                continue;
            };
            #[cfg(target_family = "windows")]
            {
                buf = App::to_win_path(buf.trim());
            }
            #[cfg(not(target_family = "windows"))]
            {
                buf = buf.trim().to_owned();
            }
            let log = if buf.is_empty() { format!("{}/tiny.log", path_path) } else { buf.clone() };
            config.push(format!("log = {}", log));
            break;
        }

        // Set max of work threads in async
        loop {
            print!("Введіть maximum of work threads in async. Usually a little more than CPUs. [auto]: ");
            stdout().flush().ok()?;
            buf.clear();
            if let Err(e) = std::io::stdin().read_line(&mut buf) {
                println!("Виникла помилка: {}", e);
                continue;
            };
            buf = buf.trim().to_owned();
            let mut max = "auto".to_owned();
            if !buf.is_empty() {
                match buf.parse::<u16>() {
                    Ok(u) => max = u.to_string(),
                    Err(e) => {
                        println!("Виникла помилка при розпізнаванні числа: {}", e);
                        continue;
                    }
                }
            };
            config.push(format!("max = {}", max));
            break;
        }

        // IP address or UDS from which to accept connections.
        loop {
            print!("Введіть IP address from which to accept connections [127.0.0.1]: ");
            stdout().flush().ok()?;
            buf.clear();
            if let Err(e) = std::io::stdin().read_line(&mut buf) {
                println!("Виникла помилка: {}", e);
                continue;
            };
            buf = buf.trim().to_owned();
            let from = if buf.is_empty() { "127.0.0.1".to_owned() } else { buf.clone() };
            config.push(format!("bind_from = {}", from));
            break;
        }

        // IP address and port to work this server
        loop {
            print!("Введіть IP address from which to accept connections [127.0.0.1:12500]: ");
            stdout().flush().ok()?;
            buf.clear();
            if let Err(e) = std::io::stdin().read_line(&mut buf) {
                println!("Виникла помилка: {}", e);
                continue;
            };
            buf = buf.trim().to_owned();
            let bind = if buf.is_empty() { "127.0.0.1:12500".to_owned() } else { buf.clone() };
            config.push(format!("bind = {}", bind));
            break;
        }

        // IP address from which to accept connections for managing the server
        loop {
            print!("Введіть IP address from which to accept connections for managing the server [127.0.0.1]: ");
            stdout().flush().ok()?;
            buf.clear();
            if let Err(e) = std::io::stdin().read_line(&mut buf) {
                println!("Виникла помилка: {}", e);
                continue;
            };
            buf = buf.trim().to_owned();
            let from = if buf.is_empty() { "127.0.0.1".to_owned() } else { buf.clone() };
            config.push(format!("rpc_from = {}", from));
            break;
        }

        // IP address and port to manage this server
        loop {
            print!("Введіть IP address and port to manage this server [127.0.0.1:12501]: ");
            stdout().flush().ok()?;
            buf.clear();
            if let Err(e) = std::io::stdin().read_line(&mut buf) {
                println!("Виникла помилка: {}", e);
                continue;
            };
            buf = buf.trim().to_owned();
            let rpc = if buf.is_empty() { "127.0.0.1:12500".to_owned() } else { buf.clone() };
            config.push(format!("rpc = {}", rpc));
            break;
        }

        // salt for a crypto functions
        loop {
            let salt = App::generate_salt();
            print!("Введіть salt for a crypto functions [{}]: ", &salt);
            stdout().flush().ok()?;
            buf.clear();
            if let Err(e) = std::io::stdin().read_line(&mut buf) {
                println!("Виникла помилка: {}", e);
                continue;
            };
            buf = buf.trim().to_owned();
            let salt = if buf.is_empty() { salt } else { buf.clone() };
            config.push(format!("salt = {}", salt));
            break;
        }

        // Postgresql database host
        loop {
            print!("Введіть Postgresql database host [127.0.0.1]: ");
            stdout().flush().ok()?;
            buf.clear();
            if let Err(e) = std::io::stdin().read_line(&mut buf) {
                println!("Виникла помилка: {}", e);
                continue;
            };
            buf = buf.trim().to_owned();
            let host = if buf.is_empty() { "127.0.0.1".to_owned() } else { buf.clone() };
            config.push(format!("db_host = {}", host));
            break;
        }

        // Postgresql database port
        loop {
            print!("Введіть Postgresql database port [5432]: ");
            stdout().flush().ok()?;
            buf.clear();
            if let Err(e) = std::io::stdin().read_line(&mut buf) {
                println!("Виникла помилка: {}", e);
                continue;
            };
            buf = buf.trim().to_owned();
            let port = if buf.is_empty() {
                5432
            } else {
                match buf.parse::<u16>() {
                    Ok(u) => u,
                    Err(e) => {
                        println!("Виникла помилка при розпізнаванні числа: {}", e);
                        continue;
                    }
                }
            };
            config.push(format!("db_port = {}", port));
            break;
        }

        // Postgresql database name
        loop {
            print!("Введіть Postgresql database name [tiny]: ");
            stdout().flush().ok()?;
            buf.clear();
            if let Err(e) = std::io::stdin().read_line(&mut buf) {
                println!("Виникла помилка: {}", e);
                continue;
            };
            buf = buf.trim().to_owned();
            let name = if buf.is_empty() { "tiny".to_owned() } else { buf.clone() };
            config.push(format!("db_name = {}", name));
            break;
        }

        // Postgresql database user
        loop {
            print!("Введіть Postgresql database user [tiny]: ");
            stdout().flush().ok()?;
            buf.clear();
            if let Err(e) = std::io::stdin().read_line(&mut buf) {
                println!("Виникла помилка: {}", e);
                continue;
            };
            buf = buf.trim().to_owned();
            let user = if buf.is_empty() { "tiny".to_owned() } else { buf.clone() };
            config.push(format!("db_user = {}", user));
            break;
        }

        // Postgresql database pwd
        loop {
            print!("Введіть Postgresql database user pwd []: ");
            stdout().flush().ok()?;
            buf.clear();
            if let Err(e) = std::io::stdin().read_line(&mut buf) {
                println!("Виникла помилка: {}", e);
                continue;
            };
            buf = buf.trim().to_owned();
            let pwd = if buf.is_empty() { "".to_owned() } else { buf.clone() };
            config.push(format!("db_pwd = {}", pwd));
            break;
        }

        // Number of connections to the database for all work threads in async.
        loop {
            print!("Введіть Number of connections to the database for all work threads in async. [auto]: ");
            stdout().flush().ok()?;
            buf.clear();
            if let Err(e) = std::io::stdin().read_line(&mut buf) {
                println!("Виникла помилка: {}", e);
                continue;
            };
            buf = buf.trim().to_owned();
            let mut max = "auto".to_owned();
            if !buf.is_empty() {
                match buf.parse::<u16>() {
                    Ok(u) => max = u.to_string(),
                    Err(e) => {
                        println!("Виникла помилка при розпізнаванні числа: {}", e);
                        continue;
                    }
                }
            };
            config.push(format!("db_max = {}", max));
            break;
        }

        // Postgresql database sslmode mode.
        loop {
            print!("Введіть Postgresql database sslmode mode (require or no) [no]: ");
            stdout().flush().ok()?;
            buf.clear();
            if let Err(e) = std::io::stdin().read_line(&mut buf) {
                println!("Виникла помилка: {}", e);
                continue;
            };
            buf = buf.trim().to_owned();
            let sslmode = if buf.is_empty() {
                "no".to_owned()
            } else if buf == "no" || buf == "require" {
                buf.clone()
            } else {
                println!("Виникла помилка. Тільки require or no");
                continue;
            };
            config.push(format!("sslmode = {}", sslmode));
            break;
        }

        // Postgresql database time zone.
        loop {
            print!("Введіть Postgresql database time zone or empty to default (example: Europe/Kyiv) []: ");
            stdout().flush().ok()?;
            buf.clear();
            if let Err(e) = std::io::stdin().read_line(&mut buf) {
                println!("Виникла помилка: {}", e);
                continue;
            };
            buf = buf.trim().to_owned();
            let zone = if buf.is_empty() { "".to_owned() } else { buf.clone() };
            config.push(format!("zone = {}", zone));
            break;
        }

        let mut file = match OpenOptions::new().write(true).create_new(true).open(&config_path) {
            Ok(f) => f,
            Err(e) => {
                println!("Виникла помилка при створенні файлу {}: {}", &config_path, e);
                return None;
            }
        };
        if let Err(e) = file.write_all(config.join("\n").as_bytes()) {
            println!("Виникла помилка при записі даних в файл {}: {}", &config_path, e);
            return None;
        };
        if let Err(e) = file.sync_all() {
            println!("Виникла помилка при записі файлу {} на диск: {}", &config_path, e);
            return None;
        };

        // Install database script
        println!("It is necessary to download the https://github.com/tryteex/tiny-web/blob/main/install.sql and create a database structure.");
        println!("This part is under development.");

        Init::new(name, version, desc)
    }

    /// Generete new salt
    fn generate_salt() -> String {
        // Generate a new cookie
        let time = Local::now().format("%Y.%m.%d %H:%M:%S%.9f %:z").to_string();
        let mut hasher = Sha3_512::new();
        hasher.update(time.as_bytes());
        let salt = format!("{:#x}123467890-+_)(*&^%$#@!qazedcrfvtgbyhnujmik,ol.p;/[]:?><|", hasher.finalize());
        let mut rng = rand::thread_rng();
        let mut slice = salt.as_bytes().to_vec();
        slice.shuffle(&mut rng);
        match String::from_utf8(slice) {
            Ok(salt) => salt[..32].to_owned(),
            Err(_) => salt[..32].to_owned(),
        }
    }
}
