use std::borrow::Cow;

#[cfg(any(feature = "mail-sendmail", feature = "mail-smtp", feature = "mail-file"))]
use std::sync::Arc;

use serde::Serialize;

#[cfg(feature = "mail-smtp")]
use lettre::transport::smtp::{
    authentication::{Credentials, Mechanism},
    client::{Tls, TlsParametersBuilder},
};

#[cfg(feature = "mail-file")]
use lettre::AsyncFileTransport;

#[cfg(feature = "mail-sendmail")]
use lettre::AsyncSendmailTransport;

#[cfg(feature = "mail-smtp")]
use lettre::AsyncSmtpTransport;

#[cfg(any(feature = "mail-sendmail", feature = "mail-smtp", feature = "mail-file"))]
use lettre::{AsyncTransport, Tokio1Executor};

#[cfg(any(feature = "mail-sendmail", feature = "mail-smtp", feature = "mail-file"))]
use lettre::{
    message::{header::ContentType, Attachment, Mailbox, MultiPart, SinglePart},
    Message,
};

#[cfg(feature = "mail-db")]
use tiny_web_macro::fnv1a_64 as m_fnv1a_64;

use crate::log;

#[cfg(feature = "mail-smtp")]
use crate::sys::app::init::{Auth as InitAuth, Tls as InitTls};

#[cfg(any(feature = "mail-sendmail", feature = "mail-smtp", feature = "mail-file"))]
use crate::sys::app::init::MailConfig;

#[cfg(any(feature = "mail-sendmail", feature = "mail-smtp", feature = "mail-file"))]
use crate::tool::generate_uuid;

#[cfg(feature = "mail-db")]
use super::action::Action;

/// Add file to the message struct.
///
/// # Values
///
/// * `name: String` - Name of file;
/// * `mime: Option<String>` - Mime type of file;
/// * `data: Vec<u8>` - Data.
#[derive(Debug, Clone, Serialize)]
pub struct MailBodyFile<'a> {
    /// Name of file
    pub name: Cow<'a, str>,
    /// Mime type of file
    pub mime: Option<Cow<'a, str>>,
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
pub struct MailBodyHtml<'a> {
    /// Text part
    pub text: Option<Cow<'a, str>>,
    /// Html part
    pub html: Cow<'a, str>,
    /// List of inline files
    pub file: Vec<MailBodyFile<'a>>,
}

/// Types of email messages
#[derive(Debug, Clone, Serialize)]
pub enum MailBody<'a> {
    /// Text part
    Text(Cow<'a, str>),
    /// Html part
    Html(MailBodyHtml<'a>),
    /// File part
    File(MailBodyFile<'a>),
}

/// Email message
#[derive(Debug, Clone, Serialize)]
pub struct MailMessage<'a> {
    /// To
    pub to: Vec<Cow<'a, str>>,
    /// CC
    pub cc: Option<Vec<Cow<'a, str>>>,
    /// BCC
    pub bcc: Option<Vec<Cow<'a, str>>>,
    /// FROM
    pub from: Cow<'a, str>,
    /// REPLY-TO
    pub reply_to: Option<Cow<'a, str>>,
    /// SUBJECT
    pub subject: Cow<'a, str>,
    /// List of attachments
    pub body: Vec<MailBody<'a>>,
}

pub(crate) struct Mail;

impl Mail {
    #[cfg(any(feature = "mail-sendmail", feature = "mail-smtp", feature = "mail-file"))]
    pub(crate) async fn send(init: Arc<MailConfig>, host: &str, message: MailMessage<'_>) -> Result<(), ()> {
        #[cfg(feature = "mail-sendmail")]
        let sender = AsyncSendmailTransport::<Tokio1Executor>::new_with_command(&init.sendmail);
        #[cfg(feature = "mail-file")]
        let sender = AsyncFileTransport::<Tokio1Executor>::new(&init.path);
        #[cfg(feature = "mail-smtp")]
        let sender = {
            let sender = match init.tls {
                InitTls::None => match AsyncSmtpTransport::<Tokio1Executor>::relay(&init.server) {
                    Ok(s) => s.port(init.port),
                    Err(_e) => {
                        log!(warning, 0, "{}", _e);
                        return Err(());
                    }
                },
                InitTls::Start => {
                    let param = match TlsParametersBuilder::new(init.server.clone()).dangerous_accept_invalid_certs(true).build() {
                        Ok(param) => param,
                        Err(_e) => {
                            log!(warning, 0, "{}", _e);
                            return Err(());
                        }
                    };
                    match AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&init.server) {
                        Ok(s) => s.tls(Tls::Required(param)).port(init.port),
                        Err(_e) => {
                            log!(warning, 0, "{}", _e);
                            return Err(());
                        }
                    }
                }
                InitTls::Ssl => {
                    let param = match TlsParametersBuilder::new(init.server.clone()).dangerous_accept_invalid_certs(true).build() {
                        Ok(param) => param,
                        Err(_e) => {
                            log!(warning, 0, "{}", _e);
                            return Err(());
                        }
                    };
                    match AsyncSmtpTransport::<Tokio1Executor>::relay(&init.server) {
                        Ok(s) => s.tls(Tls::Wrapper(param)).port(init.port),
                        Err(_e) => {
                            log!(warning, 0, "{}", _e);
                            return Err(());
                        }
                    }
                }
            };
            let mut sender = match init.auth {
                InitAuth::None => sender.authentication(Vec::new()),
                InitAuth::Plain => sender.authentication(vec![Mechanism::Plain]),
                InitAuth::Login => sender.authentication(vec![Mechanism::Login]),
                InitAuth::XOAuth2 => sender.authentication(vec![Mechanism::Xoauth2]),
            };
            if init.auth != InitAuth::None {
                if let Some(user) = &init.user {
                    match &init.pwd {
                        Some(pwd) => sender = sender.credentials(Credentials::new(user.to_owned(), pwd.to_owned())),
                        None => sender = sender.credentials(Credentials::new(user.to_owned(), String::new())),
                    }
                }
            };
            sender.build()
        };

        let message = Mail::create_message(host, message)?;
        if let Err(_e) = sender.send(message).await {
            log!(warning, 0, "{}", _e);
            return Err(());
        }

        Ok(())
    }

    #[cfg(feature = "mail-db")]
    pub(crate) async fn send(action: &Action, message: MailMessage<'_>) -> Result<(), ()> {
        let json = match serde_json::to_value(&message) {
            Ok(json) => json,
            Err(_e) => {
                log!(warning, 0, "{}", _e);
                return Err(());
            }
        };

        #[cfg(not(all(feature = "session-db", feature = "access-db")))]
        let user_id = 0_i64;
        #[cfg(all(feature = "session-db", feature = "access-db"))]
        let user_id = action.session.user_id.unwrap_or(0) as i64;

        action.db.execute_prepare(m_fnv1a_64!("lib_mail_add"), &[&user_id, &json]).await;
        Ok(())
    }

    #[cfg(any(feature = "mail-sendmail", feature = "mail-smtp", feature = "mail-file"))]
    fn create_message_uuid(host: &str) -> String {
        let id = generate_uuid();
        format!("<{}@{}>", &id[..60], host)
    }

    /// Create text email message from struct MailMessage
    #[cfg(any(feature = "mail-sendmail", feature = "mail-smtp", feature = "mail-file"))]
    fn create_message(host: &str, message: MailMessage<'_>) -> Result<Message, ()> {
        let from = match message.from.parse::<Mailbox>() {
            Ok(f) => f,
            Err(_e) => {
                log!(warning, 0, "{}", _e);
                return Err(());
            }
        };

        let message_id = Mail::create_message_uuid(host);
        let mut mes = Message::builder().message_id(Some(message_id)).from(from);
        if let Some(rto) = message.reply_to {
            match rto.parse::<Mailbox>() {
                Ok(r) => mes = mes.reply_to(r),
                Err(_e) => {
                    log!(warning, 0, "{}", _e);
                    return Err(());
                }
            }
        }
        for to in message.to {
            match to.parse::<Mailbox>() {
                Ok(t) => mes = mes.to(t),
                Err(_e) => {
                    log!(warning, 0, "{}", _e);
                    return Err(());
                }
            }
        }
        if let Some(mail_cc) = message.cc {
            for cc in mail_cc {
                match cc.parse::<Mailbox>() {
                    Ok(c) => mes = mes.cc(c),
                    Err(_e) => {
                        log!(warning, 0, "{}", _e);
                        return Err(());
                    }
                }
            }
        }
        if let Some(mail_cc) = message.bcc {
            for cc in mail_cc {
                match cc.parse::<Mailbox>() {
                    Ok(c) => mes = mes.bcc(c),
                    Err(_e) => {
                        log!(warning, 0, "{}", _e);
                        return Err(());
                    }
                }
            }
        }
        mes = mes.subject(message.subject);
        let mes = if !message.body.is_empty() {
            let mut part = MultiPart::mixed().build();
            for body in message.body {
                match body {
                    MailBody::Text(s) => part = part.singlepart(SinglePart::plain(s.into_owned())),
                    MailBody::Html(html) => {
                        if html.text.is_none() && html.file.is_empty() {
                            part = part.singlepart(SinglePart::html(html.html.into_owned()));
                        } else {
                            let mut mp = MultiPart::alternative().build();
                            if let Some(s) = html.text {
                                mp = mp.singlepart(SinglePart::plain(s.into_owned()));
                            }
                            if html.file.is_empty() {
                                mp = mp.singlepart(SinglePart::html(html.html.into_owned()));
                            } else {
                                let mut m = MultiPart::related().build();
                                m = m.singlepart(SinglePart::html(html.html.into_owned()));
                                for f in html.file {
                                    let mime = match &f.mime {
                                        Some(m) => m,
                                        None => {
                                            let (_, ext) = f.name.rsplit_once('.').unwrap_or(("", ""));
                                            Mail::get_mime(ext)
                                        }
                                    };
                                    let ct = match ContentType::parse(mime) {
                                        Ok(ct) => ct,
                                        Err(_e) => {
                                            log!(warning, 0, "{}", _e);
                                            return Err(());
                                        }
                                    };
                                    let a = Attachment::new_inline(f.name.into_owned()).body(f.data, ct);
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
                                let (_, ext) = file.name.rsplit_once('.').unwrap_or(("", ""));
                                Mail::get_mime(ext)
                            }
                        };
                        let ct = match ContentType::parse(mime) {
                            Ok(ct) => ct,
                            Err(_e) => {
                                log!(warning, 0, "{}", _e);
                                return Err(());
                            }
                        };
                        let a = Attachment::new(file.name.into_owned()).body(file.data, ct);
                        part = part.singlepart(a);
                    }
                }
            }
            match mes.multipart(part) {
                Ok(mes) => mes,
                Err(_e) => {
                    log!(warning, 0, "{}", _e);
                    return Err(());
                }
            }
        } else {
            match mes.body(String::new()) {
                Ok(mes) => mes,
                Err(_e) => {
                    log!(warning, 0, "{}", _e);
                    return Err(());
                }
            }
        };
        Ok(mes)
    }

    /// Get mime from file extension
    #[cfg(any(feature = "mail-sendmail", feature = "mail-smtp", feature = "mail-file"))]
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
