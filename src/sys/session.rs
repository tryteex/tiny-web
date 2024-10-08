use std::{
    collections::{btree_map::Entry, BTreeMap},
    sync::{Arc, LazyLock},
};

use chrono::Local;
use ring::rand::{SecureRandom, SystemRandom};
use sha3::{Digest, Sha3_512};
use tiny_web_macro::fnv1a_64;
use tokio::sync::Mutex;

use crate::{fnv1a_64, StrOrI64};
use tiny_web_macro::fnv1a_64 as m_fnv1a_64;

use super::{data::Data, dbs::adapter::DB, request::Request};

/// Temporary cache for install mode only
static INSTALL_CACHE: LazyLock<Mutex<BTreeMap<i64, Data>>> = LazyLock::new(|| Mutex::new(BTreeMap::new()));

/// Types of flash messages
#[repr(i16)]
#[derive(Debug)]
pub enum Flash {
    Info = 1,
    Success = 2,
    Warning = 3,
    Error = 4,
}

impl From<i16> for Flash {
    fn from(value: i16) -> Self {
        match value {
            1 => Flash::Info,
            2 => Flash::Success,
            3 => Flash::Warning,
            4 => Flash::Error,
            #[cfg(not(debug_assertions))]
            _ => Flash::Error,
            #[cfg(debug_assertions)]
            _ => panic!("Invalid value for Status"),
        }
    }
}

impl From<Flash> for i16 {
    fn from(value: Flash) -> Self {
        value as i16
    }
}

/// User session
///
///  # Values
///
/// * `id: i64` - session_id from database.
/// * `lang_id: i64` - lang_id from database.
/// * `user_id: i64` - user_id from database.
/// * `role_id: i64` - role_id from database.
/// * `pub key: String` - Cookie key.
/// * `data: HashMap<String, Data>` - User data from database.
/// * `change: bool` - User data is changed.
#[derive(Debug)]
pub struct Session {
    /// session_id from database
    id: i64,
    /// Default lang_id for user
    lang_id: i64,
    /// user_id from database
    pub user_id: i64,
    /// role_id from database
    pub role_id: i64,
    /// Cookie key (session value)
    pub key: String,
    /// Session key
    pub session_key: Arc<String>,
    /// User data from database
    data: BTreeMap<i64, Data>,
    /// User data is changed
    change: bool,
}

impl Session {
    /// Create new session
    pub(crate) fn new(lang_id: i64, salt: &str, ip: &str, agent: &str, host: &str, session_key: Arc<String>) -> Session {
        Session {
            id: 0,
            lang_id,
            user_id: 0,
            role_id: 0,
            key: Session::generate_session(salt, ip, agent, host),
            session_key,
            data: BTreeMap::new(),
            change: false,
        }
    }

    /// Create new session by cookie (session) key
    pub(crate) fn with_key(lang_id: i64, key: String, session_key: Arc<String>) -> Session {
        Session {
            id: 0,
            lang_id,
            user_id: 0,
            role_id: 0,
            key,
            session_key,
            data: BTreeMap::new(),
            change: false,
        }
    }

    /// Load session from database
    pub(crate) async fn load_session(key: String, db: Arc<DB>, lang_id: i64, session_key: Arc<String>) -> Session {
        if db.in_use() {
            let res = match db.query_prepare(fnv1a_64!("lib_get_session"), &[&key], false).await {
                Some(r) => r,
                None => return Session::with_key(lang_id, key, session_key),
            };
            if res.is_empty() {
                return Session::with_key(lang_id, key, session_key);
            }
            let row = if let Data::Vec(row) = unsafe { res.get_unchecked(0) } {
                row
            } else {
                return Session::with_key(lang_id, key, session_key);
            };
            if row.len() != 5 {
                return Session::with_key(lang_id, key, session_key);
            }
            let session_id = if let Data::I64(val) = unsafe { row.get_unchecked(0) } {
                *val
            } else {
                return Session::with_key(lang_id, key, session_key);
            };
            let user_id = if let Data::I64(val) = unsafe { row.get_unchecked(1) } {
                *val
            } else {
                return Session::with_key(lang_id, key, session_key);
            };
            let role_id = if let Data::I64(val) = unsafe { row.get_unchecked(2) } {
                *val
            } else {
                return Session::with_key(lang_id, key, session_key);
            };
            let data = if let Data::Raw(val) = unsafe { row.get_unchecked(3) } {
                val.to_owned()
            } else {
                return Session::with_key(lang_id, key, session_key);
            };
            let lang_id = if let Data::I64(val) = unsafe { row.get_unchecked(4) } {
                *val
            } else {
                return Session::with_key(lang_id, key, session_key);
            };

            let res = if data.is_empty() {
                BTreeMap::new()
            } else {
                bincode::deserialize::<BTreeMap<i64, Data>>(&data[..]).unwrap_or_else(|_| BTreeMap::new())
            };
            Session {
                id: session_id,
                lang_id,
                user_id,
                role_id,
                key,
                session_key,
                data: res,
                change: false,
            }
        } else {
            let cache = INSTALL_CACHE.lock().await;
            match cache.get(&fnv1a_64(key.as_bytes())) {
                Some(map) => {
                    let data: BTreeMap<i64, Data> = map.clone().into();
                    let lang_id = if let Some(Data::I64(lang)) = data.get(&m_fnv1a_64!("lang_id")) {
                        *lang
                    } else {
                        return Session::with_key(lang_id, key, session_key);
                    };
                    let data = if let Some(Data::Map(data)) = data.get(&m_fnv1a_64!("data")) {
                        data.clone()
                    } else {
                        return Session::with_key(lang_id, key, session_key);
                    };
                    Session {
                        id: 0,
                        lang_id,
                        user_id: 0,
                        role_id: 0,
                        key,
                        session_key,
                        data,
                        change: false,
                    }
                }
                None => Session::with_key(lang_id, key, session_key),
            }
        }
    }

    /// Save session into database
    pub(crate) async fn save_session(db: Arc<DB>, session: &Session, request: &Request) {
        if session.change {
            if db.in_use() {
                let data = bincode::serialize(&session.data).unwrap_or_else(|_| Vec::new());
                if session.id > 0 {
                    db.execute_prepare(
                        fnv1a_64!("lib_set_session"),
                        &[&session.user_id, &session.lang_id, &data, &request.ip, &request.agent, &session.id],
                    )
                    .await;
                } else {
                    db.execute_prepare(
                        fnv1a_64!("lib_add_session"),
                        &[&session.user_id, &session.lang_id, &session.key, &data, &request.ip, &request.agent],
                    )
                    .await;
                };
            } else {
                let mut cache = INSTALL_CACHE.lock().await;
                let mut data = BTreeMap::new();
                data.insert(m_fnv1a_64!("lang_id"), session.lang_id.into());
                data.insert(m_fnv1a_64!("data"), session.data.clone().into());

                cache.insert(fnv1a_64(session.key.as_bytes()), data.into());
            }
        }
    }

    /// Generete new session key
    fn generate_session(salt: &str, ip: &str, agent: &str, host: &str) -> String {
        // Generate a new cookie
        let time = Local::now().format("%Y.%m.%d %H:%M:%S%.9f %:z").to_string();
        let cook = format!("{}{}{}{}{}", salt, ip, agent, host, time);
        let mut hasher = Sha3_512::new();
        hasher.update(cook.as_bytes());
        format!("{:#x}", hasher.finalize())
    }

    pub fn generate_salt(&self) -> String {
        let time = Local::now().format("%Y.%m.%d %H:%M:%S%.9f %:z").to_string();
        let cook = format!("{}{}", self.key, time);
        let mut hasher = Sha3_512::new();
        hasher.update(cook.as_bytes());
        let rand = format!("!@#$^&*()_-+=,<.>/?|{:#x}", hasher.finalize());
        self.shuffle_string(&rand)
    }

    fn shuffle_string(&self, s: &str) -> String {
        let mut chars: Vec<char> = s.chars().collect();
        let len = chars.len();
        let rng = SystemRandom::new();

        for i in (1..len).rev() {
            let mut buf = [0u8; 8];
            let _ = rng.fill(&mut buf);
            let rand_index = (u64::from_ne_bytes(buf) % (i as u64 + 1)) as usize;
            chars.swap(i, rand_index);
        }
        for c in chars.iter_mut() {
            let mut buf = [0u8; 1];
            let _ = rng.fill(&mut buf);
            if buf[0] % 2 == 0 {
                *c = c.to_ascii_uppercase();
            }
        }
        let mut str: String = chars.into_iter().collect();
        str.truncate(32);
        str
    }

    /// Set lang_id
    pub fn set_lang_id(&mut self, lang_id: i64) {
        if self.lang_id != lang_id {
            self.lang_id = lang_id;
            self.change = true;
        }
    }

    /// Get lang_id
    pub fn get_lang_id(&self) -> i64 {
        self.lang_id
    }

    /// Set session data
    pub fn set<T>(&mut self, key: impl StrOrI64, value: T)
    where
        T: Into<Data>,
    {
        self.change = true;
        self.data.insert(key.to_i64(), value.into());
    }

    /// Get session data for reading
    pub fn get(&mut self, key: impl StrOrI64) -> Option<&Data> {
        self.change = true;
        self.data.get(&key.to_i64())
    }

    /// Getting session data by deleting it
    pub fn take(&mut self, key: impl StrOrI64) -> Option<Data> {
        self.change = true;
        self.data.remove(&key.to_i64())
    }

    /// Remove session data
    pub fn remove(&mut self, key: impl StrOrI64) {
        self.change = true;
        self.data.remove(&key.to_i64());
    }

    /// Clear session data
    pub fn clear(&mut self) {
        self.change = true;
        self.data.clear();
    }

    /// Set flash message to session data
    pub(crate) fn set_flash(&mut self, kind: Flash, value: String) {
        self.change = true;
        match self.data.entry(fnv1a_64!("flash-message")) {
            Entry::Vacant(entry) => {
                entry.insert(vec![Data::I16(kind.into()), Data::String(value)].into());
            }
            Entry::Occupied(mut entry) => {
                let e = entry.get_mut();
                if let Data::Vec(vec) = e {
                    vec.push(Data::I16(kind.into()));
                    vec.push(Data::String(value));
                } else {
                    *e = vec![Data::I16(kind.into()), Data::String(value)].into();
                }
            }
        }
    }

    /// Take flash message from session data
    pub(crate) fn take_flash(&mut self) -> Option<Vec<(Flash, String)>> {
        self.change = true;
        if let Data::Vec(vec) = self.data.remove(&fnv1a_64!("flash-message"))? {
            if vec.len() % 2 != 0 {
                return None;
            }
            let mut result = Vec::with_capacity(vec.len() / 2);
            let mut iter = vec.into_iter();
            while let (Some(first), Some(second)) = (iter.next(), iter.next()) {
                match (first, second) {
                    (Data::I16(num), Data::String(text)) => result.push((num.into(), text)),
                    _ => return None,
                }
            }

            Some(result)
        } else {
            None
        }
    }
}
