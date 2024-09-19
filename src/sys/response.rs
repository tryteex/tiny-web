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

/// Response parameters
///
///  # Values
///
/// * `redirect: Option<Redirect>` - Redirect.
/// * `content_type: Option<String>` - Content type.
/// * `headers: Vec<String>` - Additional headers.
/// * `http_code: Option<u16>` - Http code.
/// * `css: Vec<String>` - Addition css.
/// * `js: Vec<String>` - Addition js.
/// * `mata: Vec<String>` - Addition meta.
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
    /// Addition css
    pub css: Vec<String>,
    /// Addition js
    pub js: Vec<String>,
    /// Addition meta
    pub meta: Vec<String>,
}
