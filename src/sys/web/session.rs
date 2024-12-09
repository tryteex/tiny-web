use std::{
    collections::{hash_map::Entry, HashMap},
    mem::take,
    sync::Arc,
};

#[cfg(any(feature = "session-memory", feature = "session-file"))]
use std::io::ErrorKind;

#[cfg(any(feature = "session-memory", feature = "session-file"))]
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[cfg(feature = "session-db")]
use tiny_web_macro::fnv1a_64 as m_fnv1a_64;

#[cfg(any(feature = "session-memory", feature = "session-file"))]
use tokio::fs::{read, remove_file, write};

#[cfg(feature = "session-memory")]
use tokio::sync::Mutex;

use crate::fnv1a_64;

use crate::{log, tool::generate_uuid};

#[cfg(feature = "session-db")]
use crate::sys::db::adapter::DB;

use super::data::{Data, StrOrI64};

#[repr(u8)]
#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq, Serialize, Deserialize, Hash)]
pub enum Flash {
    Info = 1,
    Success = 2,
    Warning = 3,
    Error = 4,
}

impl From<u8> for Flash {
    fn from(value: u8) -> Self {
        match value {
            1 => Flash::Info,
            2 => Flash::Success,
            3 => Flash::Warning,
            _ => Flash::Error,
        }
    }
}

impl From<Flash> for u8 {
    fn from(value: Flash) -> Self {
        value as u8
    }
}
#[cfg(feature = "session-file")]
const PATH_DEEP: usize = 6;

pub(crate) struct SessionArg {
    /// Session key
    pub session_key: Arc<String>,
    #[cfg(any(feature = "session-memory", feature = "session-file"))]
    pub session_path: Arc<PathBuf>,
    #[cfg(feature = "session-db")]
    pub db: Arc<DB>,
}

/// User session
#[derive(Debug)]
pub struct SessionLoader {
    /// Session key
    pub session_key: Arc<String>,
    #[cfg(feature = "session-db")]
    db: Arc<DB>,
    #[cfg(feature = "session-memory")]
    data: Arc<Mutex<HashMap<i64, Session>>>,
    #[cfg(any(feature = "session-memory", feature = "session-file"))]
    pub session_path: PathBuf,
}

impl SessionLoader {
    pub(crate) async fn start(arg: SessionArg) -> Result<SessionLoader, ()> {
        #[cfg(feature = "session-file")]
        let root = arg.session_path.as_ref().clone();

        #[cfg(feature = "session-memory")]
        let mut root = arg.session_path.as_ref().clone();
        #[cfg(feature = "session-memory")]
        root.push("session.bin");

        #[cfg(feature = "session-memory")]
        let data = if root.exists() {
            if !root.is_file() {
                log!(stop, 0, "{:?}", root);
                return Err(());
            }
            let data = match read(&root).await {
                Ok(data) => data,
                Err(_e) => {
                    log!(stop, 0, "{}", _e);
                    return Err(());
                }
            };
            match bincode::deserialize::<HashMap<i64, Session>>(&data) {
                Ok(data) => data,
                Err(_e) => {
                    log!(stop, 0, "{}", _e);
                    return Err(());
                }
            }
        } else {
            HashMap::new()
        };

        Ok(SessionLoader {
            session_key: arg.session_key,
            #[cfg(feature = "session-db")]
            db: arg.db,
            #[cfg(feature = "session-memory")]
            data: Arc::new(Mutex::new(data)),
            #[cfg(any(feature = "session-memory", feature = "session-file"))]
            session_path: root,
        })
    }

    pub(crate) async fn stop(self: Arc<SessionLoader>) -> Result<(), ()> {
        #[cfg(feature = "session-memory")]
        {
            let lock = self.data.lock().await;
            if !lock.is_empty() {
                let data = match bincode::serialize(&*lock) {
                    Ok(data) => data,
                    Err(_e) => {
                        log!(stop, 0, "{}", _e);
                        return Err(());
                    }
                };
                if let Err(_e) = write(&self.session_path, data).await {
                    log!(stop, 0, "{}", _e);
                    return Err(());
                }
            } else if let Err(e) = remove_file(&self.session_path).await {
                if e.kind() != ErrorKind::NotFound {
                    log!(stop, 0, "{}", e);
                    return Err(());
                }
            }
        }
        Ok(())
    }

    pub(crate) async fn load(&self, session: Option<String>) -> Result<Session, ()> {
        let s = match session {
            Some(session) => {
                let key = fnv1a_64(session.as_bytes());
                #[cfg(feature = "session-memory")]
                match self.data.lock().await.get(&key) {
                    Some(s) => {
                        let mut s = s.clone();
                        s.session = session;
                        s.change = false;
                        s
                    }
                    None => Session {
                        session,
                        data: HashMap::new(),
                        flash: HashMap::new(),
                        #[cfg(any(feature = "lang-static", feature = "lang-reload"))]
                        lang_id: None,
                        #[cfg(feature = "access-db")]
                        role_id: None,
                        #[cfg(feature = "access-db")]
                        user_id: None,
                        change: false,
                    },
                }
                #[cfg(feature = "session-file")]
                let path = self.generate_path(key);
                #[cfg(feature = "session-file")]
                if path.exists() {
                    if !path.is_file() {
                        log!(stop, 0, "{:?}", path);
                        return Err(());
                    }
                    let data = match read(&path).await {
                        Ok(data) => data,
                        Err(_e) => {
                            log!(stop, 0, "{}", _e);
                            return Err(());
                        }
                    };
                    let mut s = match bincode::deserialize::<Session>(&data) {
                        Ok(data) => data,
                        Err(_e) => {
                            log!(stop, 0, "{}", _e);
                            return Err(());
                        }
                    };
                    s.session = session;
                    s.path = Some(path);
                    s.new = false;
                    s.change = false;
                    s
                } else {
                    Session {
                        session,
                        data: HashMap::new(),
                        flash: HashMap::new(),
                        #[cfg(any(feature = "lang-static", feature = "lang-reload"))]
                        lang_id: None,
                        #[cfg(feature = "access-db")]
                        role_id: None,
                        #[cfg(feature = "access-db")]
                        user_id: None,
                        change: false,
                        path: Some(path),
                        new: true,
                    }
                }
                #[cfg(feature = "session-db")]
                match self.db.query_prepare(m_fnv1a_64!("lib_get_session"), &[&key]).await {
                    Some(res) => {
                        if res.is_empty() {
                            Session {
                                session,
                                data: HashMap::new(),
                                flash: HashMap::new(),
                                #[cfg(any(feature = "lang-static", feature = "lang-reload"))]
                                lang_id: None,
                                #[cfg(feature = "access-db")]
                                role_id: None,
                                #[cfg(feature = "access-db")]
                                user_id: None,
                                change: false,
                                new: true,
                            }
                        } else {
                            let row = unsafe { res.get_unchecked(0) };
                            let mut s = {
                                let data: Vec<u8> = row.get(2);
                                match bincode::deserialize::<Session>(&data) {
                                    Ok(data) => data,
                                    Err(_e) => {
                                        log!(stop, 0, "{}", _e);
                                        return Err(());
                                    }
                                }
                            };
                            s.session = session;
                            s.change = false;
                            s.new = false;
                            s
                        }
                    }
                    None => Session {
                        session,
                        data: HashMap::new(),
                        flash: HashMap::new(),
                        #[cfg(any(feature = "lang-static", feature = "lang-reload"))]
                        lang_id: None,
                        #[cfg(feature = "access-db")]
                        role_id: None,
                        #[cfg(feature = "access-db")]
                        user_id: None,
                        change: false,
                        new: true,
                    },
                }
            }
            None => Session {
                session: generate_uuid(),
                data: HashMap::new(),
                flash: HashMap::new(),
                #[cfg(any(feature = "lang-static", feature = "lang-reload"))]
                lang_id: None,
                #[cfg(feature = "access-db")]
                role_id: None,
                #[cfg(feature = "access-db")]
                user_id: None,
                change: false,
                #[cfg(feature = "session-file")]
                path: None,
                #[cfg(any(feature = "session-file", feature = "session-db"))]
                new: true,
            },
        };
        Ok(s)
    }

    #[cfg(feature = "session-file")]
    fn generate_path(&self, key: i64) -> PathBuf {
        let num_str = format!("{:0>width$}", key, width = PATH_DEEP + 1);
        let mut full_path = self.session_path.clone();

        for c in num_str.chars() {
            full_path.push(c.to_string());
        }
        full_path.push(num_str + ".bin");
        full_path
    }

    pub(crate) async fn save(&self, session: Session) -> Result<(), ()> {
        if session.change {
            #[cfg(feature = "session-memory")]
            {
                let key = fnv1a_64(session.session.as_bytes());
                if session.data.is_empty() && session.flash.is_empty() {
                    self.data.lock().await.remove(&key);
                } else {
                    self.data.lock().await.insert(key, session);
                }
            }
            #[cfg(feature = "session-file")]
            if session.data.is_empty() && session.flash.is_empty() {
                if !session.new {
                    let path = unsafe { session.path.unwrap_unchecked() };
                    if let Err(e) = remove_file(path).await {
                        if e.kind() != ErrorKind::NotFound {
                            log!(stop, 0, "{}", e);
                            return Err(());
                        }
                    }
                }
            } else {
                let data = match bincode::serialize(&session) {
                    Ok(data) => data,
                    Err(_e) => {
                        log!(stop, 0, "{}", _e);
                        return Err(());
                    }
                };
                let path = match session.path {
                    Some(path) => path,
                    None => {
                        let key = fnv1a_64(session.session.as_bytes());
                        self.generate_path(key)
                    }
                };

                if let Err(e) = write(path, data).await {
                    if e.kind() != ErrorKind::NotFound {
                        log!(stop, 0, "{}", e);
                        return Err(());
                    }
                }
            }
            #[cfg(feature = "session-db")]
            {
                let data = match bincode::serialize(&session) {
                    Ok(data) => data,
                    Err(_e) => {
                        log!(stop, 0, "{}", _e);
                        return Err(());
                    }
                };
                let key = fnv1a_64(session.session.as_bytes());
                let user_id = 0_i64;
                let lang_id = 0_i64;
                if !session.new {
                    self.db.execute_prepare(m_fnv1a_64!("lib_set_session"), &[&user_id, &lang_id, &data, &key]).await;
                } else {
                    self.db.execute_prepare(m_fnv1a_64!("lib_add_session"), &[&session.session, &key, &user_id, &lang_id, &data]).await;
                }
            }
        }
        Ok(())
    }
}

/// User session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    /// Cookie key (session value)
    #[serde(skip)]
    pub(crate) session: String,
    /// Some user data
    data: HashMap<i64, Data>,
    /// Flash messages
    flash: HashMap<Flash, Vec<String>>,
    #[cfg(any(feature = "lang-static", feature = "lang-reload"))]
    lang_id: Option<usize>,
    #[cfg(feature = "access-db")]
    pub(crate) role_id: Option<usize>,
    #[cfg(feature = "access-db")]
    pub(crate) user_id: Option<usize>,

    /// User data is changed
    #[serde(skip)]
    change: bool,
    #[cfg(feature = "session-file")]
    #[serde(skip)]
    path: Option<PathBuf>,
    #[cfg(any(feature = "session-file", feature = "session-db"))]
    #[serde(skip)]
    new: bool,
}

impl Session {
    /// Set session data
    pub fn set<T>(&mut self, key: impl StrOrI64, value: T)
    where
        T: Into<Data>,
    {
        self.change = true;
        self.data.insert(key.to_i64(), value.into());
    }

    #[cfg(any(feature = "lang-static", feature = "lang-reload"))]
    pub fn set_lang_id(&mut self, lang_id: usize) {
        let change = match self.lang_id {
            Some(id) => id != lang_id,
            None => true,
        };
        if change {
            self.change = true;
            self.lang_id = Some(lang_id);
        }
    }

    /// Get session data for reading
    pub fn get(&self, key: impl StrOrI64) -> Option<&Data> {
        self.data.get(&key.to_i64())
    }

    #[cfg(any(feature = "lang-static", feature = "lang-reload"))]
    pub fn get_lang_id(&self) -> Option<usize> {
        self.lang_id
    }

    /// Getting session data by deleting it
    pub fn take(&mut self, key: impl StrOrI64) -> Option<Data> {
        self.change = true;
        self.data.remove(&key.to_i64())
    }

    #[cfg(any(feature = "lang-static", feature = "lang-reload"))]
    pub fn take_lang_id(&mut self) -> Option<usize> {
        self.lang_id?;
        self.change = true;
        self.lang_id.take()
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
        match self.flash.entry(kind) {
            Entry::Vacant(entry) => {
                entry.insert(vec![value]);
            }
            Entry::Occupied(mut entry) => {
                let vec = entry.get_mut();
                vec.push(value);
            }
        }
    }

    /// Take flash message from session data
    pub(crate) fn take_flash(&mut self) -> Option<HashMap<Flash, Vec<String>>> {
        if self.flash.is_empty() {
            None
        } else {
            self.change = true;
            Some(take(&mut self.flash))
        }
    }
}
