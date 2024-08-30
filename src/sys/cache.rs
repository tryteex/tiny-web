use std::{
    collections::{btree_map::Entry, BTreeMap},
    sync::Arc,
};

use tokio::sync::Mutex;

use crate::fnv1a_64;

use super::data::Data;

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
pub(crate) struct CacheSys {
    /// Cache data.
    data: CacheKey,
}

#[derive(Debug)]
enum CacheKey {
    Last(Data),
    More(BTreeMap<i64, CacheKey>),
}

impl CacheSys {
    /// Initializes data caching module
    pub async fn new() -> Arc<Mutex<CacheSys>> {
        Arc::new(Mutex::new(CacheSys { data: CacheKey::More(BTreeMap::new()) }))
    }

    /// Returns cloned data
    pub async fn get(cache: Arc<Mutex<CacheSys>>, keys: &[i64]) -> Option<Data> {
        if keys.is_empty() {
            return None;
        }

        let c = cache.lock().await;
        let mut last = &c.data;
        for key in keys {
            match last {
                CacheKey::Last(_) => return None,
                CacheKey::More(m) => {
                    last = m.get(key)?;
                }
            }
        }
        match last {
            CacheKey::Last(d) => Some(d.clone()),
            CacheKey::More(_) => None,
        }
    }

    /// Inserts a data
    pub async fn set(cache: Arc<Mutex<CacheSys>>, keys: &[i64], data: Data) {
        if keys.is_empty() {
            return;
        }
        let key_last = unsafe { keys.get_unchecked(keys.len() - 1) };
        let mut c = cache.lock().await;
        let mut last = &mut c.data;

        for key in &keys[..keys.len() - 1] {
            match last {
                CacheKey::Last(_) => {
                    *last = CacheKey::More(BTreeMap::new());
                    match last {
                        CacheKey::More(m) => {
                            last = m.entry(*key).or_insert_with(|| CacheKey::More(BTreeMap::new()));
                        }
                        _ => unreachable!(),
                    }
                }
                CacheKey::More(m) => {
                    last = m.entry(*key).or_insert_with(|| CacheKey::More(BTreeMap::new()));
                }
            }
        }
        match last {
            CacheKey::Last(_) => {
                *last = CacheKey::More(BTreeMap::new());
                match last {
                    CacheKey::More(m) => match m.entry(*key_last) {
                        Entry::Vacant(v) => {
                            v.insert(CacheKey::Last(data));
                        }
                        Entry::Occupied(mut o) => {
                            let v = o.get_mut();
                            *v = CacheKey::Last(data);
                        }
                    },
                    _ => unreachable!(),
                }
            }
            CacheKey::More(m) => match m.entry(*key_last) {
                Entry::Vacant(v) => {
                    v.insert(CacheKey::Last(data));
                }
                Entry::Occupied(mut o) => {
                    let v = o.get_mut();
                    *v = CacheKey::Last(data);
                }
            },
        }
    }

    /// Removes a key.
    ///
    /// If `key` ends with a `:` character, all data beginning with that `key` is deleted.
    ///
    /// # Safety
    ///
    /// With a large cache, this operation can block it for a long time.
    pub async fn del(cache: Arc<Mutex<CacheSys>>, keys: &[i64]) {
        if keys.is_empty() {
            return;
        }
        let key_last = unsafe { keys.get_unchecked(keys.len() - 1) };
        let mut c = cache.lock().await;
        let mut last = &mut c.data;
        for key in &keys[..keys.len() - 1] {
            match last {
                CacheKey::Last(_) => return,
                CacheKey::More(m) => {
                    match m.get_mut(key) {
                        Some(s) => last = s,
                        None => return,
                    };
                }
            }
        }
        if let CacheKey::More(m) = last {
            m.remove(key_last);
        }
    }

    /// Clear all cache
    pub async fn clear(cache: Arc<Mutex<CacheSys>>) {
        let mut c = cache.lock().await;
        if let CacheKey::More(m) = &mut c.data {
            m.clear()
        }
    }

    /// Converts &str to Vec<i64> with ":" separator and hash function fnv1a_64
    pub fn get_hash(key: &str) -> Vec<i64> {
        let mut count = 1;
        let keys = key.as_bytes();
        for item in keys {
            if *item == b':' {
                count += 1;
            }
        }
        let mut vec = Vec::with_capacity(count);
        if count > 1 {
            let mut start = 0;
            let mut index = 0;
            for item in keys {
                if *item == b':' {
                    vec.push(fnv1a_64(&keys[start..index]));
                    start = index + 1;
                }
                index += 1;
            }
            vec.push(fnv1a_64(&keys[start..index]));
        } else {
            vec.push(fnv1a_64(key.as_bytes()));
        }
        vec
    }

    pub async fn show(cache: Arc<Mutex<CacheSys>>) {
        let c = cache.lock().await;
        println!("{:?}", c.data)
    }
}

pub trait StrOrArrI64 {
    fn to_arr(self) -> Vec<i64>;
}

impl StrOrArrI64 for &str {
    fn to_arr(self) -> Vec<i64> {
        CacheSys::get_hash(self)
    }
}
impl StrOrArrI64 for Vec<i64> {
    fn to_arr(self) -> Vec<i64> {
        self
    }
}

/// Cache struct
#[derive(Debug)]
pub struct Cache {
    cache: Arc<Mutex<CacheSys>>,
}

impl Cache {
    /// Create new Cache instanse
    pub(crate) fn new(cache: Arc<Mutex<CacheSys>>) -> Cache {
        Cache { cache }
    }

    /// Get cache
    pub async fn get<T>(&mut self, keys: T) -> (Option<Data>, Vec<i64>)
    where
        T: StrOrArrI64,
    {
        let key = keys.to_arr();
        (CacheSys::get(Arc::clone(&self.cache), &key).await, key)
    }

    /// Set cache
    pub async fn set<T>(&mut self, keys: T, data: Data)
    where
        T: StrOrArrI64,
    {
        CacheSys::set(Arc::clone(&self.cache), &keys.to_arr(), data).await
    }

    /// Removes a key from the Cache.
    ///
    /// If `key` ends with a `:` character, all data beginning with that `key` is deleted.
    pub async fn remove<T>(&mut self, keys: T)
    where
        T: StrOrArrI64,
    {
        CacheSys::del(Arc::clone(&self.cache), &keys.to_arr()).await
    }

    /// Clear all cache
    pub async fn clear(&mut self) {
        CacheSys::clear(Arc::clone(&self.cache)).await
    }

    /// Show all data in cache
    pub async fn show(&mut self) {
        CacheSys::show(Arc::clone(&self.cache)).await
    }
}
