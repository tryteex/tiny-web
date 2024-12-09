pub(crate) mod app;

pub(crate) mod db;

#[cfg(any(feature = "debug-v", feature = "debug-vv", feature = "debug-vvv"))]
pub(crate) mod log;

pub(crate) mod net;

pub(crate) mod plugin;

pub(crate) mod stat;

pub mod web;

#[cfg(any(feature = "html-reload", feature = "lang-reload", feature = "cache"))]
pub(crate) mod wrlock;

#[cfg(any(
    all(feature = "debug-v", any(feature = "debug-vv", feature = "debug-vvv")),
    all(feature = "debug-vv", feature = "debug-vvv")
))]
compile_error!("Only one features from 'debug-v', 'debug-vv', 'debug-vv' can be enabled for this crate.");

#[macro_export]
macro_rules! log {
    ($level:ident, $number:expr) => {
        {
            #[cfg(any(feature = "debug-v", feature = "debug-vv", feature = "debug-vvv"))]
            {
                $crate::sys::log::Log::$level($number, None, line!(), file!());
            }
        }
    };
    ($level:ident, $number:expr, $fmt:expr, $($arg:tt)*) => {
        {
            #[cfg(any(feature = "debug-v", feature = "debug-vv", feature = "debug-vvv"))]
            {
                $crate::sys::log::Log::$level($number, Some(format!($fmt, $($arg)*)), line!(), file!());
            }
        }
    };
}

#[macro_export]
macro_rules! log_v {
    ($($arg:tt)*) => {
        log!($($arg)*);
    };
}

#[macro_export]
macro_rules! log_vv {
    ($level:ident, $number:expr) => {
        {
            #[cfg(any(feature = "debug-vv", feature = "debug-vvv"))]
            {
                $crate::sys::log::Log::$level($number, None, line!(), file!());
            }
        }
    };
    ($level:ident, $number:expr, $fmt:expr, $($arg:tt)*) => {
        {
            #[cfg(any(feature = "debug-vv", feature = "debug-vvv"))]
            {
                $crate::sys::log::Log::$level($number, Some(format!($fmt, $($arg)*)), line!(), file!());
            }
        }
    };
}

#[macro_export]
macro_rules! log_vvv {
    ($level:ident, $number:expr) => {
        {
            #[cfg(feature = "7-vvv")]
            {
                $crate::sys::log::Log::$level($number, None, line!(), file!());
            }
        }
    };
    ($level:ident, $number:expr, $fmt:expr, $($arg:tt)*) => {
        {
            #[cfg(feature = "debug-vvv")]
            {
                $crate::sys::log::Log::$level($number, Some(format!($fmt, $($arg)*)), line!(), file!());
            }
        }
    };
}
