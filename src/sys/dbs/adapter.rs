use postgres::types::Type;

/// External prepare statements
#[derive(Debug, Clone)]
pub struct DBPrepare {
    /// Query string
    pub query: String,
    /// Prepare types
    pub types: Vec<Type>,
}

/// Trait representing types that can be converted to a query or a key statement.
pub trait KeyOrQuery {
    /// Return key
    fn to_i64(&self) -> i64;
    /// Return text of query
    fn to_str(&self) -> &str;
    /// If value is key
    fn is_key(&self) -> bool;
}

impl KeyOrQuery for i64 {
    /// Return key
    fn to_i64(&self) -> i64 {
        *self
    }

    /// Return text of query
    fn to_str(&self) -> &str {
        "key_statement"
    }

    fn is_key(&self) -> bool {
        true
    }
}

impl KeyOrQuery for &str {
    /// Return key
    fn to_i64(&self) -> i64 {
        0
    }

    /// Return text of query
    fn to_str(&self) -> &str {
        self
    }

    fn is_key(&self) -> bool {
        false
    }
}

/// A trait representing types that can be converted to either `i64` or `usize`.
pub trait StrOrI64OrUSize {
    /// Converts the implementor to an `i64`.
    fn to_i64(&self) -> i64;

    /// Converts the implementor to a `usize`.
    fn to_usize(&self) -> usize;
}

impl StrOrI64OrUSize for i64 {
    /// Converts `i64` to itself.
    fn to_i64(&self) -> i64 {
        *self
    }

    /// Converts `i64` to `usize`, always returning `0`.
    fn to_usize(&self) -> usize {
        usize::MAX
    }
}

impl StrOrI64OrUSize for &str {
    /// Converts `&str` to an `i64` using the FNV1a hash algorithm.
    fn to_i64(&self) -> i64 {
        crate::fnv1a_64(self.as_bytes())
    }

    /// Converts `&str` to `usize`, always returning `0`.
    fn to_usize(&self) -> usize {
        usize::MAX
    }
}

impl StrOrI64OrUSize for usize {
    /// Converts `usize` to `i64`, always returning `0`.
    fn to_i64(&self) -> i64 {
        0
    }

    /// Converts `usize` to itself.
    fn to_usize(&self) -> usize {
        *self
    }
}
