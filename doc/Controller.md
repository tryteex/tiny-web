## Controller
Each internet request is associated with a specific web controller. These controllers are grouped into a three-tier hierarchy: / __Module__ / __Class__ / __Action__ /.

To create a controller, it is necessary to create a new directory in `./src/app/` with the module name, for example, `api`.

Next, create a new file with the class name and extension `.rs`, for example, `product.rs`. So the path to the file will be: `./src/app/api/product.rs`.

To create a controller in this file, you need to create a function with the name of this controller using the following template:

```rust
pub async fn <controller_name>(this: &mut Action) -> Answer {

    Answer::None
}
```
For example, for the "get" controller:
```rust
pub async fn get(this: &mut Action) -> Answer {
    let param = this.param;
    Answer::None
}
```
There can be an unlimited number of controllers (__Action__) in one class.

The main functionality for the controller is presented on the __Action__ structure page  [Action.md](https://github.com/tryteex/tiny-web/blob/main/doc/Action.md).

All created class and controller files will be automatically added to the project. To do this, follow these steps:

1. Add the following dependencies to the `Cargo.toml` file:
```toml
[dependencies]
tiny-web-macro="0.1"
tiny-web="0.4"
```
2. Add the tiny_web::run macro to the `./src/main.rs` file:
```rust
pub mod app;

fn main() {
    tiny_web::run(
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION"),
        env!("CARGO_PKG_DESCRIPTION"),
        || {
            tiny_web_macro::addfn!();
        },
    );
} 
```
3. Create the `./src/app/mod.rs` file with the following content:

```rust
tiny_web_macro::addmod!();
```
___
Next => __Action__ structure [Action.md](https://github.com/tryteex/tiny-web/blob/main/doc/Action.md)  
Index => Contents [Index.md](https://github.com/tryteex/tiny-web/blob/main/doc/Index.md)  
