use std::{path::Path, sync::Arc};

use lettre::{
    message::{header::ContentType, Attachment, Mailbox, MultiPart, SinglePart},
    transport::smtp::{
        authentication::{Credentials, Mechanism},
        client::{Tls, TlsParametersBuilder},
    },
    AsyncFileTransport, AsyncSendmailTransport, AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
};
use serde::Serialize;
use serde_json::Value;
use tokio::fs::create_dir_all;

use super::{log::Log, pool::DBPool};

/// Add file to the message struct.
///
/// # Values
///
/// * `name: String` - Name of file;
/// * `mime: Option<String>` - Mime type of file;
/// * `data: Vec<u8>` - Data.
#[derive(Debug, Clone, Serialize)]
pub struct MailBodyFile {
    /// Name of file
    pub name: String,
    /// Mime type of file
    pub mime: Option<String>,
    /// Data
    pub data: Vec<u8>,
}

/// Add html page to the message struct.
///
/// # Values
///
/// * `text: Option<String>` - Text part;
/// * `html: String` - Html part;
/// * `file: Vec<MailBodyFile>` - List of inline files.
#[derive(Debug, Clone, Serialize)]
pub struct MailBodyHtml {
    /// Text part
    pub text: Option<String>,
    /// Html part
    pub html: String,
    /// List of inline files
    pub file: Vec<MailBodyFile>,
}

/// Types of email messages
#[derive(Debug, Clone, Serialize)]
pub enum MailBody {
    /// Text part
    Text(String),
    /// Html part
    Html(MailBodyHtml),
    /// File part
    File(MailBodyFile),
}

/// Email message
#[derive(Debug, Clone, Serialize)]
pub struct MailMessage {
    /// To
    pub to: Vec<String>,
    /// CC
    pub cc: Option<Vec<String>>,
    /// BCC
    pub bcc: Option<Vec<String>>,
    /// FROM
    pub from: String,
    /// REPLY-TO
    pub reply_to: Option<String>,
    /// SUBJECT
    pub subject: String,
    /// List of attachments
    pub body: Vec<MailBody>,
}

/// Email config
#[derive(Debug, Clone)]
pub struct SmtpInfo {
    /// Server we are connecting to
    server: String,
    /// Port to connect to
    port: u16,
    /// TLS security configuration
    tls: Tls,
    /// Optional enforced authentication mechanism
    authentication: Vec<Mechanism>,
    /// Credentials
    credentials: Option<Credentials>,
}

/// Email provider
#[derive(Debug, Clone)]
pub enum MailProvider {
    /// Don't send emails
    None,
    /// Use sendmail
    Sendmail(String),
    /// Use SMTP protocol
    SMTP(SmtpInfo),
    /// Save to folder
    File(String),
}

/// Send email struct
#[derive(Debug)]
pub struct Mail {
    /// Provider
    pub provider: MailProvider,
}

impl Mail {
    /// Create new provider
    pub async fn new(db: Arc<DBPool>) -> Mail {
        Mail { provider: Mail::get_provider(db).await }
    }

    /// Get provider from database
    async fn get_provider(db: Arc<DBPool>) -> MailProvider {
        match db.query_fast(9, &[&1441962092377564137_i64]).await {
            // mail:provider
            Some(res) => {
                if !res.is_empty() {
                    let provider: String = unsafe { res.get_unchecked(0) }.get(0);
                    match provider.as_ref() {
                        "Sendmail" => {
                            match db.query_fast(9, &[&-3979813852156915759_i64]).await {
                                // mail:sendmail
                                Some(res) => {
                                    if !res.is_empty() {
                                        let path: String = unsafe { res.get_unchecked(0) }.get(0);
                                        if !path.is_empty() {
                                            MailProvider::File(path)
                                        } else {
                                            Log::warning(3011, Some("mail:sendmail".to_owned()));
                                            MailProvider::None
                                        }
                                    } else {
                                        Log::warning(3011, Some("mail:sendmail".to_owned()));
                                        MailProvider::None
                                    }
                                }
                                None => {
                                    Log::warning(3011, Some("mail:sendmail".to_owned()));
                                    MailProvider::None
                                }
                            }
                        }
                        "SMTP" => {
                            let server = match db.query_fast(9, &[&-390595084051732771_i64]).await {
                                // mail:smtp:server
                                Some(res) => {
                                    if !res.is_empty() {
                                        let server: String = unsafe { res.get_unchecked(0) }.get(0);
                                        if !server.is_empty() {
                                            server
                                        } else {
                                            Log::warning(3011, Some("mail:smtp:server".to_owned()));
                                            return MailProvider::None;
                                        }
                                    } else {
                                        Log::warning(3011, Some("mail:smtp:server".to_owned()));
                                        return MailProvider::None;
                                    }
                                }
                                None => {
                                    Log::warning(3011, Some("mail:smtp:server".to_owned()));
                                    return MailProvider::None;
                                }
                            };
                            let port = match db.query_fast(9, &[&-1521500012746197243_i64]).await {
                                // mail:smtp:port
                                Some(res) => {
                                    if !res.is_empty() {
                                        let port: String = unsafe { res.get_unchecked(0) }.get(0);
                                        match port.parse::<u16>() {
                                            Ok(port) => port,
                                            Err(_) => {
                                                Log::warning(3011, Some("mail:smtp:port".to_owned()));
                                                return MailProvider::None;
                                            }
                                        }
                                    } else {
                                        Log::warning(3011, Some("mail:smtp:port".to_owned()));
                                        return MailProvider::None;
                                    }
                                }
                                None => {
                                    Log::warning(3011, Some("mail:smtp:port".to_owned()));
                                    return MailProvider::None;
                                }
                            };
                            let tls = match db.query_fast(9, &[&4706107683829871299_i64]).await {
                                // mail:smtp:tls
                                Some(res) => {
                                    if !res.is_empty() {
                                        let tls: String = unsafe { res.get_unchecked(0) }.get(0);
                                        if !tls.is_empty() {
                                            match tls.as_ref() {
                                                "None" => Tls::None,
                                                "STARTTLS" => {
                                                    let param = match TlsParametersBuilder::new(server.clone())
                                                        .dangerous_accept_invalid_certs(true)
                                                        .dangerous_accept_invalid_hostnames(true)
                                                        .build()
                                                    {
                                                        Ok(param) => param,
                                                        Err(_) => {
                                                            Log::warning(3011, Some("mail:smtp:tls".to_owned()));
                                                            return MailProvider::None;
                                                        }
                                                    };
                                                    Tls::Required(param)
                                                }
                                                "SSL/TLS" => {
                                                    let param = match TlsParametersBuilder::new(server.clone())
                                                        .dangerous_accept_invalid_certs(true)
                                                        .dangerous_accept_invalid_hostnames(true)
                                                        .build()
                                                    {
                                                        Ok(param) => param,
                                                        Err(_) => {
                                                            Log::warning(3011, Some("mail:smtp:tls".to_owned()));
                                                            return MailProvider::None;
                                                        }
                                                    };
                                                    Tls::Wrapper(param)
                                                }
                                                _ => {
                                                    Log::warning(3011, Some("mail:smtp:tls".to_owned()));
                                                    return MailProvider::None;
                                                }
                                            }
                                        } else {
                                            Log::warning(3011, Some("mail:smtp:tls".to_owned()));
                                            return MailProvider::None;
                                        }
                                    } else {
                                        Log::warning(3011, Some("mail:smtp:tls".to_owned()));
                                        return MailProvider::None;
                                    }
                                }
                                None => {
                                    Log::warning(3011, Some("mail:smtp:tls".to_owned()));
                                    return MailProvider::None;
                                }
                            };
                            let auth = match db.query_fast(9, &[&-8449193462972437408_i64]).await {
                                // mail:smtp:auth
                                Some(res) => {
                                    if !res.is_empty() {
                                        let auth: String = unsafe { res.get_unchecked(0) }.get(0);
                                        if !auth.is_empty() {
                                            match auth.as_ref() {
                                                "None" => Vec::new(),
                                                "PLAIN" => vec![Mechanism::Plain],
                                                "LOGIN" => vec![Mechanism::Login],
                                                "XOAUTH2" => vec![Mechanism::Xoauth2],
                                                _ => {
                                                    Log::warning(3011, Some("mail:smtp:auth".to_owned()));
                                                    return MailProvider::None;
                                                }
                                            }
                                        } else {
                                            Log::warning(3011, Some("mail:smtp:auth".to_owned()));
                                            return MailProvider::None;
                                        }
                                    } else {
                                        Log::warning(3011, Some("mail:smtp:auth".to_owned()));
                                        return MailProvider::None;
                                    }
                                }
                                None => {
                                    Log::warning(3011, Some("mail:smtp:auth".to_owned()));
                                    return MailProvider::None;
                                }
                            };
                            let user = match db.query_fast(9, &[&1199393424318567565_i64]).await {
                                // mail:smtp:user
                                Some(res) => {
                                    if !res.is_empty() {
                                        let user: String = unsafe { res.get_unchecked(0) }.get(0);
                                        user
                                    } else {
                                        String::new()
                                    }
                                }
                                None => {
                                    Log::warning(3011, Some("mail:smtp:user".to_owned()));
                                    return MailProvider::None;
                                }
                            };
                            let pwd = match db.query_fast(9, &[&2346365514808828621_i64]).await {
                                // mail:smtp:pwd
                                Some(res) => {
                                    if !res.is_empty() {
                                        let pwd: String = unsafe { res.get_unchecked(0) }.get(0);
                                        pwd
                                    } else {
                                        String::new()
                                    }
                                }
                                None => {
                                    Log::warning(3011, Some("mail:smtp:pwd".to_owned()));
                                    return MailProvider::None;
                                }
                            };
                            let cred = if !auth.is_empty() { Some(Credentials::new(user, pwd)) } else { None };
                            MailProvider::SMTP(SmtpInfo {
                                server,
                                port,
                                tls,
                                authentication: auth,
                                credentials: cred,
                            })
                        }
                        "File" => {
                            match db.query_fast(9, &[&-4738603782623769110_i64]).await {
                                // mail:file
                                Some(res) => {
                                    if !res.is_empty() {
                                        let path: String = unsafe { res.get_unchecked(0) }.get(0);
                                        if !path.is_empty() {
                                            if !Path::new(&path).is_dir() {
                                                if let Err(e) = create_dir_all(&path).await {
                                                    Log::warning(3015, Some(e.to_string()));
                                                    return MailProvider::None;
                                                }
                                            }
                                            MailProvider::File(path)
                                        } else {
                                            Log::warning(3011, Some("mail:file".to_owned()));
                                            MailProvider::None
                                        }
                                    } else {
                                        Log::warning(3011, Some("mail:file".to_owned()));
                                        MailProvider::None
                                    }
                                }
                                None => {
                                    Log::warning(3011, Some("mail:file".to_owned()));
                                    MailProvider::None
                                }
                            }
                        }
                        "None" => MailProvider::None,
                        _ => {
                            Log::warning(3011, Some("mail:provider".to_owned()));
                            MailProvider::None
                        }
                    }
                } else {
                    Log::warning(3011, Some("mail:provider".to_owned()));
                    MailProvider::None
                }
            }
            None => {
                Log::warning(3011, Some("mail:provider".to_owned()));
                MailProvider::None
            }
        }
    }

    /// Send email
    pub async fn send(provider: MailProvider, db: Arc<DBPool>, message: MailMessage, user_id: i64, host: String) -> bool {
        let json = match serde_json::to_value(&message) {
            Ok(json) => json,
            Err(e) => {
                Log::warning(3002, Some(format!("Error: {}\nMessage: {:?}. ", e, message)));
                return false;
            }
        };
        let mut id: i64 = 0;
        match provider {
            MailProvider::Sendmail(path) => {
                match Mail::create_message(Arc::clone(&db), message, &json, user_id, host, &mut id, "Sendmail").await {
                    Ok(mes) => {
                        let sender = AsyncSendmailTransport::<Tokio1Executor>::new_with_command(path);
                        match sender.send(mes).await {
                            Ok(_) => {
                                db.query_fast(11, &[&id]).await;
                                true
                            }
                            Err(e) => {
                                let e = Log::warning(3012, Some(format!("Error: {}", e)));
                                db.query_fast(10, &[&e, &id]).await;
                                false
                            }
                        }
                    }
                    Err(e) => {
                        if id > 0 {
                            db.query_fast(10, &[&e, &id]).await;
                        }
                        false
                    }
                }
            }
            MailProvider::SMTP(smtp) => {
                match Mail::create_message(Arc::clone(&db), message, &json, user_id, host, &mut id, "SMTP").await {
                    Ok(mes) => {
                        let mut sender = match &smtp.tls {
                            Tls::None => match AsyncSmtpTransport::<Tokio1Executor>::relay(&smtp.server) {
                                Ok(s) => s.port(smtp.port),
                                Err(e) => {
                                    let e = Log::warning(3014, Some(format!("Error: {}", e)));
                                    db.query_fast(10, &[&e, &id]).await;
                                    return false;
                                }
                            },
                            Tls::Required(_) => match AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&smtp.server) {
                                Ok(s) => s.tls(smtp.tls).port(smtp.port),
                                Err(e) => {
                                    let e = Log::warning(3014, Some(format!("Error: {}", e)));
                                    db.query_fast(10, &[&e, &id]).await;
                                    return false;
                                }
                            },
                            Tls::Wrapper(_) => match AsyncSmtpTransport::<Tokio1Executor>::relay(&smtp.server) {
                                Ok(s) => s.tls(smtp.tls).port(smtp.port),
                                Err(e) => {
                                    let e = Log::warning(3014, Some(format!("Error: {}", e)));
                                    db.query_fast(10, &[&e, &id]).await;
                                    return false;
                                }
                            },
                            Tls::Opportunistic(_) => match AsyncSmtpTransport::<Tokio1Executor>::relay(&smtp.server) {
                                Ok(s) => s.port(smtp.port),
                                Err(e) => {
                                    let e = Log::warning(3014, Some(format!("Error: {}", e)));
                                    db.query_fast(10, &[&e, &id]).await;
                                    return false;
                                }
                            },
                        };
                        if !smtp.authentication.is_empty() {
                            sender = sender.authentication(smtp.authentication);
                        }
                        if let Some(credentials) = smtp.credentials {
                            sender = sender.credentials(credentials);
                        }

                        match sender.build().send(mes).await {
                            Ok(_) => {
                                db.query_fast(11, &[&id]).await;
                                true
                            }
                            Err(e) => {
                                let e = Log::warning(3014, Some(format!("Error: {}", e)));
                                db.query_fast(10, &[&e, &id]).await;
                                false
                            }
                        }
                    }
                    Err(e) => {
                        if id > 0 {
                            db.query_fast(10, &[&e, &id]).await;
                        }
                        false
                    }
                }
            }
            MailProvider::File(path) => {
                match Mail::create_message(Arc::clone(&db), message, &json, user_id, host, &mut id, "File").await {
                    Ok(mes) => {
                        let sender = AsyncFileTransport::<Tokio1Executor>::new(path);
                        match sender.send(mes).await {
                            Ok(_) => {
                                db.query_fast(11, &[&id]).await;
                                true
                            }
                            Err(e) => {
                                let e = Log::warning(3013, Some(format!("Error: {}", e)));
                                db.query_fast(10, &[&e, &id]).await;
                                false
                            }
                        }
                    }
                    Err(e) => {
                        if id > 0 {
                            db.query_fast(10, &[&e, &id]).await;
                        }
                        false
                    }
                }
            }
            MailProvider::None => db.query_fast(8, &[&user_id, &json]).await.is_some(),
        }
    }

    /// Create text email message from struct MailMessage
    async fn create_message(
        db: Arc<DBPool>,
        message: MailMessage,
        json: &Value,
        user_id: i64,
        host: String,
        id: &mut i64,
        transport: &str,
    ) -> Result<Message, String> {
        let message_id = match db.query_fast(7, &[&user_id, &json, &transport]).await {
            Some(r) => {
                if r.len() != 1 {
                    Log::warning(3003, Some(format!("Message: {:?}.", &json)));
                    return Err("".to_owned());
                }
                *id = unsafe { r.get_unchecked(0).get(0) };
                format!("{}@{}", id, host)
            }
            None => return Err("".to_owned()),
        };

        let from = match message.from.parse::<Mailbox>() {
            Ok(f) => f,
            Err(e) => {
                let res = Log::warning(3004, Some(format!("Message: {:?}. Error: {}.", &json, e)));
                return Err(res);
            }
        };
        let mut mes = Message::builder().message_id(Some(message_id)).from(from);
        if let Some(rto) = message.reply_to {
            match rto.parse::<Mailbox>() {
                Ok(r) => mes = mes.reply_to(r),
                Err(e) => {
                    let res = Log::warning(3005, Some(format!("Message: {:?}. Error: {}.", &json, e)));
                    return Err(res);
                }
            }
        }
        for to in message.to {
            match to.parse::<Mailbox>() {
                Ok(t) => mes = mes.to(t),
                Err(e) => {
                    let res = Log::warning(3006, Some(format!("Message: {:?}. Error: {}.", &json, e)));
                    return Err(res);
                }
            }
        }
        if let Some(mail_cc) = message.cc {
            for cc in mail_cc {
                match cc.parse::<Mailbox>() {
                    Ok(c) => mes = mes.cc(c),
                    Err(e) => {
                        let res = Log::warning(3007, Some(format!("Message: {:?}. Error: {}.", &json, e)));
                        return Err(res);
                    }
                }
            }
        }
        if let Some(mail_cc) = message.bcc {
            for cc in mail_cc {
                match cc.parse::<Mailbox>() {
                    Ok(c) => mes = mes.bcc(c),
                    Err(e) => {
                        let res = Log::warning(3008, Some(format!("Message: {:?}. Error: {}.", &json, e)));
                        return Err(res);
                    }
                }
            }
        }
        mes = mes.subject(message.subject);
        let mes = if !message.body.is_empty() {
            let mut part = MultiPart::mixed().build();
            for body in message.body {
                match body {
                    MailBody::Text(s) => part = part.singlepart(SinglePart::plain(s)),
                    MailBody::Html(html) => {
                        if html.text.is_none() && html.file.is_empty() {
                            part = part.singlepart(SinglePart::html(html.html));
                        } else {
                            let mut mp = MultiPart::alternative().build();
                            if let Some(s) = html.text {
                                mp = mp.singlepart(SinglePart::plain(s));
                            }
                            if html.file.is_empty() {
                                mp = mp.singlepart(SinglePart::html(html.html));
                            } else {
                                let mut m = MultiPart::related().build();
                                m = m.singlepart(SinglePart::html(html.html));
                                for f in html.file {
                                    let mime = match &f.mime {
                                        Some(m) => m,
                                        None => {
                                            let ext = match f.name.rsplit_once('.') {
                                                Some((_, ext)) => ext,
                                                None => "",
                                            };
                                            Mail::get_mime(ext)
                                        }
                                    };
                                    let ct = match ContentType::parse(mime) {
                                        Ok(ct) => ct,
                                        Err(e) => {
                                            let res = Log::warning(3010, Some(format!("Message: {:?}. Error: {}.", &json, e)));
                                            return Err(res);
                                        }
                                    };
                                    let a = Attachment::new_inline(f.name).body(f.data, ct);
                                    m = m.singlepart(a);
                                }
                                mp = mp.multipart(m);
                            }
                            part = part.multipart(mp);
                        }
                    }
                    MailBody::File(file) => {
                        let mime = match &file.mime {
                            Some(m) => m,
                            None => {
                                let ext = match file.name.rsplit_once('.') {
                                    Some((_, ext)) => ext,
                                    None => "",
                                };
                                Mail::get_mime(ext)
                            }
                        };
                        let ct = match ContentType::parse(mime) {
                            Ok(ct) => ct,
                            Err(e) => {
                                let res = Log::warning(3010, Some(format!("Message: {:?}. Error: {}.", &json, e)));
                                return Err(res);
                            }
                        };
                        let a = Attachment::new(file.name).body(file.data, ct);
                        part = part.singlepart(a);
                    }
                }
            }
            match mes.multipart(part) {
                Ok(mes) => mes,
                Err(e) => {
                    let res = Log::warning(3009, Some(format!("Message: {:?}. Error: {}.", &json, e)));
                    return Err(res);
                }
            }
        } else {
            match mes.body("".to_string()) {
                Ok(mes) => mes,
                Err(e) => {
                    let res = Log::warning(3009, Some(format!("Message: {:?}. Error: {}.", &json, e)));
                    return Err(res);
                }
            }
        };
        Ok(mes)
    }

    /// Get mime from file extension
    fn get_mime(ext: &str) -> &'static str {
        match ext {
            "7z" => "application/x-7z-compressed",
            "aac" => "audio/aac",
            "abw" => "application/x-abiword",
            "arc" => "application/x-freearc",
            "avi" => "video/x-msvideo",
            "avif" => "image/avif",
            "azw" => "application/vnd.amazon.ebook",
            "bin" => "application/octet-stream",
            "bmp" => "image/bmp",
            "bz" => "application/x-bzip",
            "bz2" => "application/x-bzip2",
            "cda" => "application/x-cdf",
            "csh" => "application/x-csh",
            "css" => "text/css",
            "csv" => "text/csv",
            "doc" => "application/msword",
            "docx" => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
            "eot" => "application/vnd.ms-fontobject",
            "epub" => "application/epub+zip",
            "gif" => "image/gif",
            "gz" => "application/gzip",
            "htm" => "text/html",
            "html" => "text/html",
            "ico" => "image/vnd.microsoft.icon",
            "ics" => "text/calendar",
            "jar" => "application/java-archive",
            "jpeg" => "image/jpeg",
            "jpg" => "image/jpeg",
            "js" => "text/javascript",
            "json" => "application/json",
            "jsonld" => "application/ld+json",
            "mjs" => "text/javascript",
            "mp3" => "audio/mpeg",
            "mp4" => "video/mp4",
            "mpeg" => "video/mpeg",
            "mpkg" => "application/vnd.apple.installer+xml",
            "odp" => "application/vnd.oasis.opendocument.presentation",
            "ods" => "application/vnd.oasis.opendocument.spreadsheet",
            "odt" => "application/vnd.oasis.opendocument.text",
            "oga" => "audio/ogg",
            "ogv" => "video/ogg",
            "ogx" => "application/ogg",
            "opus" => "audio/opus",
            "otf" => "font/otf",
            "pdf" => "application/pdf",
            "php" => "application/x-httpd-php",
            "png" => "image/png",
            "ppt" => "application/vnd.ms-powerpoint",
            "pptx" => "application/vnd.openxmlformats-officedocument.presentationml.presentation",
            "rar" => "application/vnd.rar",
            "rs" => "text/plain",
            "rtf" => "application/rtf",
            "sh" => "application/x-sh",
            "svg" => "image/svg+xml",
            "tar" => "application/x-tar",
            "tif" => "image/tiff",
            "tiff" => "image/tiff",
            "ts" => "video/mp2t",
            "ttf" => "font/ttf",
            "txt" => "text/plain",
            "vsd" => "application/vnd.visio",
            "wav" => "audio/wav",
            "weba" => "audio/webm",
            "webm" => "video/webm",
            "webp" => "image/webp",
            "woff" => "font/woff",
            "woff2" => "font/woff2",
            "xhtml" => "application/xhtml+xml",
            "xls" => "application/vnd.ms-excel",
            "xlsx" => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
            "xml" => "application/xml",
            "xul" => "application/vnd.mozilla.xul+xml",
            "zip" => "application/zip",
            _ => "application/octet-stream",
        }
    }
}
