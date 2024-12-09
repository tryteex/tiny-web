pub mod stream;

pub mod worker;

#[cfg(feature = "fastcgi")]
pub mod fastcgi;

#[cfg(any(feature = "http", feature = "https"))]
pub mod http;

#[cfg(feature = "scgi")]
pub mod scgi;

#[cfg(feature = "uwsgi")]
pub mod uwsgi;

#[cfg(any(
    not(any(
        feature = "fastcgi",
        feature = "http",
        feature = "https",
        feature = "scgi",
        feature = "uwsgi"
    )),
    all(
        feature = "fastcgi",
        any(feature = "http", feature = "https", feature = "scgi", feature = "uwsgi")
    ),
    all(
        feature = "http",
        any(feature = "fastcgi", feature = "https", feature = "scgi", feature = "uwsgi")
    ),
    all(
        feature = "https",
        any(feature = "fastcgi", feature = "http", feature = "scgi", feature = "uwsgi")
    ),
    all(
        feature = "scgi",
        any(feature = "fastcgi", feature = "http", feature = "https", feature = "uwsgi")
    ),
    all(
        feature = "uwsgi",
        any(feature = "fastcgi", feature = "http", feature = "https", feature = "scgi")
    ),
))]
compile_error!("Only one features from 'fastcgi', 'scgi', 'uwsgi', 'http', 'https' must be enabled for this crate.");
