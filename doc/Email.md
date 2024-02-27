## Sending Email Messages
The library has a mechanism for sending email messages.  
All email messages are stored in the data table `mail`, even if the `mail:provider` field is not specified.  
It is not recommended to use this mechanism for mass mailings.

### Settings
To configure email, you need to change the data in the `setting` database table.

Settings:
* `mail:provider` - Email provider.  
Possible values:
  * `None` - do not send email;
  * `Sendmail` - use the sendmail system utility;
  * `SMTP` - Send via SMTP server;
  * `File` - Save to file.
* `mail:sendmail` - Path to the sendmail application. Used if `mail:provider = Sendmail`.
* `mail:file` - Path to the directory where email should be stored. Used if `mail:provider = File`.
* `mail:smtp:server` - SMTP server connection address. Used if `mail:provider = SMTP`.
* `mail:smtp:port` - Port for SMTP server connection. Used if `mail:provider = SMTP`.
* `mail:smtp:tls` - TLS protocol used on the SMTP server. Used if `mail:provider = SMTP`.  
Possible values: `None`, `STARTTLS`, `SSL/TLS`.
* `mail:smtp:auth` - Authentication type on the SMTP server. Used if `mail:provider = SMTP`.  
Possible values: `None`, `PLAIN`, `LOGIN`, `XOAUTH2`.
* `mail:smtp:user` - User for connection to the SMTP server. Used if `mail:provider = SMTP`.
* `mail:smtp:pwd` - User password for connection to the SMTP server. Used if `mail:provider = SMTP`.

### Sending Email

To send an email, you need to call the `this.mail(message)` function in the controller, where message is a `MailMessage` structure.
Available Properties of `MailMessage`:
* `to` - List of addresses to send email to.
* `cc` - List of addresses to send a copy of the email to.
* `bcc` - List of addresses to send a blind copy of the email to.
* `from` - Address from which to send the email.
* `reply_to` - Reply-to address to return replies to the email.
* `subject` - Email subject.
* `body` - Body of the email, which can be an enumeration of `MailBody`.

List of `MailBody`:
* `Text` - Message text in __text/plain__ format.
* `Html` - Message text in __text/html__ format. Represented in the `MailBodyHtml` structure.
* `File` - Attached file (not inline) that is attached to the email. Represented in the `MailBodyFile` structure.

Structure of `MailBodyHtml`:
* `text` - text in __text/plain__ format.
* `html` - text in __html__ format.
* `file` - List of files inserted into the middle of the html text (inline). Represented in the `MailBodyFile` structure

Structure of `MailBodyFile`:
* `name` - File name.
* `mime` - Mime type of the file.
* `data` - Array u8, binary content of the file.
___
### Example
```rust
pub async fn index(this: &mut Action) -> Answer {
    let mut f = std::fs::File::open("D:\\rust-logo.jpg").expect("no file found");
    let metadata = std::fs::metadata("D:\\rust-logo.jpg").expect("unable to read metadata");
    let mut buffer = vec![0; metadata.len() as usize];
    std::io::Read::read(&mut f, &mut buffer).expect("buffer overflow");

    let file = MailBodyFile {
        name: "123.jpg".to_owned(),
        mime: None,
        data: buffer,
    };

    let text = "Hello world! img".to_owned();
    let html = "<p><b>Hello</b>, <i>world</i>! <img src=cid:123.jpg></p>".to_owned();
    let html = MailBody::Html(MailBodyHtml {
        text: None,
        html,
        file: vec![file],
    });
    let text = MailBody::Text(text);
    let file1 = MailBody::File(MailBodyFile {
        name: "name1.txt".to_owned(),
        mime: None,
        data: "data1".to_owned().as_bytes().to_vec(),
    });
    let file2 = MailBody::File(MailBodyFile {
        name: "name2.txt".to_owned(),
        mime: None,
        data: "Another data".to_owned().as_bytes().to_vec(),
    });
    let mes = MailMessage {
        to: vec!["to@tiny.com.ua".to_owned()],
        cc: None,
        bcc: None,
        from: "From me <email@tiny.com.ua>".to_owned(),
        reply_to: None,
        subject: "Some subject тема".to_owned(),
        body: vec![html, text, file1, file2],
    };
    this.mail(mes).await;
}
```
___
Next => Configuring nginx [Nginx.md](https://github.com/tryteex/tiny-web/blob/main/doc/Nginx.md)  
Index => Contents [Index.md](https://github.com/tryteex/tiny-web/blob/main/doc/Index.md)  