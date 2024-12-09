#[cfg(all(feature = "redirect-db", feature = "cache"))]
use std::sync::Arc;

#[cfg(all(feature = "redirect-db", feature = "cache"))]
use crate::log;

#[cfg(all(feature = "redirect-db", feature = "cache"))]
use super::cache::Cache;

#[cfg(all(feature = "redirect-db", feature = "cache"))]
use super::data::Data;

#[derive(Debug, Clone)]
pub struct Redirect {
    pub url: String,
    pub permanently: bool,
}

#[derive(Debug)]
pub struct Response {
    pub redirect: Option<Redirect>,
    pub content_type: Option<String>,
    pub headers: Vec<(String, String)>,
    pub http_code: Option<u16>,
    pub css: Vec<String>,
    pub js: Vec<String>,
    pub meta: Vec<String>,
}

impl Redirect {
    #[cfg(all(feature = "redirect-db", feature = "cache"))]
    pub(crate) async fn get(cache: Arc<Cache>, key: &str) -> Option<Option<Redirect>> {
        let data = cache.get(key).await?;
        match data {
            Data::None => return Some(None),
            Data::Vec(vec) => {
                if vec.len() == 2 {
                    if let (Data::String(url), Data::Bool(permanently)) = unsafe { (vec.get_unchecked(0), vec.get_unchecked(1)) } {
                        return Some(Some(Redirect {
                            url: url.to_owned(),
                            permanently: *permanently,
                        }));
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

    #[cfg(all(feature = "redirect-db", feature = "cache"))]
    pub(crate) async fn set(cache: Arc<Cache>, key: &str, data: Option<&Redirect>) {
        let data = match data {
            Some(redirect) => Data::Vec(vec![Data::String(redirect.url.to_owned()), Data::Bool(redirect.permanently)]),
            None => Data::None,
        };
        cache.set(key, data).await;
    }
}
