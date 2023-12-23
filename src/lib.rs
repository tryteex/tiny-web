/// Show help message
pub mod help;
/// Web server
pub mod sys;

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

/// Session paramater name
pub const TINY_KEY: &str = "tinysession";

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
pub fn fnv1a_64(text: &str) -> i64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    let prime: u64 = 0x100000001b3;

    for c in text.bytes() {
        hash ^= u64::from(c);
        hash = hash.wrapping_mul(prime);
    }
    unsafe { *(&hash as *const u64 as *const i64) }
}

/// Trait for types that can be hashed using the FNV-1a algorithm with a 64-bit hash.
pub trait FNV1A64 {
    fn to_i64(&self) -> i64;
    fn to_string(&self) -> String;
}

impl FNV1A64 for i64 {
    fn to_i64(&self) -> i64 {
        *self
    }

    fn to_string(&self) -> String {
        ToString::to_string(self)
    }
}

impl FNV1A64 for &str {
    fn to_i64(&self) -> i64 {
        fnv1a_64(self)
    }

    fn to_string(&self) -> String {
        ToString::to_string(self)
    }
}
