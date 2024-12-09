use std::{collections::HashMap, future::Future, pin::Pin, sync::Arc};

#[cfg(feature = "file-disk")]
use std::io::ErrorKind;

use tokio::{
    sync::mpsc::Sender,
    task::{yield_now, JoinHandle},
};

#[cfg(any(
    feature = "html-static",
    feature = "html-reload",
    all(feature = "session-db", any(feature = "lang-static", feature = "lang-reload")),
    feature = "redirect-db",
    feature = "route-db",
    feature = "setting-db",
    feature = "access-db",
))]
use tiny_web_macro::fnv1a_64 as m_fnv1a_64;

#[cfg(feature = "file-disk")]
use tokio::fs::remove_file;

#[cfg(any(feature = "html-reload", feature = "lang-reload"))]
use tokio::sync::RwLock;

use crate::{
    fnv1a_64,
    sys::{
        net::{stream::MessageWrite, worker::Worker},
        stat::stat::Stat,
    },
};

#[cfg(any(
    feature = "file-disk",
    all(feature = "cache", any(feature = "route-db", feature = "access-db"))
))]
use crate::log;

#[cfg(any(feature = "mail-sendmail", feature = "mail-smtp", feature = "mail-file"))]
use crate::sys::app::init::MailConfig;

#[cfg(any(feature = "pgsql", feature = "mssql"))]
use crate::sys::db::adapter::DB;

use super::{
    data::{Data, StrOrI64},
    request::{Request, Route},
    response::Response,
};

#[cfg(feature = "cache")]
use super::cache::Cache;

#[cfg(any(feature = "html-static", feature = "html-reload"))]
use super::html::{Html, Nodes};

#[cfg(any(feature = "lang-static", feature = "lang-reload"))]
use super::lang::{Lang, LangItem};

#[cfg(any(
    feature = "mail-sendmail",
    feature = "mail-smtp",
    feature = "mail-file",
    feature = "mail-db"
))]
use super::mail::{Mail, MailMessage};

#[cfg(feature = "redirect-db")]
use super::response::Redirect;

#[cfg(feature = "file-disk")]
use super::request::WebFile;

#[cfg(any(feature = "session-memory", feature = "session-file", feature = "session-db"))]
use super::session::{Flash, Session, SessionLoader};

pub type Act = fn(&mut Action) -> Pin<Box<dyn Future<Output = Answer> + Send + '_>>;
pub type ActionMap = HashMap<i64, Act>;
pub type ClassMap = HashMap<i64, ActionMap>;
pub type ModuleMap = HashMap<i64, ClassMap>;

/// Type of Answer  
#[derive(Debug)]
pub enum Answer {
    /// Answer in the form of text.
    String(String),
    /// Answer in binary data
    Raw(Vec<u8>),
    /// Without answer
    None,
}

pub(crate) enum ActionRedirect {
    Action(Action),
    #[cfg(feature = "redirect-db")]
    Redirect(Redirect),
}

#[derive(Debug)]
pub(crate) struct ActionData {
    pub id: u64,
    pub mon: Arc<Stat>,
    pub engine: Arc<ModuleMap>,
    pub salt: Arc<String>,
    pub request: Request,
    pub tx: Arc<Sender<MessageWrite>>,
    pub index: Arc<[i64; 3]>,
    pub not_found: Option<Arc<[i64; 3]>>,
    #[cfg(any(feature = "pgsql", feature = "mssql"))]
    pub db: Arc<DB>,

    #[cfg(feature = "html-static")]
    pub html: Arc<Html>,
    #[cfg(feature = "html-reload")]
    pub html: Arc<RwLock<Html>>,
    #[cfg(any(feature = "session-memory", feature = "session-file", feature = "session-db"))]
    pub session_loader: Arc<SessionLoader>,
    #[cfg(any(feature = "session-memory", feature = "session-file", feature = "session-db"))]
    pub session: Option<String>,
    #[cfg(feature = "lang-static")]
    pub lang: Arc<Lang>,
    #[cfg(feature = "lang-reload")]
    pub lang: Arc<RwLock<Lang>>,
    #[cfg(any(feature = "mail-sendmail", feature = "mail-smtp", feature = "mail-file"))]
    pub mail: Arc<MailConfig>,
    #[cfg(feature = "cache")]
    pub cache: Arc<Cache>,
}

#[cfg(any(feature = "redirect-db", feature = "route-db"))]
struct RouteRedirectParam<'a> {
    db: Arc<DB>,
    url: &'a str,
    #[cfg(feature = "cache")]
    cache: Arc<Cache>,
}

#[derive(Debug)]
pub struct Action {
    pub id: u64,
    pub request: Request,
    pub response: Response,
    pub salt: Arc<String>,
    pub internal: bool,
    pub monitor: Arc<Stat>,
    #[cfg(any(feature = "pgsql", feature = "mssql"))]
    pub db: Arc<DB>,
    #[cfg(any(feature = "session-memory", feature = "session-file", feature = "session-db"))]
    pub session: Session,
    #[cfg(feature = "cache")]
    pub cache: Arc<Cache>,

    pub(crate) header_send: bool,
    pub(crate) tx: Arc<Sender<MessageWrite>>,

    current_module_id: i64,
    current_class_id: i64,
    route: Route,
    data: HashMap<i64, Data>,
    engine: Arc<ModuleMap>,
    not_found: Option<Arc<[i64; 3]>>,
    #[cfg(any(feature = "html-static", feature = "html-reload"))]
    html: Option<Arc<HashMap<i64, Nodes>>>,
    #[cfg(feature = "html-static")]
    template: Arc<Html>,
    #[cfg(feature = "html-reload")]
    template: Arc<RwLock<Html>>,
    #[cfg(any(feature = "lang-static", feature = "lang-reload"))]
    lang: Option<Arc<HashMap<i64, String>>>,
    #[cfg(feature = "lang-static")]
    language: Arc<Lang>,
    #[cfg(feature = "lang-reload")]
    language: Arc<RwLock<Lang>>,
    #[cfg(any(feature = "lang-static", feature = "lang-reload"))]
    lang_id: usize,
    #[cfg(any(feature = "mail-sendmail", feature = "mail-smtp", feature = "mail-file"))]
    mail: Arc<MailConfig>,
}

impl Action {
    pub async fn load(&mut self, module: impl StrOrI64, class: impl StrOrI64, action: impl StrOrI64, param: Option<String>) -> Answer {
        let route = Route {
            module_id: module.to_i64(),
            class_id: class.to_i64(),
            action_id: action.to_i64(),
            param,
            #[cfg(any(feature = "lang-static", feature = "lang-reload"))]
            lang_id: None,
        };
        self.start_route(route, true).await
    }

    pub fn set<T>(&mut self, key: impl StrOrI64, value: T)
    where
        T: Into<Data>,
    {
        self.data.insert(key.to_i64(), value.into());
    }

    pub fn get<T>(&self, key: impl StrOrI64) -> Option<&T>
    where
        for<'a> &'a T: From<&'a Data>,
    {
        self.data.get(&key.to_i64()).map(|value| value.into())
    }

    pub fn take<T>(&mut self, key: impl StrOrI64) -> Option<T>
    where
        T: From<Data>,
    {
        self.data.remove(&key.to_i64()).map(|value| value.into())
    }

    #[cfg(feature = "setting-db")]
    pub async fn get_setting(&self, key: impl StrOrI64) -> Option<String> {
        let res = self.db.query_prepare(m_fnv1a_64!("lib_get_setting"), &[&key.to_i64()]).await?;
        if res.is_empty() {
            return None;
        }
        let row = unsafe { res.get_unchecked(0) };
        Some(row.get(0))
    }

    /// Set flash message to session data
    #[cfg(any(feature = "session-memory", feature = "session-file", feature = "session-db"))]
    pub async fn set_flash(&mut self, kind: Flash, value: String) {
        self.session.set_flash(kind, value);
    }

    /// Take flash message from session data
    #[cfg(any(feature = "session-memory", feature = "session-file", feature = "session-db"))]
    pub async fn take_flash(&mut self) -> Option<HashMap<Flash, Vec<String>>> {
        self.session.take_flash()
    }

    /// Get translate
    #[cfg(any(feature = "lang-static", feature = "lang-reload"))]
    pub fn lang(&self, text: impl StrOrI64) -> String {
        if let Some(l) = &self.lang {
            if let Some(str) = l.get(&text.to_i64()) {
                return str.to_owned();
            }
        }
        if text.is_str() {
            text.to_str().to_owned()
        } else {
            text.to_i64().to_string()
        }
    }

    #[cfg(any(feature = "lang-static", feature = "lang-reload"))]
    pub fn set_lang(&mut self, key: impl StrOrI64) {
        let idkey = key.to_i64();
        if let Some(l) = &self.lang {
            if let Some(str) = l.get(&idkey) {
                self.data.insert(idkey, Data::String(str.to_owned()));
                return;
            }
        }
        if key.is_str() {
            self.data.insert(idkey, Data::String(format!("{{{}}}", key.to_str())));
        } else {
            self.data.insert(idkey, Data::String(format!("{{{}}}", key.to_i64())));
        }
    }

    /// Set an array of values for the template from the translation
    #[cfg(any(feature = "lang-static", feature = "lang-reload"))]
    pub fn set_lang_arr(&mut self, keys: &[impl StrOrI64]) {
        for key in keys {
            let idkey = key.to_i64();
            if let Some(l) = &self.lang {
                if let Some(str) = l.get(&idkey) {
                    self.data.insert(idkey, Data::String(str.to_owned()));
                    continue;
                }
            }
            if key.is_str() {
                self.data.insert(idkey, Data::String(format!("{{{}}}", key.to_str())));
            } else {
                self.data.insert(idkey, Data::String(format!("{{{}}}", key.to_i64())));
            }
        }
    }

    /// Get current lang
    #[cfg(any(feature = "lang-static", feature = "lang-reload"))]
    pub async fn lang_current(&self) -> Arc<LangItem> {
        let lang_id = self.lang_id;
        #[cfg(feature = "lang-static")]
        {
            Arc::clone(unsafe { self.language.langs.get_unchecked(lang_id) })
        }
        #[cfg(feature = "lang-reload")]
        {
            Arc::clone(unsafe { self.language.read().await.langs.get_unchecked(lang_id) })
        }
    }

    #[cfg(any(feature = "lang-static", feature = "lang-reload"))]
    pub async fn lang_list(&self) -> Arc<Vec<Arc<LangItem>>> {
        #[cfg(feature = "lang-static")]
        {
            Arc::clone(&self.language.langs)
        }
        #[cfg(feature = "lang-reload")]
        {
            Arc::clone(&self.language.read().await.langs)
        }
    }

    #[cfg(any(feature = "lang-static", feature = "lang-reload"))]
    pub async fn lang_list_all(&self) -> Arc<Vec<Arc<LangItem>>> {
        #[cfg(feature = "session-db")]
        {
            match self.db.query_prepare(m_fnv1a_64!("lib_get_all_langs"), &[]).await {
                Some(list) => {
                    let mut vec = Vec::with_capacity(list.len());
                    for row in list {
                        let id: i64 = row.get(0);
                        let code = row.get(1);
                        let name = row.get(2);
                        let index: i64 = match row.try_get(3) {
                            Ok(index) => index,
                            Err(_) => panic!("Not set lang_id index for lang {}", id),
                        };
                        let lang = LangItem {
                            id: id as usize,
                            code,
                            name,
                            index: index as usize,
                        };
                        vec.push(Arc::new(lang));
                    }
                    Arc::new(vec)
                }
                None => Arc::new(Vec::new()),
            }
        }
        #[cfg(not(feature = "session-db"))]
        {
            #[cfg(feature = "lang-static")]
            {
                Arc::clone(&self.language.langs)
            }
            #[cfg(feature = "lang-reload")]
            {
                Arc::clone(&self.language.read().await.langs)
            }
        }
    }

    pub fn spawn<F>(&self, future: F) -> JoinHandle<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        tokio::spawn(future)
    }

    pub async fn write(&mut self, answer: Answer) {
        let vec = match answer {
            Answer::String(str) => str.as_bytes().to_vec(),
            Answer::Raw(raw) => raw,
            Answer::None => return,
        };
        Worker::write(self, vec).await;
        self.header_send = true;
        yield_now().await;
    }

    /// Get url
    #[cfg(all(feature = "route-db", any(feature = "lang-static", feature = "lang-reload")))]
    pub async fn get_url(&mut self, module: &str, class: &str, action: &str, param: Option<&str>, lang_id: Option<usize>) -> String {
        let lang_id = match lang_id {
            Some(lang_id) => lang_id,
            None => self.lang_id,
        };
        self.get_url_query(module, class, action, param, Some(lang_id)).await
    }

    #[cfg(all(feature = "route-db", not(any(feature = "lang-static", feature = "lang-reload"))))]
    pub async fn get_url(&mut self, module: &str, class: &str, action: &str, param: Option<&str>) -> String {
        self.get_url_query(module, class, action, param, None).await
    }

    #[cfg(feature = "route-db")]
    async fn get_url_query(&mut self, module: &str, class: &str, action: &str, param: Option<&str>, lang_id: Option<usize>) -> String {
        let module_id = fnv1a_64(module.as_bytes());
        let class_id = fnv1a_64(class.as_bytes());
        let action_id = fnv1a_64(action.as_bytes());

        #[cfg(feature = "cache")]
        let param_key = match param {
            Some(param) => fnv1a_64(param.as_bytes()),
            None => m_fnv1a_64!(""),
        };
        #[cfg(feature = "cache")]
        let lang_key = match lang_id {
            Some(lang_id) => lang_id as i64,
            None => -1,
        };
        #[cfg(feature = "cache")]
        let cache_key = format!("sys:url:{}.{}.{}.{}.{}", module_id, class_id, action_id, param_key, lang_key);
        #[cfg(feature = "cache")]
        if let Some(result) = self.cache.get(&cache_key).await {
            match result {
                Data::None => return Action::format_route(module, class, action, param),
                Data::String(url) => return url,
                _ => {}
            }
            log!(warning, 0, "{}", key);
            self.cache.remove(&cache_key).await;
        }

        let lang_id = lang_id.map(|x| x as i64);
        match self.db.query_prepare(m_fnv1a_64!("lib_get_url"), &[&module_id, &class_id, &action_id, &param, &lang_id]).await {
            Some(rows) => {
                if rows.is_empty() {
                    #[cfg(feature = "cache")]
                    self.cache.set(&cache_key, Data::None).await;
                    Action::format_route(module, class, action, param)
                } else {
                    let row = unsafe { rows.get_unchecked(0) };
                    let url: String = row.get(0);
                    #[cfg(feature = "cache")]
                    self.cache.set(&cache_key, Data::String(url.clone())).await;
                    url
                }
            }
            None => Action::format_route(module, class, action, param),
        }
    }

    /// Render template
    #[cfg(any(feature = "html-static", feature = "html-reload"))]
    pub fn render(&mut self, template: impl StrOrI64) -> Answer {
        match &self.html {
            Some(h) => match h.get(&template.to_i64()) {
                Some(vec) => {
                    if !self.response.css.is_empty() {
                        let mut vec = Vec::with_capacity(self.response.css.len());
                        for css in self.response.css.drain(..) {
                            vec.push(Data::String(css));
                        }
                        self.data.insert(m_fnv1a_64!("css"), Data::Vec(vec));
                    }
                    if !self.response.js.is_empty() {
                        let mut vec = Vec::with_capacity(self.response.js.len());
                        for js in self.response.js.drain(..) {
                            vec.push(Data::String(js));
                        }
                        self.data.insert(m_fnv1a_64!("js"), Data::Vec(vec));
                    }
                    if !self.response.meta.is_empty() {
                        let mut vec = Vec::with_capacity(self.response.meta.len());
                        for meta in self.response.meta.drain(..) {
                            vec.push(Data::String(meta));
                        }
                        self.data.insert(m_fnv1a_64!("meta"), Data::Vec(vec));
                    }
                    Html::render(&self.data, vec)
                }
                None => Answer::String(format!("{{{}}}", template.to_str())),
            },
            None => Answer::String(format!("{{{}}}", template.to_str())),
        }
    }

    /// Get access to run controller
    #[cfg(feature = "access-db")]
    pub async fn get_access(&self, module: impl StrOrI64, class: impl StrOrI64, action: impl StrOrI64) -> bool {
        let module_id = module.to_i64();
        let class_id = class.to_i64();
        let action_id = action.to_i64();
        let role_id = match self.session.role_id {
            Some(role_id) => role_id as i64,
            None => 0,
        };
        #[cfg(feature = "cache")]
        let cache_key = format!("sys:access:{}.{}.{}.{}", module_id, class_id, action_id, role_id);
        #[cfg(feature = "cache")]
        if let Some(result) = self.cache.get(&cache_key).await {
            if let Data::Bool(access) = result {
                return access;
            }
            log!(warning, 0, "{}", key);
            self.cache.remove(&cache_key).await;
        }

        // Prepare sql query
        let res = self
            .db
            .query_prepare(m_fnv1a_64!("lib_get_auth"), &[&role_id, &module_id, &module_id, &module_id, &class_id, &class_id, &action_id])
            .await;
        match res {
            Some(rows) => {
                if !rows.is_empty() {
                    let row = unsafe { rows.get_unchecked(0) };
                    let access = row.get(0);
                    #[cfg(feature = "cache")]
                    self.cache.set(&cache_key, Data::Bool(access)).await;
                    access
                } else {
                    #[cfg(feature = "cache")]
                    self.cache.set(&cache_key, Data::Bool(false)).await;
                    false
                }
            }
            None => false,
        }
    }

    #[cfg(any(
        feature = "mail-sendmail",
        feature = "mail-smtp",
        feature = "mail-file",
        feature = "mail-db"
    ))]
    pub async fn mail(&self, message: MailMessage<'_>) -> Result<(), ()> {
        #[cfg(any(feature = "mail-sendmail", feature = "mail-smtp", feature = "mail-file"))]
        {
            Mail::send(Arc::clone(&self.mail), &self.request.host, message).await
        }
        #[cfg(feature = "mail-db")]
        {
            Mail::send(self, message).await
        }
    }

    #[cfg(feature = "route-db")]
    fn format_route(module: &str, class: &str, action: &str, param: Option<&str>) -> String {
        match param {
            Some(s) => {
                format!("/{}/{}/{}/{}", module, class, action, s)
            }
            None => format!("/{}/{}/{}", module, class, action),
        }
    }

    #[cfg(feature = "redirect-db")]
    async fn check_redirect(param: &RouteRedirectParam<'_>) -> Result<Option<Redirect>, ()> {
        #[cfg(feature = "cache")]
        let key = fnv1a_64(param.url.as_bytes());
        #[cfg(feature = "cache")]
        let cache_key = format!("sys:redirect:{}", key);
        #[cfg(feature = "cache")]
        if let Some(result) = Redirect::get(Arc::clone(&param.cache), &cache_key).await {
            return Ok(result);
        }

        let res = match param.db.query_prepare(m_fnv1a_64!("lib_get_redirect"), &[&param.url]).await {
            Some(res) => res,
            None => return Err(()),
        };
        if !res.is_empty() {
            let row = unsafe { res.get_unchecked(0) };
            let redirect = {
                let url = row.get(0);
                let permanently = row.get(1);
                let redirect = Redirect { url, permanently };
                #[cfg(feature = "cache")]
                Redirect::set(Arc::clone(&param.cache), &cache_key, Some(&redirect)).await;
                redirect
            };
            Ok(Some(redirect))
        } else {
            #[cfg(feature = "cache")]
            Redirect::set(Arc::clone(&param.cache), &cache_key, None).await;
            Ok(None)
        }
    }

    #[cfg(feature = "route-db")]
    async fn check_route(param: &RouteRedirectParam<'_>) -> Result<Option<Route>, ()> {
        #[cfg(feature = "cache")]
        let key = fnv1a_64(param.url.as_bytes());
        #[cfg(feature = "cache")]
        let cache_key = format!("sys:route:{}", key);
        #[cfg(feature = "cache")]
        if let Some(result) = Route::get(Arc::clone(&param.cache), &cache_key).await {
            return Ok(result);
        }
        let res = match param.db.query_prepare(m_fnv1a_64!("lib_get_route"), &[&param.url]).await {
            Some(res) => res,
            None => return Err(()),
        };
        if !res.is_empty() {
            let row = unsafe { res.get_unchecked(0) };
            let route = {
                let module_id = row.get(0);
                let class_id = row.get(1);
                let action_id = row.get(2);
                let param = row.get(3);
                #[cfg(any(feature = "lang-static", feature = "lang-reload"))]
                let lang_id: Option<i64> = row.get(4);

                Route {
                    module_id,
                    class_id,
                    action_id,
                    param,
                    #[cfg(any(feature = "lang-static", feature = "lang-reload"))]
                    lang_id: lang_id.map(|x| x as usize),
                }
            };
            #[cfg(feature = "cache")]
            Route::set(Arc::clone(&param.cache), &cache_key, Some(&route)).await;
            Ok(Some(route))
        } else {
            #[cfg(feature = "cache")]
            Route::set(Arc::clone(&param.cache), &cache_key, None).await;
            Ok(None)
        }
    }

    pub(crate) async fn init(data: ActionData) -> Result<ActionRedirect, ()> {
        #[cfg(any(feature = "redirect-db", feature = "route-db"))]
        let param = RouteRedirectParam {
            db: Arc::clone(&data.db),
            url: &data.request.url,
            #[cfg(feature = "cache")]
            cache: Arc::clone(&data.cache),
        };
        #[cfg(feature = "redirect-db")]
        match Action::check_redirect(&param).await {
            Ok(Some(redirect)) => {
                #[cfg(feature = "file-disk")]
                tokio::spawn(async move {
                    Action::clean_file(data.request.input.file).await;
                });
                return Ok(ActionRedirect::Redirect(redirect));
            }
            Err(_) => {
                #[cfg(feature = "file-disk")]
                tokio::spawn(async move {
                    Action::clean_file(data.request.input.file).await;
                });
                return Err(());
            }
            _ => {}
        }
        #[cfg(feature = "route-db")]
        let route = match Action::check_route(&param).await {
            Ok(Some(route)) => route,
            Err(_) => {
                #[cfg(feature = "file-disk")]
                tokio::spawn(async move {
                    Action::clean_file(data.request.input.file).await;
                });
                return Err(());
            }
            _ => Action::extract_route(&data.request, data.index),
        };
        #[cfg(not(feature = "route-db"))]
        let route = Action::extract_route(&data.request, data.index);

        let response = Response {
            redirect: None,
            content_type: None,
            headers: Vec::new(),
            http_code: None,
            css: Vec::new(),
            js: Vec::new(),
            meta: Vec::new(),
        };

        let current_module_id = route.module_id;
        let current_class_id = route.class_id;

        #[cfg(feature = "html-static")]
        let html = data.html.list.get(&current_module_id).and_then(|module| module.get(&current_class_id).cloned());
        #[cfg(feature = "html-reload")]
        let html = data.html.read().await.list.get(&current_module_id).and_then(|module| module.get(&current_class_id).cloned());
        #[cfg(any(feature = "session-memory", feature = "session-file", feature = "session-db"))]
        let session = match data.session_loader.load(data.session).await {
            Ok(session) => session,
            Err(_) => {
                #[cfg(feature = "file-disk")]
                tokio::spawn(async move {
                    Action::clean_file(data.request.input.file).await;
                });
                return Err(());
            }
        };

        #[cfg(any(feature = "lang-static", feature = "lang-reload"))]
        let lang_id = if let Some(lang_id) = route.lang_id {
            lang_id
        } else if let Some(lang_id) = session.get_lang_id() {
            lang_id
        } else {
            #[cfg(feature = "lang-static")]
            {
                data.lang.default
            }
            #[cfg(feature = "lang-reload")]
            {
                data.lang.read().await.default
            }
        };

        #[cfg(feature = "lang-static")]
        let lang = data
            .lang
            .list
            .get(&lang_id)
            .and_then(|langs| langs.get(&current_module_id))
            .and_then(|module| module.get(&current_class_id).cloned());
        #[cfg(feature = "lang-reload")]
        let lang = data
            .lang
            .read()
            .await
            .list
            .get(&lang_id)
            .and_then(|langs| langs.get(&current_module_id))
            .and_then(|module| module.get(&current_class_id).cloned());

        Ok(ActionRedirect::Action(Action {
            id: data.id,
            request: data.request,
            response,
            salt: data.salt,
            route,
            internal: false,
            monitor: data.mon,
            #[cfg(any(feature = "pgsql", feature = "mssql"))]
            db: data.db,
            #[cfg(feature = "cache")]
            cache: data.cache,

            header_send: false,
            tx: data.tx,

            current_module_id,
            current_class_id,
            data: HashMap::new(),
            engine: data.engine,
            not_found: data.not_found,
            #[cfg(any(feature = "html-static", feature = "html-reload"))]
            html,
            #[cfg(any(feature = "html-static", feature = "html-reload"))]
            template: data.html,
            #[cfg(any(feature = "session-memory", feature = "session-file", feature = "session-db"))]
            session,
            #[cfg(any(feature = "lang-static", feature = "lang-reload"))]
            lang,
            #[cfg(any(feature = "lang-static", feature = "lang-reload"))]
            language: data.lang,
            #[cfg(any(feature = "lang-static", feature = "lang-reload"))]
            lang_id,
            #[cfg(any(feature = "mail-sendmail", feature = "mail-smtp", feature = "mail-file"))]
            mail: data.mail,
        }))
    }

    pub(crate) async fn run(action: &mut Action) -> Vec<u8> {
        let answer = match action.start_route(action.route.clone(), false).await {
            Answer::String(str) => str.as_bytes().to_vec(),
            Answer::Raw(vec) => vec,
            Answer::None => Vec::new(),
        };
        answer
    }

    async fn start_route(&mut self, route: Route, internal: bool) -> Answer {
        #[cfg(feature = "access-db")]
        if self.get_access(route.module_id, route.class_id, route.action_id).await {
            if let Some(answer) = self.invoke(route.module_id, route.class_id, route.action_id, route.param, internal).await {
                return answer;
            };
        }
        #[cfg(not(feature = "access-db"))]
        if let Some(answer) = self.invoke(route.module_id, route.class_id, route.action_id, route.param, internal).await {
            return answer;
        };
        if !internal && !self.request.ajax {
            self.response.http_code = Some(404);
            if let Some(not_found) = &self.not_found {
                if let Some(answer) = self
                    .invoke(
                        unsafe { *not_found.get_unchecked(0) },
                        unsafe { *not_found.get_unchecked(1) },
                        unsafe { *not_found.get_unchecked(2) },
                        None,
                        internal,
                    )
                    .await
                {
                    return answer;
                };
            }
        }
        Answer::None
    }

    async fn invoke(&mut self, module_id: i64, class_id: i64, action_id: i64, param: Option<String>, internal: bool) -> Option<Answer> {
        if let Some(m) = self.engine.get(&module_id) {
            if let Some(c) = m.get(&class_id) {
                if let Some(a) = c.get(&action_id) {
                    if self.current_module_id == module_id && self.current_class_id == class_id {
                        let i = self.internal;
                        let p = match param {
                            Some(str) => self.route.param.replace(str),
                            None => self.route.param.take(),
                        };
                        self.internal = internal;
                        let res = a(self).await;
                        self.internal = i;
                        self.route.param = p;
                        return Some(res);
                    } else {
                        #[cfg(feature = "html-static")]
                        let h = match self.template.list.get(&module_id) {
                            Some(h) => match h.get(&class_id) {
                                Some(h) => self.html.replace(Arc::clone(h)),
                                None => self.html.take(),
                            },
                            None => self.html.take(),
                        };
                        #[cfg(feature = "html-reload")]
                        let h = match self.template.read().await.list.get(&module_id) {
                            Some(h) => match h.get(&class_id) {
                                Some(h) => self.html.replace(Arc::clone(h)),
                                None => self.html.take(),
                            },
                            None => self.html.take(),
                        };

                        #[cfg(any(feature = "lang-static", feature = "lang-reload"))]
                        let lang_id = self.lang_id;

                        #[cfg(feature = "lang-static")]
                        let l = match self.language.list.get(&lang_id) {
                            Some(l) => match l.get(&module_id) {
                                Some(l) => match l.get(&class_id) {
                                    Some(l) => self.lang.replace(Arc::clone(l)),
                                    None => self.lang.take(),
                                },
                                None => self.lang.take(),
                            },
                            None => self.lang.take(),
                        };
                        #[cfg(feature = "lang-reload")]
                        let l = match self.language.read().await.list.get(&lang_id) {
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
                            Some(str) => self.route.param.replace(str),
                            None => self.route.param.take(),
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
                        self.internal = i;
                        self.route.param = p;

                        #[cfg(any(feature = "html-static", feature = "html-reload"))]
                        {
                            self.html = h;
                        }
                        #[cfg(any(feature = "lang-static", feature = "lang-reload"))]
                        {
                            self.lang = l;
                        }
                        return Some(res);
                    }
                }
            }
        }
        None
    }

    fn extract_route(request: &Request, index: Arc<[i64; 3]>) -> Route {
        if request.url != "/" {
            let mut load: Vec<&str> = request.url.splitn(5, '/').collect();
            load.retain(|&x| !x.is_empty());
            match load.len() {
                1 => {
                    let module = unsafe { *load.get_unchecked(0) };
                    Route {
                        module_id: fnv1a_64(module.as_bytes()),
                        class_id: unsafe { *index.get_unchecked(1) },
                        action_id: unsafe { *index.get_unchecked(2) },
                        param: None,
                        #[cfg(any(feature = "lang-static", feature = "lang-reload"))]
                        lang_id: None,
                    }
                }
                2 => {
                    let module = unsafe { *load.get_unchecked(0) };
                    let class = unsafe { *load.get_unchecked(1) };
                    Route {
                        module_id: fnv1a_64(module.as_bytes()),
                        class_id: fnv1a_64(class.as_bytes()),
                        action_id: unsafe { *index.get_unchecked(2) },
                        param: None,
                        #[cfg(any(feature = "lang-static", feature = "lang-reload"))]
                        lang_id: None,
                    }
                }
                3 => {
                    let module = unsafe { *load.get_unchecked(0) };
                    let class = unsafe { *load.get_unchecked(1) };
                    let action = unsafe { *load.get_unchecked(2) };
                    Route {
                        module_id: fnv1a_64(module.as_bytes()),
                        class_id: fnv1a_64(class.as_bytes()),
                        action_id: fnv1a_64(action.as_bytes()),
                        param: None,
                        #[cfg(any(feature = "lang-static", feature = "lang-reload"))]
                        lang_id: None,
                    }
                }
                4 => {
                    let module = unsafe { *load.get_unchecked(0) };
                    let class = unsafe { *load.get_unchecked(1) };
                    let action = unsafe { *load.get_unchecked(2) };
                    let param = unsafe { *load.get_unchecked(3) };
                    Route {
                        module_id: fnv1a_64(module.as_bytes()),
                        class_id: fnv1a_64(class.as_bytes()),
                        action_id: fnv1a_64(action.as_bytes()),
                        param: Some(param.to_owned()),
                        #[cfg(any(feature = "lang-static", feature = "lang-reload"))]
                        lang_id: None,
                    }
                }
                _ => Route {
                    module_id: unsafe { *index.get_unchecked(0) },
                    class_id: unsafe { *index.get_unchecked(1) },
                    action_id: unsafe { *index.get_unchecked(2) },
                    param: None,
                    #[cfg(any(feature = "lang-static", feature = "lang-reload"))]
                    lang_id: None,
                },
            }
        } else {
            Route {
                module_id: unsafe { *index.get_unchecked(0) },
                class_id: unsafe { *index.get_unchecked(1) },
                action_id: unsafe { *index.get_unchecked(2) },
                param: None,
                #[cfg(any(feature = "lang-static", feature = "lang-reload"))]
                lang_id: None,
            }
        }
    }

    /// Simple remove temp file
    #[cfg(feature = "file-disk")]
    async fn clean_file(file: Arc<Vec<WebFile>>) {
        for f in &*file {
            if let Err(e) = remove_file(&f.tmp).await {
                if e.kind() != ErrorKind::NotFound {
                    log!(warning, 0, "filename={:?}. Error={}", f.tmp, e);
                }
            };
        }
    }
}
