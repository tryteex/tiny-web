use std::{collections::HashMap, net::IpAddr, path::PathBuf, str::FromStr, sync::Arc};

use serde::Serialize;

#[cfg(all(feature = "route-db", feature = "cache"))]
use crate::log;

#[cfg(all(feature = "route-db", feature = "cache"))]
use super::cache::Cache;

#[cfg(all(feature = "route-db", feature = "cache"))]
use super::data::Data;

#[derive(Debug, Clone, PartialEq)]
pub enum HttpVersion {
    None,
    HTTP1_0,
    HTTP1_1,
}

impl HttpVersion {
    pub fn get_status(&self) -> &'static str {
        match self {
            HttpVersion::None => "Status:",
            HttpVersion::HTTP1_0 => "HTTP/1.0",
            HttpVersion::HTTP1_1 => "HTTP/1.1",
        }
    }
}

#[derive(Debug, Clone)]
pub enum HttpMethod {
    Get,
    Head,
    Post,
    Put,
    Delete,
    Connect,
    Options,
    Trace,
    Patch,
    Other(String),
}

impl FromStr for HttpMethod {
    type Err = ();

    fn from_str(method: &str) -> Result<Self, Self::Err> {
        Ok(match method {
            "GET" => HttpMethod::Get,
            "HEAD" => HttpMethod::Head,
            "POST" => HttpMethod::Post,
            "PUT" => HttpMethod::Put,
            "DELETE" => HttpMethod::Delete,
            "CONNECT" => HttpMethod::Connect,
            "OPTIONS" => HttpMethod::Options,
            "TRACE" => HttpMethod::Trace,
            "PATCH" => HttpMethod::Patch,
            _ => HttpMethod::Other(method.to_owned()),
        })
    }
}

#[derive(Debug, Serialize)]
pub struct WebFile {
    pub name: String,
    pub file: String,
    pub size: usize,
    #[cfg(feature = "file-disk")]
    pub tmp: std::path::PathBuf,
    #[cfg(feature = "file-memory")]
    pub data: Vec<u8>,
}

#[derive(Debug)]
pub enum RawData {
    None,
    Raw(Vec<u8>),
}

#[derive(Debug)]
pub struct Input {
    pub get: Arc<HashMap<String, String>>,
    pub post: Arc<HashMap<String, String>>,
    pub file: Arc<Vec<WebFile>>,
    pub cookie: Arc<HashMap<String, String>>,
    pub params: Arc<HashMap<String, String>>,
    pub raw: Arc<RawData>,
}

#[derive(Debug)]
pub struct Request {
    pub ajax: bool,
    pub host: String,
    pub scheme: String,
    pub agent: String,
    pub referer: String,
    pub ip: Option<IpAddr>,
    pub method: HttpMethod,
    pub root: Arc<PathBuf>,
    pub url: String,
    pub input: Input,
    pub site: String,
    pub version: HttpVersion,
    pub content_type: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct Route {
    pub module_id: i64,
    pub class_id: i64,
    pub action_id: i64,
    pub param: Option<String>,
    #[cfg(any(feature = "lang-static", feature = "lang-reload"))]
    pub lang_id: Option<usize>,
}

impl Route {
    #[cfg(all(feature = "route-db", feature = "cache"))]
    pub(crate) async fn get(cache: Arc<Cache>, key: &str) -> Option<Option<Route>> {
        let data = cache.get(key).await?;
        match data {
            Data::None => return Some(None),
            Data::Vec(vec) => {
                #[cfg(not(any(feature = "lang-static", feature = "lang-reload")))]
                let len = 4;
                #[cfg(any(feature = "lang-static", feature = "lang-reload"))]
                let len = 5;
                if vec.len() == len {
                    let route = (
                        unsafe { vec.get_unchecked(0) },
                        unsafe { vec.get_unchecked(1) },
                        unsafe { vec.get_unchecked(2) },
                        unsafe { vec.get_unchecked(3) },
                        #[cfg(any(feature = "lang-static", feature = "lang-reload"))]
                        unsafe {
                            vec.get_unchecked(4)
                        },
                    );
                    match route {
                        #[cfg(not(any(feature = "lang-static", feature = "lang-reload")))]
                        (Data::I64(module_id), Data::I64(class_id), Data::I64(action_id), Data::None) => {
                            return Some(Some(Route {
                                module_id: *module_id,
                                class_id: *class_id,
                                action_id: *action_id,
                                param: None,
                            }));
                        }
                        #[cfg(not(any(feature = "lang-static", feature = "lang-reload")))]
                        (Data::I64(module_id), Data::I64(class_id), Data::I64(action_id), Data::String(param)) => {
                            return Some(Some(Route {
                                module_id: *module_id,
                                class_id: *class_id,
                                action_id: *action_id,
                                param: Some(param.to_owned()),
                            }));
                        }
                        #[cfg(any(feature = "lang-static", feature = "lang-reload"))]
                        (Data::I64(module_id), Data::I64(class_id), Data::I64(action_id), Data::None, Data::None) => {
                            return Some(Some(Route {
                                module_id: *module_id,
                                class_id: *class_id,
                                action_id: *action_id,
                                param: None,
                                lang_id: None,
                            }));
                        }
                        #[cfg(any(feature = "lang-static", feature = "lang-reload"))]
                        (Data::I64(module_id), Data::I64(class_id), Data::I64(action_id), Data::None, Data::Usize(lang_id)) => {
                            return Some(Some(Route {
                                module_id: *module_id,
                                class_id: *class_id,
                                action_id: *action_id,
                                param: None,
                                lang_id: Some(*lang_id),
                            }));
                        }
                        #[cfg(any(feature = "lang-static", feature = "lang-reload"))]
                        (Data::I64(module_id), Data::I64(class_id), Data::I64(action_id), Data::String(param), Data::None) => {
                            return Some(Some(Route {
                                module_id: *module_id,
                                class_id: *class_id,
                                action_id: *action_id,
                                param: Some(param.to_owned()),
                                lang_id: None,
                            }));
                        }
                        #[cfg(any(feature = "lang-static", feature = "lang-reload"))]
                        (Data::I64(module_id), Data::I64(class_id), Data::I64(action_id), Data::String(param), Data::Usize(lang_id)) => {
                            return Some(Some(Route {
                                module_id: *module_id,
                                class_id: *class_id,
                                action_id: *action_id,
                                param: Some(param.to_owned()),
                                lang_id: Some(*lang_id),
                            }));
                        }
                        _ => {}
                    }
                }
                log!(warning, 0, "{}", key);
                cache.remove(key).await;
            }
            _ => {
                log!(warning, 0, "{}", key);
                cache.remove(key).await;
            }
        };
        None
    }

    #[cfg(all(feature = "route-db", feature = "cache"))]
    pub(crate) async fn set(cache: Arc<Cache>, key: &str, data: Option<&Route>) {
        let data = match data {
            Some(route) => {
                let param = match &route.param {
                    Some(url) => Data::String(url.to_owned()),
                    None => Data::None,
                };
                #[cfg(any(feature = "lang-static", feature = "lang-reload"))]
                let lang_id = match &route.lang_id {
                    Some(lang_id) => Data::Usize(*lang_id),
                    None => Data::None,
                };
                Data::Vec(vec![
                    Data::I64(route.module_id),
                    Data::I64(route.class_id),
                    Data::I64(route.action_id),
                    param,
                    #[cfg(any(feature = "lang-static", feature = "lang-reload"))]
                    lang_id,
                ])
            }
            None => Data::None,
        };
        cache.set(key, data).await;
    }
}
