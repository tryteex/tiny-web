use std::{
    collections::{HashMap, HashSet},
    sync::{atomic::Ordering, Arc},
};

use tokio::sync::RwLock;

use crate::{fnv1a_64, log, sys::wrlock::WrLock};

use super::data::Data;

#[derive(Debug, Eq, Hash, PartialEq)]
enum CacheType {
    Element(i64),
    Group(i64),
}

struct CacheParse {
    key: Option<i64>,
    vec: Option<Vec<i64>>,
}

#[derive(Debug, Default)]
struct CacheData {
    /// Element
    data: HashMap<i64, Data>,
    /// Group -> list of CacheType
    key: HashMap<i64, HashSet<CacheType>>,
}

#[derive(Debug, Default)]
pub struct Cache {
    data: Arc<RwLock<CacheData>>,
    lock: WrLock,
}

impl Cache {
    pub fn new() -> Cache {
        Cache {
            data: Arc::new(RwLock::new(CacheData::default())),
            lock: WrLock::default(),
        }
    }

    /// Converts &str to Vec<i64> with ":" separator and hash function fnv1a_64
    fn get_hash(input: &[u8]) -> Option<CacheParse> {
        let last_symbol = *input.last()?;

        let mut count = 0;
        let mut last = 0;
        for (num, item) in input.iter().enumerate() {
            if *item == b':' {
                if last == num {
                    return None;
                }
                count += 1;
                last = num + 1;
            }
        }

        let key = if last_symbol != b':' { Some(fnv1a_64(input)) } else { None };
        if count == 0 {
            Some(CacheParse { key, vec: None })
        } else {
            let mut vec = Vec::with_capacity(count);
            let mut start = 0;
            while start < input.len() {
                if unsafe { *input.get_unchecked(start) } == b':' {
                    vec.push(fnv1a_64(&input[..start]));
                }
                start += 1;
            }
            Some(CacheParse { key, vec: Some(vec) })
        }
    }

    /// Get cache
    pub async fn get(&self, key: &str) -> Option<Data> {
        let key = key.as_bytes();
        if *key.last()? == b':' {
            return None;
        }
        while self.lock.lock.load(Ordering::Relaxed) {
            self.lock.notify.notified().await;
        }
        let read = self.data.read().await;
        Some(read.data.get(&fnv1a_64(key))?.clone())
    }

    /// Set cache
    pub async fn set(&self, key: &str, data: impl Into<Data>) -> Option<Data> {
        let key = key.as_bytes();

        if *key.last()? == b':' {
            return None;
        }
        let res = match Cache::get_hash(key) {
            Some(res) => res,
            None => {
                log!(warning, 0, "{}", key);
                return None;
            }
        };
        let key = match res.key {
            Some(key) => key,
            None => {
                log!(warning, 0, "{}", key);
                return None;
            }
        };
        loop {
            if self.lock.lock.compare_exchange(false, true, Ordering::SeqCst, Ordering::Relaxed).is_ok() {
                break;
            }
            self.lock.notify.notified().await;
        }
        let data = {
            let mut map = self.data.write().await;
            let data = map.data.insert(key, data.into());
            if data.is_none() {
                if let Some(vec) = res.vec {
                    let mut main = *unsafe { vec.get_unchecked(0) };
                    for slave in &vec[1..] {
                        let val = map.key.entry(main).or_insert_with(HashSet::new);
                        val.insert(CacheType::Group(*slave));
                        main = *slave;
                    }
                    let val = map.key.entry(main).or_insert_with(HashSet::new);
                    val.insert(CacheType::Element(key));
                }
            }
            data
        };
        self.lock.lock.store(false, Ordering::SeqCst);
        self.lock.notify.notify_waiters();
        data
    }

    /// Remove cache
    /// If `key` is &str and ends with a `:` character, all data beginning with that `key` is deleted.
    pub async fn remove(&self, key: &str) {
        let res = match Cache::get_hash(key.as_bytes()) {
            Some(res) => res,
            None => {
                log!(warning, 0, "{}", key);
                return;
            }
        };
        loop {
            if self.lock.lock.compare_exchange(false, true, Ordering::SeqCst, Ordering::Relaxed).is_ok() {
                break;
            }
            self.lock.notify.notified().await;
        }
        let mut map = self.data.write().await;

        match (res.key, res.vec) {
            (Some(key), None) => {
                map.data.remove(&key);
            }
            (None, Some(mut vec)) => {
                let last = match vec.pop() {
                    Some(last) => last,
                    None => {
                        log!(warning, 0, "{}", key);
                        return;
                    }
                };
                Cache::remove_tree(&mut map, last);
                Cache::clean_tree(&mut map, vec, CacheType::Group(last));
            }
            (Some(key), Some(vec)) => {
                map.data.remove(&key);
                Cache::clean_tree(&mut map, vec, CacheType::Element(key));
            }
            (None, None) => {
                log!(warning, 0, "{}", key);
                return;
            }
        }

        self.lock.lock.store(false, Ordering::SeqCst);
        self.lock.notify.notify_waiters();
    }

    fn clean_tree(map: &mut tokio::sync::RwLockWriteGuard<'_, CacheData>, mut vec: Vec<i64>, key: CacheType) {
        if let Some(item) = vec.pop() {
            let mut del = None;
            match map.key.get_mut(&item) {
                Some(part) => {
                    if part.remove(&key) && part.is_empty() {
                        del = Some(item);
                    };
                    Cache::clean_tree(map, vec, CacheType::Group(item));
                }
                None => {
                    log!(warning, 0, "{}", key);
                    return;
                }
            }
            if let Some(item) = del {
                map.key.remove(&item);
            }
        }
    }

    fn remove_tree(map: &mut tokio::sync::RwLockWriteGuard<'_, CacheData>, key: i64) {
        let list = match map.key.remove(&key) {
            Some(list) => list,
            None => {
                log!(warning, 0, "{}", key);
                return;
            }
        };
        for item in list {
            match item {
                CacheType::Element(key) => {
                    map.data.remove(&key);
                }
                CacheType::Group(key) => {
                    Cache::remove_tree(map, key);
                }
            }
        }
    }

    /// Clear all cache
    pub async fn clear(&mut self) {
        loop {
            if self.lock.lock.compare_exchange(false, true, Ordering::SeqCst, Ordering::Relaxed).is_ok() {
                break;
            }
            self.lock.notify.notified().await;
        }
        {
            let mut data = self.data.write().await;
            data.key.clear();
            data.data.clear();
        }
        self.lock.lock.store(false, Ordering::SeqCst);
        self.lock.notify.notify_waiters();
    }
}
