use std::{collections::HashMap, str::FromStr};

use serde_json::Value;

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

/// Raw HTTP data if the HTTP POST data was not recognized
#[derive(Debug)]
pub enum RawData {
    None,
    Json(Value),
    String(String),
    Raw(Vec<u8>),
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
    /// Raw data
    pub raw: RawData,
}

/// Http version
#[derive(Debug, Clone, PartialEq)]
pub enum HttpVersion {
    None,
    HTTP1_0,
    HTTP1_1,
    HTTP2,
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
    pub method: HttpMethod,
    /// DOCUMENT_ROOT
    pub path: String,
    /// Request url. Example: /product/view/item/145
    pub url: String,
    /// Input http protocol datas
    pub input: Input,
    /// Site name. Example: https://example.com
    pub site: String,
    /// Http version
    pub version: HttpVersion,
}

/// HTTP Methods
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
