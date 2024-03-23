## Struct __Action__
On this page, a comprehensive description of the functionality you will obtain from the __Action__ structure in controllers will be provided.

For example, to handle the request `/api/product/get`, you need to create a controller. To do this, add the following code to the file `./src/app/api/product.rs`:

```rust
pub async fn get(this: &mut Action) -> Answer {
    ...
}
```
In this case, the following functionality will be available for the variable __this__:
| Type | Name | Async| Return | Description |
|-|-|:-:|-|-|
| 𝑓 | load | yes || Invoking another controller |
| 𝑓 | lang | no | String | Retrieving a simple translation |
| 𝑓 | lang_current | no | &LangItem | Current user language |
| 𝑓 | lang_list | no | &Vec<LangItem> | List of languages |
| 𝑓 | get_access | yes | bool | Checking permissions for the controller |
| 𝑓 | not_found | yes | String | Retrieving the URL of the 404 Not Found controller |
| 𝑓 | set | no || Setting data for rendering an HTML page |
| 𝑓 | route | yes | String | Get the URL for the controller |
| 𝑓 | render | no | Answer | Rendering an HTML page |
| 𝑓 | mail | yes | bool| Sending an email |
| . | request || Request | Request parameters from the client and web server. More details [Request.md](https://github.com/tryteex/tiny-web/blob/main/doc/Request.md) |
| . | response || Response | Setting additional parameters for rendering an HTML page. More details [Response.md](https://github.com/tryteex/tiny-web/blob/main/doc/Response.md) |
| . | session || Session | Client session data. More details [Session.md](https://github.com/tryteex/tiny-web/blob/main/doc/Session.md) |
| . | salt || String | Project secret "salt" |
| . | module || String | Initial module of the call |
| . | class || String | Initial class of the call |
| . | action || String | Initial controller of the call |
| . | param || Option<String> | Parameters to the controller |
| . | internal || bool | Internal call (Controller was called by another controller) |
| . | db || DB | Executing queries to the database. More details [Database.md](https://github.com/tryteex/tiny-web/blob/main/doc/Database.md) |
| . | cache || Cache | Cache system. More details [Caching.md](https://github.com/tryteex/tiny-web/blob/main/doc/Caching.md) |

It is worth noting that data transfer between controllers is not applied. In other words, controllers should be independent and self-contained. However, for rendering an HTML page, data transfer is possible through the __set__ function. The data itself should be in the format of __Data__. More details [Data.md](https://github.com/tryteex/tiny-web/blob/main/doc/Data.md).

Now, let's delve deeper into each function:
___
### load
Invoking another controller.
```rust
fn load(key: &str, module: &str, class: &str, action: &str, param: Option<String>) 
```
* `key: &str` - The variable to which you need to set the data for rendering the html page.
* `module: &str` - Module to be called. 
* `class: &str` - Class to be called.
* `action: &str` - Acction to be called. 
* `param: Option<String>` - The option passed to the controller. 
#### Example
```rust
pub async fn get(this: &mut Action) -> Answer {
    let title = this.lang("title");
    this.load("head", "index", "index", "head", Some(title)).await;
    this.load("foot", "index", "index", "foot", None).await;
    ...
}
```
___
### lang
Retrieving a simple translation
```rust
fn lang(text: &str) -> String
```
* `text: &str` - Variable name for simple translation.
* __Return__: Returns a value depending on the set language.

More about translations in [I18N.md](https://github.com/tryteex/tiny-web/blob/main/doc/I18N.md)
#### Example
```rust
pub async fn get(this: &mut Action) -> Answer {
    let title = this.lang("title");
    this.load("head", "index", "index", "head", Some(title)).await;
    this.load("foot", "index", "index", "foot", None).await;
    ...
}
```
___
### get_access
Checking permissions for the controller
```rust
fn get_access(module: &str, class: &str, action: &str) -> bool
```
* `module: &str` - Module to be checked. 
* `class: &str` - Class to be checked.
* `action: &str` - Action to be checked. 
* __Return__: Returns __true__ if access is available.  
#### Example
```rust
pub async fn get(this: &mut Action) -> Answer {
    let show_menu_item = this.get_access("index", "menu", "permission").await;

    if show_menu_item {
        this.load("permission", "index", "menu", "permission", None).await;
    } else {
        this.set("permission", Data::None);
    }
    ...
}
```
___
### not_found
Retrieving the URL of the 404 Not Found controller
```rust
fn not_found() -> String
```
* __Return__: Returns url, for the controller ```/index/index/not_found```.  
#### Example
```rust
pub async fn head(this: &mut Action) -> Answer {
    if !this.internal {
        let url = this.not_found().await;
        this.response.redirect = Some(Redirect { url, permanently: false });
    }
    ...
}
```
___
### set
Setting data for rendering an HTML page
```rust
fn set(key: &str, value: Data)
```
* `key: &str` - The variable to set the data to render the html page.
* `value: Data` - Data for rendering the html page.  
More about __Data__ in [Data.md](https://github.com/tryteex/tiny-web/blob/main/doc/Data.md)

#### Example
```rust
pub async fn get(this: &mut Action) -> Answer {
    let show_menu_item = this.get_access("index", "menu", "permission").await;

    if show_menu_item {
        this.load("permission", "index", "menu", "permission", None).await;
    } else {
        this.set("permission", Data::None);
    }
    ...
}
```
___
### route
Get the URL for the controller
```rust
fn route(module: &str, class: &str, action: &str, param: Option<&str>, lang_id: i64) -> String
```
* `module: &str` - Module for which you want to get the URL. 
* `class: &str` - Class for which you want to get the URL.
* `action: &str` - Action for which you want to get the URL.
* `param: Option<String>` - Parameter to get the URL for. 
* `lang_id: i64` - The language to get the URL for.
* __Return__: Returns the url for the specified controller.
#### Example
```rust
pub async fn get(this: &mut Action) -> Answer {
    let url = this.route("index", "article", "get", this.param, this.session.lang_id).await;
    this.set("show_more", url);
    this.render("short_article")
}
```
___
### render
Rendering an HTML page
```rust
fn render(template: &str) -> Answer 
```
* `template: &str` - Template name. 
* __Return__: Returns __Answer__, for the specified template.  
More about __Answer__ [Answer.md](https://github.com/tryteex/tiny-web/blob/main/doc/Answer.md)  

Learn more about the template [Template.md](https://github.com/tryteex/tiny-web/blob/main/doc/Template.md) 
#### Example
```rust
pub async fn get(this: &mut Action) -> Answer {
    let url = this.route("index", "article", "get", this.param, this.session.lang_id).await;
    this.set("show_more", url);
    this.render("short_article")
}
```
___
### mail
Sending an email
```rust
fn mail(message: MailMessage) -> bool
```
* `message: MailMessage` - Message. 
* __Return__: Returns __true__ if the message has been sent.

More about __email__ [Email.md](https://github.com/tryteex/tiny-web/blob/main/doc/Email.md)
___
Next => __Answer__ structure [Answer.md](https://github.com/tryteex/tiny-web/blob/main/doc/Answer.md)  
Index => Contents [Index.md](https://github.com/tryteex/tiny-web/blob/main/doc/Index.md)  