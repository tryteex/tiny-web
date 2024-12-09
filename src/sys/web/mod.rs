pub mod action;

#[cfg(feature = "cache")]
pub(crate) mod cache;

pub mod data;

#[cfg(feature = "file-disk")]
pub(crate) mod file;

#[cfg(any(feature = "html-static", feature = "html-reload"))]
pub(crate) mod html;

#[cfg(any(feature = "lang-static", feature = "lang-reload"))]
pub(crate) mod lang;

#[cfg(any(
    feature = "mail-sendmail",
    feature = "mail-smtp",
    feature = "mail-file",
    feature = "mail-db"
))]
pub(crate) mod mail;

pub mod request;

pub mod response;

#[cfg(any(feature = "session-memory", feature = "session-file", feature = "session-db"))]
pub mod session;

#[cfg(all(feature = "html-static", feature = "html-reload"))]
compile_error!("It is impossible to simultaneously have the features of 'html-static' and 'html-reload'");

#[cfg(all(feature = "lang-static", feature = "lang-reload"))]
compile_error!("It is impossible to simultaneously have the features of 'lang-static' and 'lang-reload'");

#[cfg(all(
    any(feature = "lang-static", feature = "lang-reload"),
    not(any(feature = "session-memory", feature = "session-file", feature = "session-db"))
))]
compile_error!("It is impossible to simultaneously have the features of 'lang-static' and 'lang-reload' without 'session-memory', or 'session-file', or 'session-db' features");

#[cfg(any(
    all(feature = "session-memory", any(feature = "session-file", feature = "session-db")),
    all(feature = "session-file", any(feature = "session-memory", feature = "session-db")),
    all(feature = "session-db", any(feature = "session-memory", feature = "session-file"))
))]
compile_error!(
    "It is impossible to simultaneously have either 'session-memory', or 'session-file', or 'session-db' features at the same time"
);

#[cfg(all(feature = "session-db", not(any(feature = "pgsql", feature = "mssql"))))]
compile_error!("Cannot have feature 'session-db'  without 'pgsql' or 'mssql'");

#[cfg(all(feature = "mail-db", not(any(feature = "pgsql", feature = "mssql"))))]
compile_error!("Cannot have features 'mail-sendmail' or 'mail-smtp' or 'mail-file' or 'mail-db' without 'pgsql' or 'mssql'");

#[cfg(any(
    all(
        feature = "mail-sendmail",
        any(feature = "mail-smtp", feature = "mail-file", feature = "mail-db")
    ),
    all(
        feature = "mail-smtp",
        any(feature = "mail-sendmail", feature = "mail-file", feature = "mail-db")
    ),
    all(
        feature = "mail-file",
        any(feature = "mail-sendmail", feature = "mail-smtp", feature = "mail-db")
    ),
    all(
        feature = "mail-db",
        any(feature = "mail-sendmail", feature = "mail-smtp", feature = "mail-file")
    )
))]
compile_error!(
    "It is impossible to simultaneously have either 'mail-sendmail', or 'mail-smtp', or 'mail-file', or 'mail-db' features at the same time"
);
