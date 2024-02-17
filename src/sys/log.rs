use std::{fs::OpenOptions, io::Write, process, sync::OnceLock};

use chrono::Local;

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
static LOG_FILE: OnceLock<String> = OnceLock::new();

/// Responsible for event log messages.
pub struct Log;

impl Log {
    /// Save informational message only to log file.
    ///
    /// # Parameters
    ///
    /// * `number: u16` - Log number;
    /// * `text: Option<String>` - Additional log description.
    pub fn info(number: u16, text: Option<String>) -> String {
        Log::save(LogText { view: LogView::Info, number, text })
    }

    /// Save warning message to log file, the program may continue to run.
    ///
    /// # Parameters
    ///
    /// * `number: u16` - Log number;
    /// * `text: Option<String>` - Additional log description.
    pub fn warning(number: u16, text: Option<String>) -> String {
        Log::save(LogText { view: LogView::Warning, number, text })
    }

    /// Save stop message to log file, the program must soft stop.
    ///
    /// # Parameters
    ///
    /// * `number: u16` - Log number;
    /// * `text: Option<String>` - Additional log description.
    pub fn stop(number: u16, text: Option<String>) -> String {
        Log::save(LogText { view: LogView::Stop, number, text })
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
        Log::save(LogText { view: LogView::Error, number, text });
        process::exit(number as i32);
    }

    /// Set new path to log file.
    ///
    /// # Parameters
    ///
    /// * `path: String` - New path to log file.
    pub fn set_path(path: String) {
        if LOG_FILE.set(path).is_err() {
            Log::panic("Can't set new path to LOG_FILE");
        };
    }

    /// Simple save message to file.
    ///
    /// # Parameters
    ///
    /// * `log: LogText` - Description log message.
    fn save(log: LogText) -> String {
        let time = Local::now().format("%Y.%m.%d %H:%M:%S%.9f").to_string();
        let text = match log.text {
            Some(s) => format!("Text: {} -> {}", Log::number_to_text(log.number), s),
            None => format!("Text: {}", Log::number_to_text(log.number)),
        };
        let str = format!("ID: {} Time: {} Type: {:?}. Number: {} {}\n", process::id(), time, log.view, log.number, text);

        #[cfg(debug_assertions)]
        eprintln!("{}", str.trim_end());

        let file = match LOG_FILE.get() {
            Some(path) => path.as_str(),
            None => {
                Log::set_path("tiny.log".to_owned());
                "tiny.log"
            }
        };
        match OpenOptions::new().create(true).append(true).open(file) {
            Ok(mut file) => match file.write_all(str.as_bytes()) {
                Ok(f) => f,
                Err(e) => Log::panic(&e.to_string()),
            },
            Err(e) => Log::panic(&format!("Can't save data to log file {} -> {}", file, e)),
        };
        text
    }

    /// Save or show panic message:
    ///
    /// # Parameters
    ///
    /// * `text: &str` - Panic message.
    fn panic(text: &str) -> ! {
        let time = Local::now().format("%Y.%m.%d %H:%M:%S%.9f").to_string();
        let str = format!(
            "ID: {} Time: {} Type: {:?}. Number: Text: Panic error -> {}\n",
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
        match OpenOptions::new().create(true).append(true).open(file) {
            Ok(mut f) => {
                if let Err(e) = f.write_all(str.as_bytes()) {
                    let str = format!(
                        r#"ID: {} Time: {} Type: {:?}. Number: Text: Can't write log file "{}" - {}\n"#,
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
                    r#"ID: {} Time: {} Type: {:?}. Number: Text: Can't open log file "{}" - {}\n"#,
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
            18 => "The config file 'tine.toml' is not a TOML https://toml.io/ format",
            19 => "The lang file is not a TOML https://toml.io/ format",

            50 => "The 'salt' parameter in configuration file must be set",
            51 => "The 'lang' parameter in configuration file must be a string and consist of two characters according to ISO 639-1",
            52 => "The 'max' parameter in configuration file must be usize and greater than 0 or \"auto\"",
            53 => "The 'bind_from' parameter in configuration file must be IP address in format xxx.xxx.xxx.xxx or \"any\" or empty for Unix domain socket",
            54 => "The 'bind' parameter in configuration file must be IP:PORT address in format xxx.xxx.xxx.xxx:yyyy or Unix domain socket",
            55 => "The 'rpc_from' parameter in configuration file must be IP address in format xxx.xxx.xxx.xxx or \"any\" or empty for Unix domain socket",
            56 => "The 'rpc' parameter in configuration file must be IP:PORT address in format xxx.xxx.xxx.xxx:yyyy or Unix domain socket",
            57 => "The 'db_port' parameter in configuration file must be bool. If true sslmode is requeres",
            58 => "The 'db_max' parameter in configuration file must be usize and greater than 0 or \"auto\"",
            59 => "The 'db_host' parameter in configuration file can't be empty",
            60 => "The 'protokol' parameter in configuration file must be only string: 'FastCGI, SCGI, uWSGI, gRPC, HTTP or WebSocket'",
            61 => "The 'log' parameter in configuration file must be a string",
            62 => "The 'salt' parameter in configuration file must be a string",
            63 => "The 'db_host' parameter in configuration file must be a string",
            64 => "The 'db_name' parameter in configuration file must be a string",
            65 => "The 'db_user' parameter in configuration file must be a string",
            66 => "The 'db_pwd' parameter in configuration file must be a string",
            67 => "The 'sslmode' parameter in configuration file can be a \"require\"",
            68 => "The 'zone' parameter in configuration file must be a string and not empty",
            69 => "The 'prepare' parameter in configuration file must be a group",
            70 => "The Key parameter in the 'prepare' group in configuration file must consist from two items: \"query\" and \"types\". \"Types\" ia array of string: BOOL, INT8, INT2, INT4, TEXT, VARCHAR, FLOAT4, FLOAT8, JSON, TIMESTAMPTZ, UUID, BYTEA",

            200 => "Start",
            201 => "Stop",
            202 => "Unable to open rpc port",
            203 => "IP address for rpc control is not allowed",
            204 => "Reading from the rpc stream timed out",
            205 => "Error reading from the rpc",
            206 => "Received signal is not the stop or status signals",
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
            223 => "Can't set write_timeout",
            224 => "Can't send 'status' signal to the server",
            225 => "Can't read answer from stream",
            226 => "Can't recognize answer from stream",
            227 => "Status signal received successfully",

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
            614 => "Unknown database type",
            615 => "Key not found",

            700 => "Error parse html template",

            1100 => "Can't open root_dir/app",
            1101 => "Can't get dir entry",
            1102 => "Can't open dir",
            1103 => "Can't delete input file",

            1200 => "Unable to specify node type",
            1201 => r#"Unable to specify "if" node type"#,
            1202 => r#"Unable to specify "loop" node type"#,

            1150 => "Can't load languages from database",
            1151 => "Language list is empty",

            2000 => "Unable to read from stream",
            2001 => "It is not possible to read the first time from the stream, due to a timeout",
            2002 => "Can't create temp file",
            2003 => "Can't write temp file",
            2004 => "The temporary file is partially written",
            2005 => "Clock may have gone backwards",
            2006 => "System error with buffer",

            // Ation engine
            3000 => "Wrong cache type key of Redirect",
            3001 => "Wrong cache type key of Route",
            3002 => "Cannot serialize Mail Message",
            3003 => "Cannot get Message-ID",
            3004 => r#"Unable to read "from" mail"#,
            3005 => r#"Unable to read "reply-to" mail"#,
            3006 => r#"Unable to read "to" mail"#,
            3007 => r#"Unable to read "cc" mail"#,
            3008 => r#"Unable to read "bcc" mail"#,
            3009 => "Unable to create mail message",
            3010 => "Unable to get content type from filename",
            3011 => "Cannot read parameter for Mail config",
            3012 => "Cannot send email via sendmail transport",
            3013 => "Cannot send email via file transport",
            3014 => "Cannot send email via smtp transport",
            3015 => "Cannot create dir for file transport",

            _ => "Unknown error"
        }
    }
}
