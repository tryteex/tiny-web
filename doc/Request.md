## HTTP request
The library automatically recognizes __GET__, __POST__, __FILES__, and __COOKIE__ data.

For __POST__ data, the library supports the HTTP headers `application/x-www-form-urlencoded` or `multipart/form-data; boundary=`.  
For __FILES__, it analyzes the HTTP header `multipart/form-data; boundary=`.

Additionally, other HTTP headers are automatically recognized.

Available Properties and Functions:
* `request.ajax` - Determines if the request is an Ajax request. Data can be obtained if the HTTP header __HTTP_X_REQUESTED_WITH__ is set to __XmlHttpRequest__.
* `request.host` - String from the HTTP header __HTTP_HOST__. 
* `request.scheme` - String from the HTTP header __REQUEST_SCHEME__.
* `request.agent` - String from the HTTP header __HTTP_USER_AGENT__.
* `request.referer` - String from the HTTP header __HTTP_REFERER__.
* `request.ip` - String from the HTTP header __REMOTE_ADDR__.
* `request.method` - *РString from the HTTP header __REQUEST_METHOD__.
* `request.path` - String from the HTTP header __DOCUMENT_ROOT__.
* `request.url` - String from the HTTP header __REDIRECT_URL__ up to the '?' character.
* `request.input` -  Input data including __GET__, __POST__, __FILES__ and __COOKIE__ data.

To retrieve data, use the corresponding method:
* `request.input.get` - Input __GET__ data obtained from the HTTP header __QUERY_STRING__.
* `request.input.post` - Input  __POST__ data.
* `request.input.file` - Input  __FILES__ data.  
Each file consists of:
  * `size` - File size.
  * `name` - File name.
  * `tmp` - Local path to the temporary file. It will be deleted after the session is closed.
* `request.input.cookie` - Input __COOKIE__ data obtained from the HTTP header __HTTP_COOKIE__, excluding session data.
* `request.input.param` - All other HTTP headers that are not included in the above list are added here.
___
### Example
```rust
pub async fn index(this: &mut Action) -> Answer {
    let product_str = this.request.input.get.get("product_id").unwrap();
    ...
}
```
___
Next => Response [Response.md](https://github.com/tryteex/tiny-web/blob/main/doc/Response.md)   
Index => Contents [Index.md](https://github.com/tryteex/tiny-web/blob/main/doc/Index.md)  