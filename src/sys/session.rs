use std::{collections::BTreeMap, sync::Arc};

use chrono::Local;
use sha3::{Digest, Sha3_512};
use tiny_web_macro::fnv1a_64;

use crate::StrOrI64;

use super::{action::Request, data::Data, dbs::adapter::DB};

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
    pub(crate) session_key: Arc<String>,
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
        if !db.in_use() {
            return Session {
                id: 0,
                lang_id,
                user_id: 0,
                role_id: 0,
                key,
                session_key,
                data: BTreeMap::new(),
                change: false,
            };
        }
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
        let data = if let Data::Raw(val) = unsafe { row.get_unchecked(2) } {
            val.to_owned()
        } else {
            return Session::with_key(lang_id, key, session_key);
        };
        let lang_id = if let Data::I64(val) = unsafe { row.get_unchecked(4) } {
            *val
        } else {
            return Session::with_key(lang_id, key, session_key);
        };

        let res =
            if data.is_empty() { BTreeMap::new() } else { bincode::deserialize::<BTreeMap<i64, Data>>(&data[..]).unwrap_or_default() };
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
    }

    /// Save session into database
    pub(crate) async fn save_session(db: Arc<DB>, session: &Session, request: &Request) {
        if !db.in_use() {
            return;
        }
        if session.change {
            let data = bincode::serialize(&session.data).unwrap_or_default();
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
    pub fn set(&mut self, key: impl StrOrI64, value: Data) {
        self.change = true;
        self.data.insert(key.to_i64(), value);
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
}
