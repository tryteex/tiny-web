use std::fmt;
use std::{borrow::Cow, net::IpAddr, sync::Arc, time::SystemTime};

use crate::sys::{data::Data, init::DBConfig};

use super::adapter::{KeyOrQuery, StrOrI64OrUSize};

pub trait ToSql: fmt::Debug + Send + Sync {
    fn to_sql(&self)
    where
        Self: Sized;
}

/// Responsible for working with MsSql database
#[derive(Debug)]
pub(crate) struct WithoutSql;

impl WithoutSql {
    /// Initializes a new object `PgSql`
    pub fn new(_config: Arc<DBConfig>) -> Option<WithoutSql> {
        None
    }

    /// Connect to the database
    pub async fn connect(&mut self) -> bool {
        false
    }

    /// Execute query to database and return a result
    pub async fn query(&mut self, _query: &impl KeyOrQuery, _params: &[&dyn ToSql], _assoc: bool) -> Option<Vec<Data>> {
        None
    }

    /// Execute query to database without a result
    pub async fn execute(&mut self, _query: &impl KeyOrQuery, _params: &[&dyn ToSql]) -> Option<()> {
        None
    }

    /// Execute query to database and return a result,  
    /// and grouping tabular data according to specified conditions.
    pub async fn query_group(
        &mut self,
        _query: &impl KeyOrQuery,
        _params: &[&dyn ToSql],
        _assoc: bool,
        _conds: &[&[impl StrOrI64OrUSize]],
    ) -> Option<Data> {
        None
    }
}

impl<'a, T> ToSql for &'a T
where
    T: ToSql,
{
    fn to_sql(&self) {}
}

impl<T: ToSql> ToSql for Option<T> {
    fn to_sql(&self) {}
}

impl<'a, T: ToSql> ToSql for &'a [T] {
    fn to_sql(&self) {}
}

impl<'a> ToSql for &'a [u8] {
    fn to_sql(&self) {}
}

impl<T: ToSql> ToSql for Vec<T> {
    fn to_sql(&self) {}
}

impl<T: ToSql> ToSql for Box<[T]> {
    fn to_sql(&self) {}
}

impl<'a> ToSql for Cow<'a, [u8]> {
    fn to_sql(&self) {}
}

impl ToSql for Vec<u8> {
    fn to_sql(&self) {}
}

impl<'a> ToSql for &'a str {
    fn to_sql(&self) {}
}

impl<'a> ToSql for Cow<'a, str> {
    fn to_sql(&self) {}
}

impl ToSql for String {
    fn to_sql(&self) {}
}

impl ToSql for Box<str> {
    fn to_sql(&self) {}
}

macro_rules! simple_to {
    ($t:ty, $f:ident, $($expected:ident),+) => {
        impl ToSql for $t {
            fn to_sql(&self) {}
        }
    };
}

simple_to!(bool, bool_to_sql, BOOL);
simple_to!(i8, char_to_sql, CHAR);
simple_to!(i16, int2_to_sql, INT2);
simple_to!(i32, int4_to_sql, INT4);
simple_to!(u32, oid_to_sql, OID);
simple_to!(i64, int8_to_sql, INT8);
simple_to!(f32, float4_to_sql, FLOAT4);
simple_to!(f64, float8_to_sql, FLOAT8);

impl ToSql for SystemTime {
    fn to_sql(&self) {}
}

impl ToSql for IpAddr {
    fn to_sql(&self) {}
}
