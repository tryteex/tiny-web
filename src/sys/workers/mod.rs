/// Connection by protocol FastCGI
#[cfg(feature = "fastcgi")]
pub mod fastcgi;

// /// Connection by protocol GRPC
// pub mod grpc;

/// Connection by protocol HTTP
#[cfg(any(feature = "http", feature = "https"))]
pub mod http;

/// Connection by protocol SCGI
#[cfg(feature = "scgi")]
pub mod scgi;

/// Connection by protocol UWSGI
#[cfg(feature = "uwsgi")]
pub mod uwsgi;

// /// Connection by protocol WebSocket
// pub mod websocket;

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
