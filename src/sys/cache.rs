use std::{collections::BTreeMap, sync::Arc};

use tokio::sync::Mutex;

use crate::fnv1a_64;

use super::action::Data;

/// Data caching module.
///
/// # Values
///
/// * `data: BTreeMap<u64, Data>` - Cache data;
/// * `keys: Vec<String>` - Stored keys.
///
/// # Notice
///
/// All `Keys` encode in fnv1a_64.
///
/// A cache is a simple set of `Key`=`Value`.
/// A `key` is a unique string, but can be grouped by pattern.
/// To do this, you need to insert the symbol `:` into the line.
/// This does not change the logic of the cache in any way, other than removing data from it.
///
/// If `key` ends with a `:`, all data beginning with that key is deleted.
///
/// # Security
///
/// With a large cache, the `del` operation can block it for a long time.
#[derive(Debug)]
pub struct Cache {
    /// Cache data.
    data: BTreeMap<i64, Data>,
    /// Stored keys.
    keys: Vec<String>,
}

impl Cache {
    /// Initializes data caching module
    pub async fn new() -> Arc<Mutex<Cache>> {
        Arc::new(Mutex::new(Cache {
            data: BTreeMap::new(),
            keys: Vec::new(),
        }))
    }

    /// Returns cloned data
    pub async fn get(cache: Arc<Mutex<Cache>>, key: &str) -> Option<Data> {
        let key = fnv1a_64(key);
        let c = cache.lock().await;
        let d = c.data.get(&key);
        d.cloned()
    }

    /// Inserts a data
    pub async fn set(cache: Arc<Mutex<Cache>>, key: String, data: Data) {
        let key_u64 = fnv1a_64(&key);
        let mut c = cache.lock().await;
        if c.data.insert(key_u64, data).is_none() {
            c.keys.push(key);
        }
    }

    /// Removes a key.
    ///
    /// If `key` ends with a `:` character, all data beginning with that `key` is deleted.
    ///
    /// # Safety
    ///
    /// With a large cache, this operation can block it for a long time.
    pub async fn del(cache: Arc<Mutex<Cache>>, key: &str) {
        if key.ends_with(':') {
            let mut c = cache.lock().await;
            let mut vec = Vec::with_capacity(c.keys.len());
            c.keys.retain(|v| {
                if v.starts_with(key) {
                    vec.push(fnv1a_64(v));
                    false
                } else {
                    true
                }
            });
            for key in vec {
                c.data.remove(&key);
            }
        } else {
            let key_u64 = fnv1a_64(key);
            let mut c = cache.lock().await;
            if c.data.remove(&key_u64).is_some() {
                c.keys.retain(|v| v != key);
            }
        }
    }

    /// Clear all cache
    pub async fn clear(cache: Arc<Mutex<Cache>>) {
        let mut c = cache.lock().await;
        c.data.clear();
        c.keys.clear();
    }
}
