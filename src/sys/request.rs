use std::collections::HashMap;

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