use chrono::{serde::ts_seconds::serialize as to_ts, Utc};
use std::{
    collections::{BTreeMap, HashMap},
    future::Future,
    path::PathBuf,
    pin::Pin,
    sync::Arc,
};
use tiny_web_macro::fnv1a_64;

use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha3::{Digest, Sha3_512};
use tokio::{fs::remove_file, sync::Mutex};

use crate::{fnv1a_64, StrOrI64};

use super::{
    cache::CacheSys,
    db::DB,
    html::{Html, Nodes},
    lang::Lang,
    log::Log,
    mail::{Mail, MailMessage, MailProvider},
    worker::WorkerData,
};

/// Type of one controler. Use in engine.
pub type Act = fn(&mut Action) -> Pin<Box<dyn Future<Output = Answer> + Send + '_>>;

/// List all controllers. Use in engine.
///
/// # Index
///
/// * 1 - Module ID
/// * 2 - Class ID
/// * 3 - Action ID
pub type ActMap = BTreeMap<i64, BTreeMap<i64, BTreeMap<i64, Act>>>;

/// Data transferred between controllers, template, markers, database and cache
///
/// # Values
///
/// * `None` - No data transferred.
/// * `Usize(usize)` - No data transferred.
/// * `I16(i16)` - No data transferred.
/// * `I32(i32)` - No data transferred.
/// * `I64(i64)` - i64 data.
/// * `F32(f32)` - f32 data.
/// * `F64(f64)` - f64 data.
/// * `Bool(bool)` - bool data.
/// * `String(String)` - String data.
/// * `Date(DateTime<Utc>)` - Chrono dateTime.
/// * `Json(Value)` - Serde json.
/// * `Vec(Vec<Data>)` - List of `Data`.
/// * `Map(BTreeMap<i64, Data>)` - Map of `Data`.
/// * `Route(Route)` - Route data.
/// * `Redirect(Redirect)` - Redirect data.
/// * `MailProvider(MailProvider)` - Mail provider data.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Data {
    /// No data transferred.
    None,
    /// usize data.
    Usize(usize),
    /// i16 data.
    I16(i16),
    /// i32 data.
    I32(i32),
    /// i64 data.
    I64(i64),
    /// f32 data.
    F32(f32),
    /// f64 data.
    F64(f64),
    /// bool data.
    Bool(bool),
    /// String data.
    String(String),
    /// DateTime.
    #[serde(serialize_with = "to_ts")]
    Date(DateTime<Utc>),
    /// Json
    Json(Value),
    /// List of `Data`.
    Vec(Vec<Data>),
    /// Raw data,
    Raw(Vec<u8>),
    /// Map of `Data`.
    Map(BTreeMap<i64, Data>),
    /// Route data.
    #[serde(skip_serializing, skip_deserializing)]
    Route(Route),
    /// Redirect data.
    #[serde(skip_serializing, skip_deserializing)]
    Redirect(Redirect),
    /// Mail provider data
    #[serde(skip_serializing, skip_deserializing)]
    MailProvider(MailProvider),
}

/// Type of Answer
///
/// # Values
///
/// * `None` - Without answer.
/// * `String(String)` - Answer in the form of text.
/// * `Raw(Vec<u8>)` - Answer in binary data.
#[derive(Debug)]
pub enum Answer {
    /// Without answer
    None,
    /// Answer in the form of text.
    String(String),
    /// Answer in binary data
    Raw(Vec<u8>),
}

/// Redirect struct
///
/// # Values
///
/// * `url: String` - Url.
/// * `permanently: bool,` - Permanently redirect.
#[derive(Debug, Clone)]
pub struct Redirect {
    /// Url
    pub url: String,
    /// Permanently redirect
    pub permanently: bool,
}

/// Describes received files
///
/// # Values
///
/// * `size: usize` - File size.
/// * `name: String` - File name.
/// * `tmp: std::path::PathBuf` - Absolute path to file location.
#[derive(Debug)]
pub struct WebFile {
    /// File size
    pub size: usize,
    /// File name
    pub name: String,
    /// Absolute path to file location
    pub tmp: std::path::PathBuf,
}

/// Input http protocol datas
///
/// # Values
///
/// * `get: HashMap<String, String>` - GET data.
/// * `post: HashMap<String, String>` - POST data.
/// * `file: HashMap<String, Vec<WebFile>>` - FILE data.
/// * `cookie: HashMap<String, String>` - Cookies.
/// * `params: HashMap<String, String>` - Params from web servers.
#[derive(Debug)]
pub struct Input {
    /// GET data
    pub get: HashMap<String, String>,
    /// POST data
    pub post: HashMap<String, String>,
    /// FILE data
    pub file: HashMap<String, Vec<WebFile>>,
    /// Cookies
    pub cookie: HashMap<String, String>,
    /// Params from web servers
    pub params: HashMap<String, String>,
}

/// Request parameters
///
///  # Values
///
/// * `ajax: bool` - Ajax query (only software detect).
/// * `host: String` - Request host. Example: subdomain.domain.zone.
/// * `scheme: String` - Request scheme. Example: http / https.
/// * `agent: String` - HTTP_USER_AGENT.
/// * `referer: String` - HTTP_REFERER.
/// * `ip: String` - Client IP.
/// * `method: String` - REQUEST_METHOD.
/// * `path: String` - DOCUMENT_ROOT.
/// * `url: String` - Request url. Example: /product/view/item/145
/// * `input: Input` - Input http protocol datas.
#[derive(Debug)]
pub struct Request {
    /// Ajax query (only software detect)
    pub ajax: bool,
    /// Request host. Example: subdomain.domain.zone
    pub host: String,
    /// Request scheme. Example: http / https
    pub scheme: String,
    /// HTTP_USER_AGENT
    pub agent: String,
    /// HTTP_REFERER
    pub referer: String,
    /// Client IP
    pub ip: String,
    /// REQUEST_METHOD
    pub method: String,
    /// DOCUMENT_ROOT
    pub path: String,
    /// Request url. Example: /product/view/item/145
    pub url: String,
    /// Input http protocol datas
    pub input: Input,
}

/// Response parameters
///
///  # Values
///
/// * `redirect: Option<Redirect>` - Redirect.
/// * `content_type: Option<String>` - Content type.
/// * `headers: Vec<String>` - Additional headers.
/// * `http_code: Option<u16>` - Http code.
/// * `css: Vec<String>` - Addition css script.
/// * `js: Vec<String>` - Addition js script.
#[derive(Debug)]
pub struct Response {
    /// Redirect
    pub redirect: Option<Redirect>,
    /// Content type
    pub content_type: Option<String>,
    /// Additional headers
    pub headers: Vec<(String, String)>,
    /// Http code
    pub http_code: Option<u16>,
    /// Addition css script
    pub css: Vec<String>,
    /// Addition js script
    pub js: Vec<String>,
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
    pub lang_id: i64,
    /// user_id from database
    pub user_id: i64,
    /// role_id from database
    role_id: i64,
    /// Cookie key
    pub key: String,
    /// User data from database
    data: HashMap<String, Data>,
    /// User data is changed
    change: bool,
}

/// Data to run Action (Main controler)
///
///  # Values
///
/// * `data: WorkerData` - Worker data.
/// * `request: Request` - Request from web server.
/// * `session: Option<String>` - Session key.
pub struct ActionData {
    /// Worker data
    pub data: WorkerData,
    /// Request from web server.
    pub request: Request,
    /// Session key.
    pub session: Option<String>,
}

/// Route of request
///
///  # Values
///
/// * `module: String` - Start module.
/// * `class: String` - Start class.
/// * `action: String` - Start action (controller).
/// * `module_id: i64` - Module id.
/// * `class_id: i64` - Class id.
/// * `action_id: i64` - Action id.
/// * `param: Option<String>` - Controller param.
/// * `lang_id: Option<i64>` - Set lang id.
#[derive(Debug, Clone)]
pub struct Route {
    /// Start module
    module: String,
    /// Start class
    class: String,
    /// Start action (controller)
    action: String,
    /// Module id
    module_id: i64,
    /// Class id
    class_id: i64,
    /// Action id
    action_id: i64,
    /// Controller param
    param: Option<String>,
    /// Set lang id
    lang_id: Option<i64>,
}

/// Cache struct
#[derive(Debug)]
pub struct Cache {
    cache: Arc<Mutex<CacheSys>>,
}

/// Main struct to run web engine
///
///  # Values
///
/// * `request: Request` - Request from web server.
/// * `response: Response` - Response to web server.
/// * `session: Session` - Session data.
/// * `salt: Arc<String>` - Secret salt.
/// * `data: BTreeMap<i64, Data>` - Data transferred between controllers template markers and cache.
/// * `pub module: String` - Start module name.
/// * `pub class: String` - Start class name.
/// * `pub action: String` - Start action (controller) name.
/// * `param: Option<String>` - Start param.
/// * `module_id: i64` - Module ID.
/// * `class_id: i64,` - Class ID.
/// * `action_id: i64` - Action ID.
/// * `current_module_id: i64` - Current module ID.
/// * `current_class_id: i64` - Current class ID.
/// * `html: Option<Arc<BTreeMap<i64, Vec<Node>>>>` - Current templates.
/// * `lang: Option<Arc<BTreeMap<i64, String>>>` - Current translates.
/// * `engine: Arc<ActMap>` - Engine of server.
/// * `language: Arc<Lang>` - All translates.
/// * `template: Arc<Html>` - All templates.
/// * `cache: Arc<Mutex<Cache>>` - Cache.
/// * `db: Arc<DB>` - Database pool.
/// * `mail: Arc<Mail>` - Mail function.
/// * `internal: bool` - Internal call of controller.
#[derive(Debug)]
pub struct Action {
    /// Request from web server
    pub request: Request,
    /// Response to web server
    pub response: Response,
    /// Session data
    pub session: Session,
    /// Secret salt
    pub salt: Arc<String>,

    /// Data transferred between controllers template markers and cache
    data: BTreeMap<i64, Data>,
    /// Start module name
    pub module: String,
    /// Start class name
    pub class: String,
    /// Start action (controller) name
    pub action: String,
    /// Start param
    pub param: Option<String>,
    /// Module ID
    module_id: i64,
    /// Class ID
    class_id: i64,
    /// Action ID
    action_id: i64,
    /// Current module ID
    current_module_id: i64,
    /// Current class ID
    current_class_id: i64,
    /// Current templates
    html: Option<Arc<BTreeMap<i64, Nodes>>>,
    /// Current translates
    lang: Option<Arc<BTreeMap<i64, String>>>,

    /// Engine of server
    engine: Arc<ActMap>,
    /// All translates
    language: Arc<Lang>,
    /// All templates
    template: Arc<Html>,
    /// Cache
    pub cache: Cache,
    /// Database pool
    pub db: Arc<DB>,
    /// Mail function
    mail: Arc<Mutex<Mail>>,

    /// Internal call of controller
    pub internal: bool,
}

impl Action {
    /// Run new Action
    ///
    /// # Return
    ///
    /// `Ok(Action)` - Action (controller) was found success.
    /// `Err(Redirect, HashMap<String, Vec<WebFile>>)` - Must redirect, and then remove temp files.
    pub async fn new(data: ActionData) -> Result<Action, (Redirect, HashMap<String, Vec<WebFile>>)> {
        let response = Response {
            redirect: None,
            content_type: None,
            headers: Vec::new(),
            http_code: None,
            css: Vec::new(),
            js: Vec::new(),
        };
        let lang_id = data.data.lang.default as i64;
        let mut session = if let Some(session) = data.session {
            Session::load_session(session.clone(), Arc::clone(&data.data.db), lang_id).await
        } else {
            Session::new(lang_id, &data.data.salt, &data.request.ip, &data.request.agent, &data.request.host)
        };
        // Module, class and action (controller) from URL
        let route = match Action::extract_route(&data.request, Arc::clone(&data.data.cache), Arc::clone(&data.data.db)).await {
            Ok(r) => r,
            Err(redirect) => return Err((redirect, data.request.input.file)),
        };
        let module = route.module;
        let class = route.class;
        let action = route.action;
        let module_id = route.module_id;
        let class_id = route.class_id;
        let action_id = route.action_id;
        if let Some(lang_id) = route.lang_id {
            if session.lang_id != lang_id {
                session.change = true;
                session.lang_id = lang_id;
            }
        }
        let param = route.param;
        // Load new template list
        let html = data.data.html.list.get(&module_id).and_then(|module| module.get(&class_id).map(Arc::clone));
        // Load new translate list
        let lang = data
            .data
            .lang
            .list
            .get(&session.lang_id)
            .and_then(|langs| langs.get(&module_id))
            .and_then(|module| module.get(&class_id).map(Arc::clone));

        Ok(Action {
            request: data.request,
            response,
            session,
            salt: Arc::clone(&data.data.salt),
            data: BTreeMap::new(),

            module,
            class,
            action,
            param,
            module_id,
            class_id,
            action_id,
            current_module_id: module_id,
            current_class_id: class_id,
            html,
            lang,

            engine: data.data.engine,
            language: data.data.lang,
            template: data.data.html,
            cache: Cache::new(data.data.cache),
            db: data.data.db,
            mail: data.data.mail,
            internal: false,
        })
    }

    /// Run execute of controller
    pub async fn run(action: &mut Action) -> Answer {
        action.start_route(action.module_id, action.class_id, action.action_id, action.param.clone(), false).await
    }

    /// Load internal controller
    pub async fn load<'a>(
        &mut self,
        key: impl StrOrI64,
        module: impl StrOrI64,
        class: impl StrOrI64,
        action: impl StrOrI64,
        param: Option<String>,
    ) {
        let res = self.start_route(module.to_i64(), class.to_i64(), action.to_i64(), param, true).await;
        if let Answer::String(value) = res {
            self.data.insert(key.to_i64(), Data::String(value));
        }
    }

    /// Start internal route
    async fn start_route(
        &mut self,
        module_id: i64,
        class_id: i64,
        action_id: i64,
        param: Option<String>,
        internal: bool,
    ) -> Answer {
        // Check permission
        if self.get_access(module_id, class_id, action_id).await {
            return self.invoke(module_id, class_id, action_id, param, internal).await;
        }
        if internal {
            return Answer::None;
        }
        // If not /index/index/not_found - then redirect
        if !(module_id == fnv1a_64!("index") && class_id == fnv1a_64!("index") && action_id == fnv1a_64!("not_found")) {
            self.response.redirect = Some(Redirect {
                url: self.not_found().await,
                permanently: false,
            });
        }
        Answer::None
    }

    /// Send email
    pub async fn mail(&self, message: MailMessage) -> bool {
        let provider = {
            let mail = self.mail.lock().await;
            mail.provider.clone()
        };
        Mail::send(provider, Arc::clone(&self.db), message, self.session.user_id, self.request.host.clone()).await
    }

    /// Get translate
    pub fn lang(&self, text: impl StrOrI64) -> String {
        if let Some(l) = &self.lang {
            if let Some(str) = l.get(&text.to_i64()) {
                return str.to_owned();
            }
        }
        text.to_str()
    }

    /// Invoke found controller
    async fn invoke(&mut self, module_id: i64, class_id: i64, action_id: i64, param: Option<String>, internal: bool) -> Answer {
        if let Some(m) = self.engine.get(&module_id) {
            if let Some(c) = m.get(&class_id) {
                if let Some(a) = c.get(&action_id) {
                    if self.current_module_id == module_id && self.current_class_id == class_id {
                        // Call from the same module as the current one
                        let i = self.internal;
                        let p = match param {
                            Some(str) => self.param.replace(str),
                            None => self.param.take(),
                        };
                        self.internal = internal;
                        let res = a(self).await;
                        self.internal = i;
                        self.param = p;
                        return res;
                    } else {
                        // Call from the different module as the current one

                        // Load new template list
                        let h = match self.template.list.get(&module_id) {
                            Some(h) => match h.get(&class_id) {
                                Some(h) => self.html.replace(Arc::clone(h)),
                                None => self.html.take(),
                            },
                            None => self.html.take(),
                        };
                        // Load new translate list
                        let l = match self.language.list.get(&self.session.lang_id) {
                            Some(l) => match l.get(&module_id) {
                                Some(l) => match l.get(&class_id) {
                                    Some(l) => self.lang.replace(Arc::clone(l)),
                                    None => self.lang.take(),
                                },
                                None => self.lang.take(),
                            },
                            None => self.lang.take(),
                        };
                        let i = self.internal;
                        let p = match param {
                            Some(str) => self.param.replace(str),
                            None => self.param.take(),
                        };
                        let m = self.current_module_id;
                        self.current_module_id = module_id;
                        let c = self.current_class_id;
                        self.current_class_id = class_id;

                        self.internal = internal;

                        // Call controlle
                        let res = a(self).await;

                        self.current_module_id = m;
                        self.current_class_id = c;
                        self.html = h;
                        self.lang = l;
                        self.internal = i;
                        self.param = p;
                        return res;
                    }
                }
            }
        }
        Answer::None
    }

    /// Get access to run controller
    pub async fn get_access(&mut self, module: impl StrOrI64, class: impl StrOrI64, action: impl StrOrI64) -> bool {
        let module_id = module.to_i64();
        let class_id = class.to_i64();
        let action_id = action.to_i64();
        // Read from cache
        let key = vec![fnv1a_64!("auth"), self.session.role_id, module_id, class_id, action_id];
        let key = key.as_slice();
        if let Some(Data::Bool(a)) = self.cache.get(key).await {
            return a;
        };
        // Prepare sql query
        match self
            .db
            .query_raw(
                fnv1a_64!("lib_get_auth"),
                &[&self.session.role_id, &module_id, &module_id, &module_id, &class_id, &class_id, &action_id],
            )
            .await
        {
            Some(rows) => {
                if rows.len() == 1 {
                    let access: bool = rows[0].get(0);
                    self.cache.set(key, Data::Bool(access)).await;
                    access
                } else {
                    self.cache.set(key, Data::Bool(false)).await;
                    false
                }
            }
            None => false,
        }
    }

    /// Get not_found url
    pub async fn not_found(&mut self) -> String {
        let key = vec![fnv1a_64!("404"), self.session.lang_id];
        let key = key.as_slice();
        match self.cache.get(key).await {
            Some(d) => match d {
                Data::String(url) => url,
                _ => "/index/index/not_found".to_owned(),
            },
            None => {
                // Load from database
                match self.db.query_raw(fnv1a_64!("lib_get_not_found"), &[&self.session.lang_id]).await {
                    Some(v) => {
                        if v.is_empty() {
                            self.cache.set(key, Data::None).await;
                            "/index/index/not_found".to_owned()
                        } else {
                            let row = unsafe { v.get_unchecked(0) };
                            let url: String = row.get(0);
                            self.cache.set(key, Data::String(url.clone())).await;
                            url
                        }
                    }
                    None => "/index/index/not_found".to_owned(),
                }
            }
        }
    }

    /// Extract route from url
    async fn extract_route(request: &Request, cache: Arc<Mutex<CacheSys>>, db: Arc<DB>) -> Result<Route, Redirect> {
        // Get redirect
        let key = vec![fnv1a_64!("redirect"), fnv1a_64(request.url.as_bytes())];
        match CacheSys::get(Arc::clone(&cache), &key).await {
            Some(d) => match d {
                Data::None => {}
                Data::Redirect(r) => return Err(r),
                _ => {
                    Log::warning(3000, Some(format!("{:?}", d)));
                }
            },
            None => {
                // Load from database
                match db.query_raw(fnv1a_64!("lib_get_redirect"), &[&request.url]).await {
                    Some(v) => {
                        if v.is_empty() {
                            CacheSys::set(Arc::clone(&cache), &key, Data::None).await;
                        } else {
                            let row = unsafe { v.get_unchecked(0) };
                            let r = Redirect {
                                url: row.get(0),
                                permanently: row.get(1),
                            };
                            CacheSys::set(Arc::clone(&cache), &key, Data::Redirect(r.clone())).await;
                            return Err(r);
                        }
                    }
                    None => {
                        let r = Route {
                            module: "index".to_owned(),
                            class: "index".to_owned(),
                            action: "err".to_owned(),
                            module_id: fnv1a_64!("index"),
                            class_id: fnv1a_64!("index"),
                            action_id: fnv1a_64!("err"),
                            param: None,
                            lang_id: None,
                        };
                        return Ok(r);
                    }
                }
            }
        }

        // Get route
        let key = vec![fnv1a_64!("route"), fnv1a_64(request.url.as_bytes())];
        match CacheSys::get(Arc::clone(&cache), &key[..]).await {
            Some(d) => match d {
                Data::None => {}
                Data::Route(r) => return Ok(r),
                _ => {
                    Log::warning(3001, Some(format!("{:?}", d)));
                }
            },
            None => {
                // Load from database
                match db.query_raw(fnv1a_64!("lib_get_route"), &[&request.url]).await {
                    Some(v) => {
                        if v.is_empty() {
                            CacheSys::set(Arc::clone(&cache), &key, Data::None).await;
                        } else {
                            let row = unsafe { v.get_unchecked(0) };
                            let r = Route {
                                module: row.get(0),
                                class: row.get(1),
                                action: row.get(2),
                                module_id: row.get(3),
                                class_id: row.get(4),
                                action_id: row.get(5),
                                param: row.get(6),
                                lang_id: row.get(7),
                            };
                            CacheSys::set(Arc::clone(&cache), &key, Data::Route(r.clone())).await;
                            return Ok(r);
                        }
                    }
                    None => {
                        let r = Route {
                            module: "index".to_owned(),
                            class: "index".to_owned(),
                            action: "err".to_owned(),
                            module_id: fnv1a_64!("index"),
                            class_id: fnv1a_64!("index"),
                            action_id: fnv1a_64!("err"),
                            param: None,
                            lang_id: None,
                        };
                        return Ok(r);
                    }
                }
            }
        }

        if request.url != "/" {
            let mut load: Vec<&str> = request.url.splitn(5, '/').collect();
            load.retain(|&x| !x.is_empty());
            let r = match load.len() {
                1 => {
                    let module = unsafe { *load.get_unchecked(0) };
                    Route {
                        module: module.to_owned(),
                        class: "index".to_owned(),
                        action: "index".to_owned(),
                        module_id: fnv1a_64(module.as_bytes()),
                        class_id: fnv1a_64!("index"),
                        action_id: fnv1a_64!("index"),
                        param: None,
                        lang_id: None,
                    }
                }
                2 => {
                    let module = unsafe { *load.get_unchecked(0) };
                    let class = unsafe { *load.get_unchecked(1) };
                    Route {
                        module: module.to_owned(),
                        class: class.to_owned(),
                        action: "index".to_owned(),
                        module_id: fnv1a_64(module.as_bytes()),
                        class_id: fnv1a_64(class.as_bytes()),
                        action_id: fnv1a_64!("index"),
                        param: None,
                        lang_id: None,
                    }
                }
                3 => {
                    let module = unsafe { *load.get_unchecked(0) };
                    let class = unsafe { *load.get_unchecked(1) };
                    let action = unsafe { *load.get_unchecked(2) };
                    Route {
                        module: module.to_owned(),
                        class: class.to_owned(),
                        action: action.to_owned(),
                        module_id: fnv1a_64(module.as_bytes()),
                        class_id: fnv1a_64(class.as_bytes()),
                        action_id: fnv1a_64(action.as_bytes()),
                        param: None,
                        lang_id: None,
                    }
                }
                4 => {
                    let module = unsafe { *load.get_unchecked(0) };
                    let class = unsafe { *load.get_unchecked(1) };
                    let action = unsafe { *load.get_unchecked(2) };
                    let param = unsafe { *load.get_unchecked(3) };
                    Route {
                        module: module.to_owned(),
                        class: class.to_owned(),
                        action: action.to_owned(),
                        module_id: fnv1a_64(module.as_bytes()),
                        class_id: fnv1a_64(class.as_bytes()),
                        action_id: fnv1a_64(action.as_bytes()),
                        param: Some(param.to_owned()),
                        lang_id: None,
                    }
                }
                _ => Route {
                    module: "index".to_owned(),
                    class: "index".to_owned(),
                    action: "index".to_owned(),
                    module_id: fnv1a_64!("index"),
                    class_id: fnv1a_64!("index"),
                    action_id: fnv1a_64!("index"),
                    param: None,
                    lang_id: None,
                },
            };
            Ok(r)
        } else {
            Ok(Route {
                module: "index".to_owned(),
                class: "index".to_owned(),
                action: "index".to_owned(),
                module_id: fnv1a_64!("index"),
                class_id: fnv1a_64!("index"),
                action_id: fnv1a_64!("index"),
                param: None,
                lang_id: None,
            })
        }
    }

    /// Stop controller
    pub async fn stop(action: Action) {
        // Save session
        let handle = Session::save_session(action.db, &action.session, &action.request);
        // Remove temp file
        for val in action.request.input.file.values() {
            for f in val {
                if let Err(e) = remove_file(&f.tmp).await {
                    Log::warning(1103, Some(format!("filename={}. Error={}", &f.tmp.display(), e)));
                };
            }
        }
        handle.await;
    }

    /// Simple remove temp file after redirect
    pub async fn clean_file(file: Vec<PathBuf>) {
        for f in file {
            if let Err(e) = remove_file(&f).await {
                Log::warning(1103, Some(format!("filename={}. Error={}", f.display(), e)));
            };
        }
    }

    /// Set value for the template
    pub fn set(&mut self, key: impl StrOrI64, value: Data) {
        self.data.insert(key.to_i64(), value);
    }

    /// Render template
    ///
    /// # Value
    ///
    /// * `template: &str` - Name of template
    pub fn render(&self, template: impl StrOrI64) -> Answer {
        match &self.html {
            Some(h) => match h.get(&template.to_i64()) {
                Some(vec) => Html::render(&self.data, vec),
                None => Answer::None,
            },
            None => Answer::None,
        }
    }

    /// Get route
    pub async fn route(&mut self, module: &str, class: &str, action: &str, param: Option<&str>, lang_id: i64) -> String {
        // Read from cache
        let key = match param {
            Some(p) => vec![
                fnv1a_64!("route"),
                fnv1a_64(module.as_bytes()),
                fnv1a_64(class.as_bytes()),
                fnv1a_64(action.as_bytes()),
                fnv1a_64(p.as_bytes()),
                lang_id,
            ],
            None => vec![
                fnv1a_64!("route"),
                fnv1a_64(module.as_bytes()),
                fnv1a_64(class.as_bytes()),
                fnv1a_64(action.as_bytes()),
                0,
                lang_id,
            ],
        };
        let key = key.as_slice();
        if let Some(Data::String(s)) = self.cache.get(key).await {
            return s;
        };
        // Prepare sql query
        match self
            .db
            .query_raw(
                fnv1a_64!("lib_get_url"),
                &[&fnv1a_64(module.as_bytes()), &fnv1a_64(class.as_bytes()), &fnv1a_64(action.as_bytes()), &param, &lang_id],
            )
            .await
        {
            Some(rows) => {
                if rows.len() == 1 {
                    let url: String = rows[0].get(0);
                    self.cache.set(key, Data::String(url.clone())).await;
                    url
                } else {
                    let url = match param {
                        Some(s) => {
                            format!("/{}/{}/{}/{}", module, class, action, s)
                        }
                        None => format!("/{}/{}/{}", module, class, action),
                    };
                    self.cache.set(key, Data::String(url.clone())).await;
                    url
                }
            }
            None => match param {
                Some(s) => {
                    format!("/{}/{}/{}/{}", module, class, action, s)
                }
                None => format!("/{}/{}/{}", module, class, action),
            },
        }
    }
}

impl Session {
    /// Create new session
    fn new(lang_id: i64, salt: &str, ip: &str, agent: &str, host: &str) -> Session {
        Session {
            id: 0,
            lang_id,
            user_id: 0,
            role_id: 0,
            key: Session::generate_session(salt, ip, agent, host),
            data: HashMap::new(),
            change: false,
        }
    }

    /// Create new session by cookie (session) key
    fn with_key(lang_id: i64, key: String) -> Session {
        Session {
            id: 0,
            lang_id,
            user_id: 0,
            role_id: 0,
            key,
            data: HashMap::new(),
            change: false,
        }
    }

    /// Load session from database
    async fn load_session(key: String, db: Arc<DB>, lang_id: i64) -> Session {
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
            HashMap::new()
        } else {
            match bincode::deserialize::<HashMap<String, Data>>(data) {
                Ok(r) => r,
                Err(_) => HashMap::new(),
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
    async fn save_session(db: Arc<DB>, session: &Session, request: &Request) {
        if session.change {
            let data = match bincode::serialize(&session.data) {
                Ok(r) => r,
                Err(_) => Vec::new(),
            };
            if session.id == 0 {
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
}

impl Cache {
    /// Create new Cache instanse
    pub fn new(cache: Arc<Mutex<CacheSys>>) -> Cache {
        Cache { cache }
    }

    /// Get cache
    pub async fn get(&mut self, keys: &[i64]) -> Option<Data> {
        CacheSys::get(Arc::clone(&self.cache), keys).await
    }

    /// Set cache
    pub async fn set(&mut self, keys: &[i64], data: Data) {
        CacheSys::set(Arc::clone(&self.cache), keys, data).await
    }

    /// Removes a key from the Cache.
    ///
    /// If `key` ends with a `:` character, all data beginning with that `key` is deleted.
    pub async fn del(&mut self, keys: &[i64]) {
        CacheSys::del(Arc::clone(&self.cache), keys).await
    }

    /// Clear all cache
    pub async fn clear(&mut self) {
        CacheSys::clear(Arc::clone(&self.cache)).await
    }

    pub async fn show(&mut self) {
        CacheSys::show(Arc::clone(&self.cache)).await
    }
}
