## HTTP response
To manage responses, the structure __Response__ is used. 

Available Properties:
* `response.redirect` - Specifies the address that will be generated in the HTTP response header __Location__. It's advisable to exit the controller immediately after using `response.redirect`.
* `response.content_type` - Adds data to the HTTP response header __Content-Type__.
* `response.headers` - Adds other headers to the HTTP response.
* `response.http_code` - Allows specifying a specific HTTP response code.
* `response.css` - A vector of strings containing paths to CSS files. It will be added to the controller's render as a CSS template variable.
* `response.js` - A vector of strings containing paths to JavaScript files. It will be added to the controller's render as a JavaScript template variable.
* `response.meta` - A vector of strings containing meta infotmations. It will be added to the controller's render as a `meta` template variable.
___
### Example
```rust
pub async fn index(this: &mut Action) -> Answer {
    this.response.redirect = Some(Redirect { url: "/admin".to_owned(), permanently: false });
    Answer::None
}
```
___
### Example
```rust
pub async fn index(this: &mut Action) -> Answer {
    this.response.content_type = Some("application/json".to_owned());
    this.set("pay", self.get_json_pay());
    this.response.css.push("<link href=\"/css/pay.css\" rel=\"stylesheet\" />".to_owned());
    this.response.http_code = Some(402);
    this.render("need_pay")
}
```
___
Next => Email system [https://github.com/tryteex/tiny-web/blob/main/doc/Email.md](https://github.com/tryteex/tiny-web/blob/main/doc/Email.md)  
Index => Contents [https://github.com/tryteex/tiny-web/blob/main/doc/Index.md](https://github.com/tryteex/tiny-web/blob/main/doc/Index.md)  