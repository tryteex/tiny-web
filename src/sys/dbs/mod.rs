/// Database adapter
pub mod adapter;

/// PostgreSQL database
#[cfg(feature = "pgsql")]
pub mod pgsql;

/// Ms Sql Server database
#[cfg(feature = "mssql")]
pub mod mssql;

#[cfg(not(any(feature = "pgsql", feature = "mssql")))]
pub mod without_sql;

#[cfg(not(any(feature = "pgsql", feature = "mssql")))]
compile_error!("Either feature \"pgsql\" or \"mssql\" must be enabled for this crate.");
