use std::{collections::BTreeMap, sync::Arc};

use chrono::Local;
use sha3::{Digest, Sha3_512};
use tiny_web_macro::fnv1a_64;

use crate::StrOrI64;

use super::{
    action::{Data, Request},
    db::DB,
};

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
    pub(crate) role_id: i64,
    /// Cookie key
    pub key: String,
    /// User data from database
    data: BTreeMap<i64, Data>,
    /// User data is changed
    change: bool,
}

impl Session {
    /// Create new session
    pub(crate) fn new(lang_id: i64, salt: &str, ip: &str, agent: &str, host: &str) -> Session {
        Session {
            id: 0,
            lang_id,
            user_id: 0,
            role_id: 0,
            key: Session::generate_session(salt, ip, agent, host),
            data: BTreeMap::new(),
            change: false,
        }
    }

    /// Create new session by cookie (session) key
    pub(crate) fn with_key(lang_id: i64, key: String) -> Session {
        Session {
            id: 0,
            lang_id,
            user_id: 0,
            role_id: 0,
            key,
            data: BTreeMap::new(),
            change: false,
        }
    }

    /// Load session from database
    pub(crate) async fn load_session(key: String, db: Arc<DB>, lang_id: i64) -> Session {
        let res = match db.query_raw(fnv1a_64!("lib_get_session"), &[&key]).await {
            Some(r) => r,
            None => return Session::with_key(lang_id, key),
        };
        if res.is_empty() {
            return Session::with_key(lang_id, key);
        }
        let row = &res[0];
        let session_id: i64 = row.get(0);
        let user_id: i64 = row.get(1);
        let role_id: i64 = row.get(2);
        let data: &[u8] = row.get(3);
        let lang_id: i64 = row.get(4);

        let res = if data.is_empty() {
            BTreeMap::new()
        } else {
            match bincode::deserialize::<BTreeMap<i64, Data>>(data) {
                Ok(r) => r,
                Err(_) => BTreeMap::new(),
            }
        };
        Session {
            id: session_id,
            lang_id,
            user_id,
            role_id,
            key,
            data: res,
            change: false,
        }
    }

    /// Save session into database
    pub(crate) async fn save_session(db: Arc<DB>, session: &Session, request: &Request) {
        if session.change {
            let data = match bincode::serialize(&session.data) {
                Ok(r) => r,
                Err(_) => Vec::new(),
            };
            if session.id > 0 {
                db.query_raw(
                    fnv1a_64!("lib_set_session"),
                    &[&session.user_id, &session.lang_id, &data, &request.ip, &request.agent, &session.id],
                )
                .await;
            } else {
                db.query_raw(
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
    pub fn read(&mut self, key: impl StrOrI64) -> Option<&Data> {
        self.change = true;
        self.data.get(&key.to_i64())
    }

    /// Getting session data by deleting it
    pub fn get(&mut self, key: impl StrOrI64) -> Option<Data> {
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
