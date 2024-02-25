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
___
Next => Request [Request.md](https://github.com/tryteex/tiny-web/blob/main/doc/Request.md)  
Index => Contents [Index.md](https://github.com/tryteex/tiny-web/blob/main/doc/Index.md)  