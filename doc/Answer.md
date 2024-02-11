## Struct __Answer__
The __Answer__ structure represents the response that a controller should return after execution.
```rust
pub enum Answer {
    None,
    String(String),
    Raw(Vec<u8>),
}
```
* ```Answer::None``` - The controller does not return any response.
* ```Answer::String(String)``` - The controller returns a response as plain UTF-8 text.
* ```Raw(Vec<u8>)``` -  The controller returns a response as a byte array.  
This is useful when the controller returns binary data, such as images.

If a Redirect is specified in the controller, the library will generate the header:
```http
Status: 301 Moved Permanently
Location: /<new_location>
```
or 
```http
Status: 302 Found
Location: /<new_location>
```
Additionally, cookies with a duration of 1 year will be added:
```http
Set-Cookie: <Key_Name>=<Value>; Expires=<ONE_YEAR>; Max-Age=<ONE_YEAR>; path=/; domain=<host>; Secure; SameSite=none"
```
If the HTTP header ```Content-Type``` is specified, the library adds it; otherwise, the following will be added:
```http
Content-Type: text/html; charset=utf-8
```
If additional headers are added via [Response](https://github.com/tryteex/tiny-web/blob/main/doc/Response.md), they will be added as well.

Finally, the following headers are always added:
```http
Connection: Keep-Alive
Content-Length: <length>
```
___
Next => __Data__ structure [Data.md](https://github.com/tryteex/tiny-web/blob/main/doc/Data.md)   
Index => Contents [Index.md](https://github.com/tryteex/tiny-web/blob/main/doc/Index.md)  