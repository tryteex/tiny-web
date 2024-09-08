## Cache system
Data cache is stored in the system's RAM.  
All data for the cache must adhere to the format specified in the [__Data__](https://github.com/tryteex/tiny-web/blob/main/doc/Data.md) documentation.

Upon restarting the application, the cache is not preserved.

Available Properties and Functions:
* `cache.get(key)`: Retrieve cache data, cloned it.
* `cache.set(key, value)`: Set cache data.
* `cache.remove(key)`: Remove cache data.
* `cache.clear()`: Clear all cache's data.
___
### Example
```rust
pub async fn index(this: &mut Action) -> Answer {
    this.cache.get("product_en_444_url", Data::String("/en/notebook_asus_iron_s150_p444".to_owned()));
    ...
}
```
Cache keys have a hierarchical structure. The symbol `:` is used to separate keys in the hierarchy. Hierarchy is useful when it's necessary to clear cache at a certain level.

For example: cache keys for permissions are formed in the format `auth:user_role_id:module_id:class_id:action_id`. And if it's necessary to clear all permissions but not reset other cache, then you need to execute:
```rust
this.cache.remove("auth").await
```
If it's necessary to clear all permissions only for `role_id=2`:
```rust
this.cache.remove("auth:2").await
```
___
Next => Request [https://github.com/tryteex/tiny-web/blob/main/doc/Request.md](https://github.com/tryteex/tiny-web/blob/main/doc/Request.md)  
Index => Contents [https://github.com/tryteex/tiny-web/blob/main/doc/Index.md](https://github.com/tryteex/tiny-web/blob/main/doc/Index.md)  