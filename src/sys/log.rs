use std::{cell::OnceCell, path::PathBuf, process};

use chrono::Local;

#[derive(Debug)]
enum LogView {
    Info,
    Warning,
    Stop,
    Error,
    Critical,
}

#[derive(Debug)]
struct LogText<'a> {
    view: LogView,
    number: u16,
    text: Option<String>,
    file: &'a str,
    line: u32,
}

pub(crate) enum InitLog {
    None,
    Path(PathBuf),
    File(String),
}

static mut LOG_FILE: OnceCell<PathBuf> = OnceCell::new();

pub(crate) struct Log;

impl Log {
    /// Save informational message only to log file.
    pub(crate) fn info(number: u16, text: Option<String>, line: u32, file: &str) {
        Log::save(LogText {
            view: LogView::Info,
            number,
            text,
            line,
            file,
        });
    }

    /// Save warning message to log file, the program may continue to run.
    pub(crate) fn warning(number: u16, text: Option<String>, line: u32, file: &str) {
        Log::save(LogText {
            view: LogView::Warning,
            number,
            text,
            line,
            file,
        });
    }

    /// Save stop message to log file, the program must soft stop.
    pub(crate) fn stop(number: u16, text: Option<String>, line: u32, file: &str) {
        Log::save(LogText {
            view: LogView::Stop,
            number,
            text,
            line,
            file,
        });
    }

    /// Save error message to log file, this is abnormal behavior, the program stops immediately.
    pub(crate) fn error(number: u16, text: Option<String>, line: u32, file: &str) -> ! {
        Log::save(LogText {
            view: LogView::Error,
            number,
            text,
            line,
            file,
        });
        panic!("Unpredictable program behavior. Error code {number}.")
    }

    pub(crate) fn init(file: InitLog) {
        let file = match file {
            InitLog::None => PathBuf::new(),
            InitLog::Path(mut path) => {
                path.push("app.log");
                path
            }
            InitLog::File(file) => file.into(),
        };
        unsafe { LOG_FILE = file.into() }
    }

    fn save(log: LogText) {
        let time = Local::now().format("%Y-%m-%d %H:%M:%S%.9f").to_string();
        let text = log.text.unwrap_or_default();
        let str = format!(
            "ID: {} Time: {} Type: {:?}. Number: {} Line:{} File:{} {}\n",
            process::id(),
            time,
            log.view,
            log.number,
            log.line,
            log.file,
            text
        );

        #[cfg(debug_assertions)]
        match log.view {
            LogView::Info => println!("{}", str.trim_end()),
            _ => eprintln!("{}", str.trim_end()),
        }

        let logfile = match unsafe { LOG_FILE.get() } {
            Some(file) => file,
            None => Log::panic("Log is not initialized"),
        };

        match std::fs::OpenOptions::new().create(true).append(true).open(logfile) {
            Ok(mut file) => match std::io::Write::write_all(&mut file, str.as_bytes()) {
                Ok(f) => f,
                Err(e) => Log::panic(&e.to_string()),
            },
            Err(e) => Log::panic(&format!("Can't save data to log file {:?} -> {}", logfile, e)),
        };
    }

    fn panic(text: &str) -> ! {
        let time = Local::now().format("%Y-%m-%d %H:%M:%S%.9f").to_string();
        let str = format!("ID: {} Time: {} Type: {:?}. Number: Text: Panic error -> {}\n", process::id(), time, LogView::Critical, text);
        eprintln!("{}", &str);
        process::exit(1);
    }
}
