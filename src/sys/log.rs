use std::{fs::OpenOptions, io::Write, process, sync::Arc};

use chrono::Local;
use once_cell::sync::OnceCell;

/// Responsible for log level
///
/// # Values
///
/// * `Info` - Informational message only;
/// * `Warning` - Warning, the program may continue to run;
/// * `Stop` - Error, the program must soft stop;
/// * `Error` - Abnormal behavior, the program stops immediately;
/// * `Critical` - Critical error, the program stops immediately, for internal use only.
#[derive(Debug)]
enum LogView {
    /// Informational message only.
    Info,
    /// Warning, the program may continue to run.
    Warning,
    /// Error, the program must soft stop.
    Stop,
    /// Abnormal behavior, the program stops immediately.
    Error,
    /// Critical error, the program stops immediately, for internal use only.
    Critical,
}

/// Responsible for description log message
///
/// # Values
///
/// * `view: LogView` - Log level;
/// * `number: u16` - Number of log message;
/// * `text: Option<String>` - Optional additional text;
#[derive(Debug)]
struct LogText {
    /// Log level.
    view: LogView,
    /// Number of log message.
    number: u16,
    /// Optional additional text;
    text: Option<String>,
}

/// Path to log file
static LOG_FILE: OnceCell<Arc<String>> = OnceCell::new();

/// Responsible for event log messages.
pub struct Log;

impl Log {
    /// Save informational message only to log file.
    ///
    /// # Parameters
    ///
    /// * `number: u16` - Log number;
    /// * `text: Option<String>` - Additional log description.
    pub fn info(number: u16, text: Option<String>) {
        Log::save(LogText {
            view: LogView::Info,
            number,
            text,
        });
    }

    /// Save warning message to log file, the program may continue to run.
    ///
    /// # Parameters
    ///
    /// * `number: u16` - Log number;
    /// * `text: Option<String>` - Additional log description.
    pub fn warning(number: u16, text: Option<String>) {
        Log::save(LogText {
            view: LogView::Warning,
            number,
            text,
        });
    }

    /// Save stop message to log file, the program must soft stop.
    ///
    /// # Parameters
    ///
    /// * `number: u16` - Log number;
    /// * `text: Option<String>` - Additional log description.
    pub fn stop(number: u16, text: Option<String>) {
        Log::save(LogText {
            view: LogView::Stop,
            number,
            text,
        });
    }

    /// Save error message to log file, this is abnormal behavior, the program stops immediately.
    ///
    /// # Parameters
    ///
    /// * `number: u16` - Log number;
    /// * `text: Option<String>` - Additional log description.
    ///
    /// # Return
    ///
    /// Program call process::exit(1).
    pub fn error(number: u16, text: Option<String>) -> ! {
        Log::save(LogText {
            view: LogView::Error,
            number,
            text,
        });
        process::exit(number as i32);
    }

    /// Set new path to log file.
    ///
    /// # Parameters
    ///
    /// * `path: String` - New path to log file.
    pub fn set_path(path: String) {
        if LOG_FILE.set(Arc::new(path)).is_err() {
            Log::panic("Can't set new path to LOG_FILE");
        };
    }

    /// Simple save message to file.
    ///
    /// # Parameters
    ///
    /// * `log: LogText` - Description log message.
    fn save(log: LogText) {
        let time = Local::now().format("%Y.%m.%d %H:%M:%S%.9f").to_string();
        let file = match LOG_FILE.get() {
            Some(path) => path.as_str(),
            None => {
                Log::set_path("tiny.log".to_owned());
                "tiny.log"
            }
        };

        let str = match log.text {
            Some(s) => format!(
                "ID:{} Time: {} Type: {:?} Number:{} Text:{} -> {}\n",
                process::id(),
                time,
                log.view,
                log.number,
                Log::number_to_text(log.number),
                s
            ),
            None => format!(
                "ID:{} Time: {} Type: {:?} Number:{} Text:{}\n",
                process::id(),
                time,
                log.view,
                log.number,
                Log::number_to_text(log.number)
            ),
        };
        match OpenOptions::new()
            .create(true)
            .write(true)
            .append(true)
            .open(file)
        {
            Ok(mut file) => match file.write_all(str.as_bytes()) {
                Ok(f) => f,
                Err(e) => Log::panic(&e.to_string()),
            },
            Err(e) => Log::panic(&format!("Can't save data to log file {} -> {}", file, e)),
        };
    }

    /// Save or show panic message:
    ///
    /// # Parameters
    ///
    /// * `text: &str` - Panic message.
    fn panic(text: &str) -> ! {
        let time = Local::now().format("%Y.%m.%d %H:%M:%S%.9f").to_string();
        let str = format!(
            "ID:{} Time:{} Type: {:?} Number: Text: Panic error -> {}\n",
            process::id(),
            time,
            LogView::Critical,
            text
        );
        let file = match LOG_FILE.get() {
            Some(path) => path.as_str(),
            None => {
                Log::set_path("tiny.log".to_owned());
                "tiny.log"
            }
        };
        match OpenOptions::new()
            .create(true)
            .write(true)
            .append(true)
            .open(file)
        {
            Ok(mut f) => {
                if let Err(e) = f.write_all(str.as_bytes()) {
                    let str = format!(
                        "ID:{} Time:{} Type: {:?} Number: Text: Can't write log file \"{}\" - {}\n",
                        process::id(),
                        time,
                        LogView::Critical,
                        file,
                        e
                    );
                    eprint!("{}", &str);
                }
            }
            Err(e) => {
                let str = format!(
                    "ID:{} Time:{} Type: {:?} Number: Text: Can't open log file \"{}\" - {}\n",
                    process::id(),
                    time,
                    LogView::Critical,
                    file,
                    e
                );
                eprint!("{}", &str);
            }
        };
        process::exit(1);
    }

    /// Get static text by log number:
    ///
    /// # Parameters
    ///
    /// * `number: u16` - Log number.
    ///
    /// # Return
    ///
    /// * `&'static str` - Static text with log number description
    fn number_to_text(number: u16) -> &'static str {
        match number {
            1 => "Can't create runtime for async server",
            2 => "Mutex can't lock cache",
            3 => "Implementation of this protocol later",

            10 => "Unable to get the app path",
            11 => "The app path contains invalid characters",
            12 => "The app must be on a local computer",
            13 => "There is no path to the config file specified after the -r option",
            14 => "Can't read the config file",
            15 => "The config file is not found",
            16 => "Can't detect the app path",
            17 => "Init mode",

            50 => "The 'salt' parameter in configuration file must be set",
            51 => "The 'lang' parameter in configuration file must consist of two characters according to ISO 639-1",
            52 => "The 'max' parameter in configuration file must be usize and greater than 0",
            53 => "The 'bind_from' parameter in configuration file must be IP address in format xxx.xxx.xxx.xxx",
            54 => "The 'bind' parameter in configuration file must be IP:PORT address in format xxx.xxx.xxx.xxx:yyyy or Unix domain socket",
            55 => "The 'rpc_from' parameter in configuration file must be IP address in format xxx.xxx.xxx.xxx",
            56 => "The 'rpc' parameter in configuration file must be IP:PORT address in format xxx.xxx.xxx.xxx:yyyy or Unix domain socket",
            57 => "The 'db_port' parameter in configuration file must be u16 and greater than 0",
            58 => "The 'db_max' parameter in configuration file must be usize and greater than 0",
            59 => "The 'db_host' parameter in configuration file can't be empty",

            200 => "Start",
            201 => "Stop",
            202 => "Unable to open rpc port",
            203 => "IP address for rpc control is not allowed",
            204 => "Reading from the rpc stream timed out",
            205 => "Error reading from the rpc",
            206 => "Received signal is not the stop signal",
            207 => "Stop signal received successfully",
            211 => "The app start successfully",
            212 => "Can't start the app",
            213 => "Can't connect to the server",
            214 => "Can't send 'stop' signal to the server",
            215 => "Can't write 'stop' signal to the stream",
            216 => "Can't set read_timeout",
            217 => "Can't read signal from stream",
            218 => "Stop signal sent successfully", 
            219 => "Can't set TCP_NODELAY", 
            220 => "An error occurred while waiting for the main thread to stop",
            221 => "Unable to connect to server due to timeout",
            222 => "Can't connect to server",

            500 => "Unable to open server port",
            501 => "IP address from which to accept connections is not allowed",
            502 => "Failed to send completion signal",
            503 => "An error occurred while waiting for the main thread to stop",
            504 => "Critical socket operation error",
            505 => "An error occurred while waiting for the threads to abort",
            506 => "Can't set TCP_NODELAY", 

            600 => "Can't create tlsconnector to database",
            601 => "Can't connect to database",
            602 => "Can't execute query",
            603 => "Connection to database is lost",
            604 => "Database is not initialized",
            605 => "Database is closed. Reconnect is disabled.",
            606 => "Can't receive free permit from pool database.",
            607 => "Can't find free database, but semaphore says that can free.",
            609 => "Can't parse connection string",
            610 => "Can't create pool of connections",
            611 => "Error close connection task in connection with database",
            612 => "Error close connection with database",
            613 => "Can't prepare statement",

            1100 => "Can't open root_dir/app",
            1101 => "Can't get dir entry",
            1102 => "Can't open dir",
            1103 => "Can't delete input file",

            1200 => "Unable to specify node type",
            1201 => "Unable to specify \"if\" node type",
            1202 => "Unable to specify \"loop\" node type",

            1150 => "Can't load languages from database",
            1151 => "Language list is empty",

            2000 => "Unable to read from stream",
            2001 => "It is not possible to read the first time from the stream, due to a timeout",

            // FastCGI Error
            2100 => "Unable to recognize fastCGI record",
            2101 => "Incorrect fastCGI header",
            2102 => "Unsupport fastCGI header type",
            2103 => "Unsupport UTF8 symbol in FastCGI params",

            // Ation engine
            3000 => "Wrong cache type key of Redirect",
            3001 => "Wrong cache type key of Route",

            _ => "Unknown error"
        }
    }
}
