## Session
User sessions are utilized for user identification through a cookie mechanism.

Sessions are automatically created upon the user's first login. Session data is stored in the database.  
Data loading from the session occurs automatically for each user.  
Data storage in the database takes place after sending a response to the user.

Available Properties and Functions:
* `session.user_id`: User identifier. `user_id=0` denotes a guest user.
* `session.key`: User cookie record.
* `session.lang_id`: Current user language.
* `session.read(key)`: Retrieve user data from the session for reading only.
* `session.get(key)`: Retrieve user data from the session, removing it afterward.
* `session.set(key, value)`: Load user data.
* `session.remove(key)`: Remove user data from the session.
* `session.clear()`: Clear the user's session.

___
### Example
```rust
pub async fn add_product_cart(this: &mut Action) -> Answer {
    
    let product_str = this.request.input.get.get("product_id").unwrap();
    let product_id = product_str.parse::<i64>().unwrap();
    
    let cart = match this.session.get("cart") {
        Some(Data::Map(mut cart)) => {
            match cart.entry(product_id) {
                Entry::Vacant(v) => {
                    let quantity = 1;
                    v.insert(Data::I32(quantity));
         
                },
                Entry::Occupied(mut o) => {
                    if let Data::I32(quantity) = o.get_mut() {
                        *quantity += 1;
                    } else {
                        let quantity = 1;
                        o.insert(Data::I32(quantity));
                    }
                },
            }
            cart
        },
        _ => {
            let mut cart = BTreeMap::new();
            let quantity = 1;
            cart.insert(product_id, Data::I32(quantity));
            cart
        },
    };
    this.session.set("cart", Data::Map(cart));
    ...
}
```
___
Next => Caching [Caching.md](https://github.com/tryteex/tiny-web/blob/main/doc/Caching.md)  
Index => Contents [Index.md](https://github.com/tryteex/tiny-web/blob/main/doc/Index.md)  