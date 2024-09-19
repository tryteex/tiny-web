use std::{
    collections::{BTreeMap, HashMap},
    fs::File,
    future::Future,
    io::{Error, Write},
    path::PathBuf,
    pin::Pin,
    sync::Arc,
};
use tiny_web_macro::fnv1a_64;

use tokio::{
    fs::remove_file,
    sync::{mpsc::Sender, Mutex},
    task::{yield_now, JoinHandle},
};

#[cfg(debug_assertions)]
use tokio::sync::RwLock;

use crate::{fnv1a_64, StrOrI64};

use super::{
    app::App,
    cache::{Cache, CacheSys},
    data::Data,
    dbs::adapter::DB,
    html::{Html, Nodes},
    init::Addr,
    lang::{Lang, LangItem},
    log::Log,
    mail::{Mail, MailMessage},
    request::{Request, WebFile},
    response::{Redirect, Response},
    route::Route,
    session::{Flash, Session},
    worker::{MessageWrite, Worker},
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

/// Data to run Action (Main controler)
#[derive(Debug)]
pub(crate) struct ActionData {
    /// Engine - binary tree of controller functions.
    pub engine: Arc<ActMap>,
    /// I18n system.
    #[cfg(not(debug_assertions))]
    pub lang: Arc<Lang>,
    /// I18n system.
    #[cfg(debug_assertions)]
    pub lang: Arc<RwLock<Lang>>,
    /// Template maker.
    #[cfg(not(debug_assertions))]
    pub html: Arc<Html>,
    /// Template maker.
    #[cfg(debug_assertions)]
    pub html: Arc<RwLock<Html>>,
    /// Cache system.
    pub cache: Arc<Mutex<CacheSys>>,
    /// Database connections pool.
    pub db: Arc<DB>,
    /// Session key.
    pub session_key: Arc<String>,
    /// Salt for a crypto functions.
    pub salt: Arc<String>,
    /// Mail provider.
    pub mail: Arc<Mutex<Mail>>,
    /// Request from web server.
    pub request: Request,
    /// Session value.
    pub session: Option<String>,
    /// Sender data to output stream
    pub(crate) tx: Arc<Sender<MessageWrite>>,
    /// Default controller for request "/" or default class or default action
    pub(crate) action_index: Arc<Route>,
    /// Default controller for 404 Not Found
    pub(crate) action_not_found: Arc<Route>,
    /// Default controller for error_route
    pub(crate) action_err: Arc<Route>,
    /// Stop signal
    pub(crate) stop: Option<(Arc<Addr>, i64, Arc<String>)>,
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
    #[cfg(not(debug_assertions))]
    language: Arc<Lang>,
    /// All translates
    #[cfg(debug_assertions)]
    language: Arc<RwLock<Lang>>,
    /// All templates
    #[cfg(not(debug_assertions))]
    template: Arc<Html>,
    /// All templates
    #[cfg(debug_assertions)]
    template: Arc<RwLock<Html>>,
    /// Cache
    pub cache: Cache,
    /// Database pool
    pub db: Arc<DB>,
    /// Mail function
    mail: Arc<Mutex<Mail>>,

    /// Internal call of controller
    pub internal: bool,

    /// Sender data to output stream
    pub(crate) tx: Arc<Sender<MessageWrite>>,

    /// Header was sended
    pub(crate) header_send: bool,

    /// Default controller for 404 Not Found
    not_found: Arc<Route>,

    /// Stop signal
    stop: Option<(Arc<Addr>, i64, Arc<String>)>,
}

impl Action {
    /// Run new Action
    ///
    /// # Return
    ///
    /// `Ok(Action)` - Action (controller) was found success.
    /// `Err(Redirect, HashMap<String, Vec<WebFile>>)` - Must redirect, and then remove temp files.
    pub(crate) async fn new(data: ActionData) -> Result<Action, (Redirect, HashMap<String, Vec<WebFile>>)> {
        let response = Response {
            redirect: None,
            content_type: None,
            headers: Vec::new(),
            http_code: None,
            css: Vec::new(),
            js: Vec::new(),
            meta: Vec::new(),
        };
        #[cfg(not(debug_assertions))]
        let lang_id = data.lang.default as i64;
        #[cfg(debug_assertions)]
        let lang_id = data.lang.read().await.default as i64;

        let mut session = if let Some(session) = data.session {
            Session::load_session(session.clone(), Arc::clone(&data.db), lang_id, data.session_key).await
        } else {
            Session::new(lang_id, &data.salt, &data.request.ip, &data.request.agent, &data.request.host, data.session_key)
        };
        // Module, class and action (controller) from URL
        let route = match Action::extract_route(
            &data.request,
            Arc::clone(&data.cache),
            Arc::clone(&data.db),
            Arc::clone(&data.action_index),
            Arc::clone(&data.action_err),
        )
        .await
        {
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
            session.set_lang_id(lang_id);
        }
        let param = route.param;
        // Load new template list
        #[cfg(not(debug_assertions))]
        let html = data.html.list.get(&module_id).and_then(|module| module.get(&class_id).cloned());
        #[cfg(debug_assertions)]
        let html = data.html.read().await.list.get(&module_id).and_then(|module| module.get(&class_id).cloned());
        // Load new translate list
        #[cfg(not(debug_assertions))]
        let lang = data
            .lang
            .list
            .get(&session.get_lang_id())
            .and_then(|langs| langs.get(&module_id))
            .and_then(|module| module.get(&class_id).cloned());
        #[cfg(debug_assertions)]
        let lang = data
            .lang
            .read()
            .await
            .list
            .get(&session.get_lang_id())
            .and_then(|langs| langs.get(&module_id))
            .and_then(|module| module.get(&class_id).cloned());

        Ok(Action {
            request: data.request,
            response,
            session,
            salt: data.salt,
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

            engine: data.engine,
            language: data.lang,
            template: data.html,
            cache: Cache::new(data.cache),
            db: data.db,
            mail: data.mail,

            internal: false,

            tx: data.tx,
            header_send: false,
            not_found: data.action_not_found,

            stop: data.stop,
        })
    }

    /// Load internal controller
    pub async fn load(
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

    /// Get translate
    pub fn lang(&self, text: impl StrOrI64) -> String {
        if let Some(l) = &self.lang {
            if let Some(str) = l.get(&text.to_i64()) {
                return str.to_owned();
            }
        }
        text.to_str().to_owned()
    }

    /// Get current lang
    pub async fn lang_current(&self) -> Arc<LangItem> {
        #[cfg(not(debug_assertions))]
        {
            Arc::clone(unsafe { self.language.langs.get_unchecked(self.session.get_lang_id() as usize) })
        }
        #[cfg(debug_assertions)]
        {
            Arc::clone(unsafe { self.language.read().await.langs.get_unchecked(self.session.get_lang_id() as usize) })
        }
    }

    /// Get vector of system languages
    pub async fn lang_list(&self) -> Arc<Vec<Arc<LangItem>>> {
        #[cfg(not(debug_assertions))]
        {
            Arc::clone(&self.language.langs)
        }
        #[cfg(debug_assertions)]
        {
            Arc::clone(&self.language.read().await.langs)
        }
    }

    /// Setting data into internal memory
    pub fn set<T>(&mut self, key: impl StrOrI64, value: T)
    where
        T: Into<Data>,
    {
        self.data.insert(key.to_i64(), value.into());
    }

    /// Getting references to data from internal memory
    pub fn get<T>(&self, key: impl StrOrI64) -> Option<&T>
    where
        for<'a> &'a T: From<&'a Data>,
    {
        self.data.get(&key.to_i64()).map(|value| value.into())
    }

    /// Taking (removing) data from internal memory
    pub fn take<T>(&mut self, key: impl StrOrI64) -> Option<T>
    where
        T: From<Data>,
    {
        self.data.remove(&key.to_i64()).map(|value| value.into())
    }

    /// Set flash message to session data
    pub fn set_flash(&mut self, kind: Flash, value: String) {
        self.session.set_flash(kind, value);
    }

    /// Take flash message from session data
    pub fn take_flash(&mut self) -> Option<Vec<(Flash, String)>> {
        self.session.take_flash()
    }

    /// Set value for the template from translate
    pub fn set_lang(&mut self, key: impl StrOrI64) {
        let idkey = key.to_i64();
        if let Some(l) = &self.lang {
            if let Some(str) = l.get(&idkey) {
                self.data.insert(idkey, Data::String(str.to_owned()));
                return;
            }
        }
        self.data.insert(idkey, Data::String(key.to_str().to_owned()));
    }

    /// Set an array of values for the template from the translation
    pub fn set_lang_arr(&mut self, keys: &[impl StrOrI64]) {
        for key in keys {
            let idkey = key.to_i64();
            if let Some(l) = &self.lang {
                if let Some(str) = l.get(&idkey) {
                    self.data.insert(idkey, Data::String(str.to_owned()));
                    continue;
                }
            }
            self.data.insert(idkey, Data::String(key.to_str().to_owned()));
        }
    }

    /// Spawns a new asynchronous task, returning a
    /// [`tokio::task::JoinHandle`](tokio::task::JoinHandle) for it.
    ///
    /// The provided future will start running in the background immediately
    /// when `spawn` is called, even if you don't await the returned
    /// `JoinHandle`.
    pub fn spawn<F>(&self, future: F) -> JoinHandle<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        tokio::spawn(future)
    }

    /// Write data to the output stream
    pub async fn write(&mut self, answer: Answer) {
        let vec = match answer {
            Answer::None => return,
            Answer::String(str) => str.as_bytes().to_vec(),
            Answer::Raw(raw) => raw,
        };
        Worker::write(self, vec).await;
        self.header_send = true;
        yield_now().await;
    }

    /// Get access to run controller
    pub async fn get_access(&mut self, module: impl StrOrI64, class: impl StrOrI64, action: impl StrOrI64) -> bool {
        if !self.db.in_use() {
            return true;
        }
        let module_id = module.to_i64();
        let class_id = class.to_i64();
        let action_id = action.to_i64();
        // Read from cache
        let key = vec![fnv1a_64!("auth"), self.session.role_id, module_id, class_id, action_id];
        let (data, key) = self.cache.get(key).await;
        if let Some(Data::Bool(a)) = data {
            return a;
        };
        // Prepare sql query
        match self
            .db
            .query_prepare(
                fnv1a_64!("lib_get_auth"),
                &[&self.session.role_id, &module_id, &module_id, &module_id, &class_id, &class_id, &action_id],
                false,
            )
            .await
        {
            Some(rows) => {
                if rows.len() == 1 {
                    let access = if let Data::Vec(row) = unsafe { rows.get_unchecked(0) } {
                        if row.is_empty() {
                            return false;
                        }
                        if let Data::I32(val) = unsafe { row.get_unchecked(0) } {
                            *val != 0
                        } else {
                            return false;
                        }
                    } else {
                        return false;
                    };

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

    /// Restart the web server after creating the configuration file
    pub fn install_end(&mut self, conf: String) -> Result<(), Error> {
        if !self.db.in_use() {
            if let Some((rpc, stop, path)) = self.stop.take() {
                let mut file = File::create(format!("{}/tiny.toml", path))?;
                file.write_all(conf.as_bytes())?;

                App::stop(rpc, stop);
            }
        }
        Ok(())
    }

    /// Render template
    ///
    /// # Value
    ///
    /// * `template: &str` - Name of template
    pub fn render(&mut self, template: impl StrOrI64) -> Answer {
        match &self.html {
            Some(h) => match h.get(&template.to_i64()) {
                Some(vec) => {
                    if !self.response.css.is_empty() {
                        let mut vec = Vec::with_capacity(self.response.css.len());
                        for css in self.response.css.drain(..) {
                            vec.push(Data::String(css));
                        }
                        self.data.insert(fnv1a_64!("css"), Data::Vec(vec));
                    }
                    if !self.response.js.is_empty() {
                        let mut vec = Vec::with_capacity(self.response.js.len());
                        for js in self.response.js.drain(..) {
                            vec.push(Data::String(js));
                        }
                        self.data.insert(fnv1a_64!("js"), Data::Vec(vec));
                    }
                    if !self.response.meta.is_empty() {
                        let mut vec = Vec::with_capacity(self.response.meta.len());
                        for meta in self.response.meta.drain(..) {
                            vec.push(Data::String(meta));
                        }
                        self.data.insert(fnv1a_64!("meta"), Data::Vec(vec));
                    }
                    Html::render(&self.data, vec)
                }
                None => Answer::None,
            },
            None => Answer::None,
        }
    }

    /// Get route
    pub async fn route(&mut self, module: &str, class: &str, action: &str, param: Option<&str>, lang_id: Option<i64>) -> String {
        if self.db.in_use() {
            // Read from cache
            let key = match (param, lang_id) {
                (Some(p), Some(l)) => vec![
                    fnv1a_64!("route"),
                    fnv1a_64(module.as_bytes()),
                    fnv1a_64(class.as_bytes()),
                    fnv1a_64(action.as_bytes()),
                    fnv1a_64(p.as_bytes()),
                    l,
                ],
                (Some(p), None) => vec![
                    fnv1a_64!("route"),
                    fnv1a_64(module.as_bytes()),
                    fnv1a_64(class.as_bytes()),
                    fnv1a_64(action.as_bytes()),
                    fnv1a_64(p.as_bytes()),
                    -1,
                ],
                (None, Some(l)) => {
                    vec![fnv1a_64!("route"), fnv1a_64(module.as_bytes()), fnv1a_64(class.as_bytes()), fnv1a_64(action.as_bytes()), 0, l]
                }
                (None, None) => {
                    vec![fnv1a_64!("route"), fnv1a_64(module.as_bytes()), fnv1a_64(class.as_bytes()), fnv1a_64(action.as_bytes()), 0, -1]
                }
            };
            let (data, key) = self.cache.get(key).await;
            if let Some(Data::String(s)) = data {
                return s;
            };
            // Prepare sql query
            match self
                .db
                .query_prepare(
                    fnv1a_64!("lib_get_url"),
                    &[&fnv1a_64(module.as_bytes()), &fnv1a_64(class.as_bytes()), &fnv1a_64(action.as_bytes()), &param, &lang_id],
                    false,
                )
                .await
            {
                Some(rows) => {
                    if rows.len() == 1 {
                        let row = if let Data::Vec(vec) = unsafe { rows.get_unchecked(0) } {
                            vec
                        } else {
                            return Action::format_route(module, class, action, param);
                        };
                        if row.is_empty() {
                            return Action::format_route(module, class, action, param);
                        }

                        let url = if let Data::String(url) = unsafe { row.get_unchecked(0) } {
                            url.clone()
                        } else {
                            return Action::format_route(module, class, action, param);
                        };
                        self.cache.set(key, Data::String(url.clone())).await;
                        url
                    } else {
                        let url = Action::format_route(module, class, action, param);
                        self.cache.set(key, Data::String(url.clone())).await;
                        url
                    }
                }
                None => Action::format_route(module, class, action, param),
            }
        } else {
            Action::format_route(module, class, action, param)
        }
    }
    /// Send email
    pub async fn mail(&self, message: MailMessage) -> bool {
        let provider = {
            let mail = self.mail.lock().await;
            mail.provider.clone()
        };
        Mail::send(provider, Arc::clone(&self.db), message, self.session.user_id, self.request.host.clone()).await
    }

    /// Get not_found url
    pub async fn not_found(&mut self) -> String {
        if !self.db.in_use() {
            let install = Route::default_install();
            return format!("/{}/{}/not_found", install.module, install.class);
        }
        let key = vec![fnv1a_64!("404"), self.session.get_lang_id()];
        let (data, key) = self.cache.get(key).await;
        match data {
            Some(d) => match d {
                Data::String(url) => url,
                _ => match &self.not_found.param {
                    Some(param) => {
                        format!("/{}/{}/{}/{}", self.not_found.module, self.not_found.class, self.not_found.action, param)
                    }
                    None => {
                        format!("/{}/{}/{}", self.not_found.module, self.not_found.class, self.not_found.action)
                    }
                },
            },
            None => {
                // Load from database
                match self.db.query_prepare(fnv1a_64!("lib_get_not_found"), &[&self.session.get_lang_id()], false).await {
                    Some(v) => {
                        if v.is_empty() {
                            self.cache.set(key, Data::None).await;
                            match &self.not_found.param {
                                Some(param) => {
                                    format!("/{}/{}/{}/{}", self.not_found.module, self.not_found.class, self.not_found.action, param)
                                }
                                None => format!("/{}/{}/{}", self.not_found.module, self.not_found.class, self.not_found.action),
                            }
                        } else if let Data::Vec(row) = unsafe { v.get_unchecked(0) } {
                            if !row.is_empty() {
                                if let Data::String(url) = unsafe { row.get_unchecked(0) } {
                                    self.cache.set(key, Data::String(url.clone())).await;
                                    url.clone()
                                } else {
                                    self.cache.set(key, Data::None).await;
                                    match &self.not_found.param {
                                        Some(param) => format!(
                                            "/{}/{}/{}/{}",
                                            self.not_found.module, self.not_found.class, self.not_found.action, param
                                        ),
                                        None => {
                                            format!("/{}/{}/{}", self.not_found.module, self.not_found.class, self.not_found.action)
                                        }
                                    }
                                }
                            } else {
                                self.cache.set(key, Data::None).await;
                                match &self.not_found.param {
                                    Some(param) => {
                                        format!("/{}/{}/{}/{}", self.not_found.module, self.not_found.class, self.not_found.action, param)
                                    }
                                    None => {
                                        format!("/{}/{}/{}", self.not_found.module, self.not_found.class, self.not_found.action)
                                    }
                                }
                            }
                        } else {
                            self.cache.set(key, Data::None).await;
                            match &self.not_found.param {
                                Some(param) => {
                                    format!("/{}/{}/{}/{}", self.not_found.module, self.not_found.class, self.not_found.action, param)
                                }
                                None => format!("/{}/{}/{}", self.not_found.module, self.not_found.class, self.not_found.action),
                            }
                        }
                    }
                    None => match &self.not_found.param {
                        Some(param) => {
                            format!("/{}/{}/{}/{}", self.not_found.module, self.not_found.class, self.not_found.action, param)
                        }
                        None => format!("/{}/{}/{}", self.not_found.module, self.not_found.class, self.not_found.action),
                    },
                }
            }
        }
    }

    /// Run execute of controller
    pub(crate) async fn run(action: &mut Action) -> Answer {
        action.start_route(action.module_id, action.class_id, action.action_id, action.param.clone(), false).await
    }

    /// Finish work of controller
    pub(crate) async fn end(action: Action) {
        // Save session
        Session::save_session(action.db, &action.session, &action.request).await;
        // Remove temp file
        for val in action.request.input.file.values() {
            for f in val {
                if let Err(e) = remove_file(&f.tmp).await {
                    Log::warning(1103, Some(format!("filename={}. Error={}", &f.tmp.display(), e)));
                };
            }
        }
    }

    /// Simple remove temp file after redirect
    pub(crate) async fn clean_file(file: Vec<PathBuf>) {
        for f in file {
            if let Err(e) = remove_file(&f).await {
                Log::warning(1103, Some(format!("filename={}. Error={}", f.display(), e)));
            };
        }
    }

    /// Start internal route
    async fn start_route(&mut self, module_id: i64, class_id: i64, action_id: i64, param: Option<String>, internal: bool) -> Answer {
        // Check permission
        if self.get_access(module_id, class_id, action_id).await {
            if let Some(answer) = self.invoke(module_id, class_id, action_id, param, internal).await {
                return answer;
            };
        }
        if internal {
            return Answer::None;
        }
        if self.request.ajax {
            self.response.http_code = Some(404);
            return Answer::None;
        }

        // If not /index/index/not_found - then redirect
        if !(module_id == self.not_found.module_id && class_id == self.not_found.class_id && class_id == self.not_found.action_id) {
            self.response.redirect = Some(Redirect {
                url: self.not_found().await,
                permanently: false,
            });
        }
        Answer::None
    }

    /// Invoke found controller
    async fn invoke(&mut self, module_id: i64, class_id: i64, action_id: i64, param: Option<String>, internal: bool) -> Option<Answer> {
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
                        return Some(res);
                    } else {
                        // Call from the different module as the current one

                        // Load new template list
                        #[cfg(not(debug_assertions))]
                        let h = match self.template.list.get(&module_id) {
                            Some(h) => match h.get(&class_id) {
                                Some(h) => self.html.replace(Arc::clone(h)),
                                None => self.html.take(),
                            },
                            None => self.html.take(),
                        };
                        #[cfg(debug_assertions)]
                        let h = match self.template.read().await.list.get(&module_id) {
                            Some(h) => match h.get(&class_id) {
                                Some(h) => self.html.replace(Arc::clone(h)),
                                None => self.html.take(),
                            },
                            None => self.html.take(),
                        };
                        // Load new translate list
                        #[cfg(not(debug_assertions))]
                        let l = match self.language.list.get(&self.session.get_lang_id()) {
                            Some(l) => match l.get(&module_id) {
                                Some(l) => match l.get(&class_id) {
                                    Some(l) => self.lang.replace(Arc::clone(l)),
                                    None => self.lang.take(),
                                },
                                None => self.lang.take(),
                            },
                            None => self.lang.take(),
                        };
                        #[cfg(debug_assertions)]
                        let l = match self.language.read().await.list.get(&self.session.get_lang_id()) {
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
                        return Some(res);
                    }
                }
            }
        }
        None
    }

    fn error_route(action_err: Arc<Route>) -> Route {
        Route::clone(&action_err)
    }

    /// Extract route from url
    async fn extract_route(
        request: &Request,
        cache: Arc<Mutex<CacheSys>>,
        db: Arc<DB>,
        action_index: Arc<Route>,
        action_err: Arc<Route>,
    ) -> Result<Route, Redirect> {
        if db.in_use() {
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
                    match db.query_prepare(fnv1a_64!("lib_get_redirect"), &[&request.url], false).await {
                        Some(v) => {
                            if v.is_empty() {
                                CacheSys::set(Arc::clone(&cache), &key, Data::None).await;
                            } else {
                                let row = if let Data::Vec(row) = unsafe { v.get_unchecked(0) } {
                                    row
                                } else {
                                    return Ok(Action::error_route(action_err));
                                };
                                if row.len() != 2 {
                                    return Ok(Action::error_route(action_err));
                                }
                                let url = if let Data::String(url) = unsafe { row.get_unchecked(0) } {
                                    url.to_owned()
                                } else {
                                    return Ok(Action::error_route(action_err));
                                };
                                let permanently = if let Data::Bool(permanently) = unsafe { row.get_unchecked(1) } {
                                    *permanently
                                } else {
                                    return Ok(Action::error_route(action_err));
                                };
                                let r = Redirect { url, permanently };
                                CacheSys::set(Arc::clone(&cache), &key, Data::Redirect(r.clone())).await;
                                return Err(r);
                            }
                        }
                        None => return Ok(Action::error_route(action_err)),
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
                    match db.query_prepare(fnv1a_64!("lib_get_route"), &[&request.url], false).await {
                        Some(v) => {
                            if v.is_empty() {
                                CacheSys::set(Arc::clone(&cache), &key, Data::None).await;
                            } else {
                                let row = if let Data::Vec(row) = unsafe { v.get_unchecked(0) } {
                                    row
                                } else {
                                    return Ok(Action::error_route(action_err));
                                };
                                if row.len() != 8 {
                                    return Ok(Action::error_route(action_err));
                                }
                                let module = if let Data::String(module) = unsafe { row.get_unchecked(0) } {
                                    module.to_owned()
                                } else {
                                    return Ok(Action::error_route(action_err));
                                };
                                let class = if let Data::String(class) = unsafe { row.get_unchecked(1) } {
                                    class.to_owned()
                                } else {
                                    return Ok(Action::error_route(action_err));
                                };
                                let action = if let Data::String(action) = unsafe { row.get_unchecked(2) } {
                                    action.to_owned()
                                } else {
                                    return Ok(Action::error_route(action_err));
                                };
                                let param = match unsafe { row.get_unchecked(6) } {
                                    Data::None => None,
                                    Data::String(param) => {
                                        if param.is_empty() {
                                            None
                                        } else {
                                            Some(param.to_owned())
                                        }
                                    }
                                    _ => return Ok(Action::error_route(action_err)),
                                };
                                let module_id = if let Data::I64(module_id) = unsafe { row.get_unchecked(3) } {
                                    *module_id
                                } else {
                                    return Ok(Action::error_route(action_err));
                                };
                                let class_id = if let Data::I64(class_id) = unsafe { row.get_unchecked(4) } {
                                    *class_id
                                } else {
                                    return Ok(Action::error_route(action_err));
                                };
                                let action_id = if let Data::I64(action_id) = unsafe { row.get_unchecked(5) } {
                                    *action_id
                                } else {
                                    return Ok(Action::error_route(action_err));
                                };
                                let lang_id = match unsafe { row.get_unchecked(7) } {
                                    Data::None => None,
                                    Data::I64(lang_id) => Some(*lang_id),
                                    _ => return Ok(Action::error_route(action_err)),
                                };
                                let r = Route {
                                    module,
                                    class,
                                    action,
                                    module_id,
                                    class_id,
                                    action_id,
                                    param,
                                    lang_id,
                                };
                                CacheSys::set(Arc::clone(&cache), &key, Data::Route(r.clone())).await;
                                return Ok(r);
                            }
                        }
                        None => return Ok(Action::error_route(action_err)),
                    }
                }
            }
        }

        if request.url != "/" {
            let mut load: Vec<&str> = request.url.splitn(5, '/').collect();
            load.retain(|&x| !x.is_empty());
            let r = match load.len() {
                1 => {
                    if db.in_use() {
                        let module = unsafe { *load.get_unchecked(0) };
                        Route {
                            module: module.to_owned(),
                            class: action_index.class.clone(),
                            action: action_index.action.clone(),
                            module_id: fnv1a_64(module.as_bytes()),
                            class_id: action_index.class_id,
                            action_id: action_index.action_id,
                            param: None,
                            lang_id: None,
                        }
                    } else {
                        Route::default_install()
                    }
                }
                2 => {
                    if db.in_use() {
                        let module = unsafe { *load.get_unchecked(0) };
                        let class = unsafe { *load.get_unchecked(1) };
                        Route {
                            module: module.to_owned(),
                            class: class.to_owned(),
                            action: action_index.action.clone(),
                            module_id: fnv1a_64(module.as_bytes()),
                            class_id: fnv1a_64(class.as_bytes()),
                            action_id: action_index.action_id,
                            param: None,
                            lang_id: None,
                        }
                    } else {
                        Route::default_install()
                    }
                }
                3 => {
                    if db.in_use() {
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
                    } else {
                        let install = Route::default_install();
                        let action = unsafe { *load.get_unchecked(2) };
                        Route {
                            module: install.module.to_owned(),
                            class: install.class.to_owned(),
                            action: action.to_owned(),
                            module_id: install.module_id,
                            class_id: install.class_id,
                            action_id: fnv1a_64(action.as_bytes()),
                            param: None,
                            lang_id: None,
                        }
                    }
                }
                4 => {
                    if db.in_use() {
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
                    } else {
                        let install = Route::default_install();
                        let action = unsafe { *load.get_unchecked(2) };
                        let param = unsafe { *load.get_unchecked(3) };
                        Route {
                            module: install.module.to_owned(),
                            class: install.class.to_owned(),
                            action: action.to_owned(),
                            module_id: install.module_id,
                            class_id: install.class_id,
                            action_id: fnv1a_64(action.as_bytes()),
                            param: Some(param.to_owned()),
                            lang_id: None,
                        }
                    }
                }
                _ => {
                    if db.in_use() {
                        Route::clone(&action_index)
                    } else {
                        Route::default_install()
                    }
                }
            };
            Ok(r)
        } else if db.in_use() {
            Ok(Route::clone(&action_index))
        } else {
            Ok(Route::default_install())
        }
    }

    fn format_route(module: &str, class: &str, action: &str, param: Option<&str>) -> String {
        match param {
            Some(s) => {
                format!("/{}/{}/{}/{}", module, class, action, s)
            }
            None => format!("/{}/{}/{}", module, class, action),
        }
    }
}
