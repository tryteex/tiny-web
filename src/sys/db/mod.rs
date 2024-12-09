/// Database adapter
#[cfg(any(feature = "pgsql", feature = "mssql"))]
pub mod adapter;

/// PostgreSQL database
#[cfg(feature = "pgsql")]
pub mod pgsql;

/// Ms Sql Server database
#[cfg(feature = "mssql")]
pub mod mssql;

#[cfg(all(feature = "pgsql", feature = "mssql"))]
compile_error!("It is impossible to simultaneously have the features of 'pgsql' and 'mssql'");

#[cfg(all(feature = "row-data", feature = "row-native"))]
compile_error!("It is impossible to simultaneously have the features of 'row-data' and 'row-native'");

#[cfg(all(
    any(feature = "row-data", feature = "row-native"),
    not(any(feature = "pgsql", feature = "mssql"))
))]
compile_error!("It is not possible to have 'row-data' or 'row-native' features without 'pgsql' or 'mssql' features");

#[cfg(all(
    any(feature = "pgsql", feature = "mssql"),
    not(any(feature = "row-data", feature = "row-native"))
))]
compile_error!("It is not possible to have 'pgsql' or 'mssql' features without 'row-data' or 'row-native' features");

#[cfg(all(feature = "redirect-db", not(any(feature = "pgsql", feature = "mssql"))))]
compile_error!("Cannot have feature 'redirect-db'  without 'pgsql' or 'mssql'");

#[cfg(all(feature = "route-db", not(any(feature = "pgsql", feature = "mssql"))))]
compile_error!("Cannot have feature 'route-db'  without 'pgsql' or 'mssql'");

#[cfg(all(feature = "session-db", not(any(feature = "pgsql", feature = "mssql"))))]
compile_error!("Cannot have feature 'session-db'  without 'pgsql' or 'mssql'");

#[cfg(all(
    feature = "access-db",
    not(all(any(feature = "pgsql", feature = "mssql"), feature = "session-db"))
))]
compile_error!("Cannot have feature 'access-db' without 'pgsql' or 'mssql' and without 'session-db'");

#[cfg(all(feature = "setting-db", not(any(feature = "pgsql", feature = "mssql"))))]
compile_error!("Cannot have feature 'setting-db'  without 'pgsql' or 'mssql'");

#[cfg(all(feature = "session-db", not(any(feature = "pgsql", feature = "mssql"))))]
compile_error!("Cannot have feature 'session-db'  without 'pgsql' or 'mssql'");
