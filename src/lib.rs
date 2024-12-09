use sys::{app::app::App, web::action::ModuleMap};

/// Show help message
pub(crate) mod help;

/// Web server
pub mod sys;

/// Different useful functions
pub(crate) mod tool;

pub fn run(name: &str, version: &str, desc: &str, func: ModuleMap) -> bool {
    App::run(name, version, desc, func).is_ok()
}

/// fnv1a_64 hash function
#[inline]
pub fn fnv1a_64(bytes: &[u8]) -> i64 {
    let mut hash = 0xcbf29ce484222325;
    for c in bytes {
        hash ^= u64::from(*c);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    unsafe { std::mem::transmute(hash) }
}
