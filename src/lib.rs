/// Show help message
pub mod help;
/// Web server
pub mod sys;

use std::mem::transmute;

use sys::{action::ActMap, app::App, log::Log};

/// Entry point
pub fn run(name: &str, version: &str, desc: &str, func: impl Fn() -> ActMap) {
    let app = match App::new(name, version, desc) {
        Some(a) => a,
        None => return,
    };
    Log::info(200, None);
    app.run(func);
    Log::info(201, None);
}

/// fnv1a_64 hash function
///
/// # Parameters
///
/// * `text: &str` - Origin string.
///
/// # Return
///
/// i64 hash
#[inline]
pub fn fnv1a_64(bytes: &[u8]) -> i64 {
    let mut hash = 0xcbf29ce484222325;
    for c in bytes {
        hash ^= u64::from(*c);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    unsafe { transmute(hash) }
}

/// Trait `StrOrI64` defines a method for converting different types to `i64`.
pub trait StrOrI64 {
    /// The `to_i64` method takes input data and returns an `i64`.
    fn to_i64(&self) -> i64;
    /// The `to_str` method takes input data and returns an `&str`.
    fn to_str(&self) -> &str;
}

impl StrOrI64 for i64 {
    /// Implementation of the `to_i64` method for the `i64` type.  
    /// Simply returns the input `i64`.
    fn to_i64(&self) -> i64 {
        *self
    }
    /// The `to_str` method takes input data and returns an `&str`.
    fn to_str(&self) -> &str {
        ""
    }
}

impl StrOrI64 for &str {
    /// Implementation of the `to_i64` method for the `&str` type.  
    /// Computes the hash of the `&str` and returns it as an `i64`.
    fn to_i64(&self) -> i64 {
        fnv1a_64(self.as_bytes())
    }
    /// The `to_str` method takes input data and returns an `&str`.
    /// Simply returns the input `i64`.
    fn to_str(&self) -> &str {
        self
    }
}
