# tiny-web

`tiny-web` is a tiny async library (backend web server) that allows you to write a Laravel-style or Django-style backend in Rust language.

This library works only with FastCGI, SCGI and UWSGI (modifier1=0) protocols.
> **Note**  
> Support for GRPC, HTTP (hiding behind a Reverse Proxy), and WebSocket is also under development. Check out our repository on GitHub.

This library works with Postgresql 15+ database. But you can try the lower version.

The `install.sql` file is in the root of the project as a temporary solution so that the server can start. In the future, the `install` and `update` command will be added to the library to install and update the database.

## Documentation and examples

* `tiny-web` library and documentation https://rust.tiny.com.ua/ .

> **Note**  
> The sites are under construction, follow the projects, and check out our repository on GitHub.

## Installation

Add `tiny-web` to your `Cargo.toml` dependencies:

```toml
[dependencies]
tiny-web = "0.4"
tiny-web-macro = "0.1"
```

You also need to prepare a `tiny.conf` file in your web server. To do this, take the sample configuration file `tiny.sample.conf` and place it in the root of your project with the new name `tiny.conf`. And adjust the corresponding values. Be sure to change the `salt` parameter. In the future, the `tiny.conf` file will be created when the `install` command is executed.

## Usage

Just enter the following code to start the server

```rust
/// Actions (web controllers)
pub mod app;

fn main() {
    tiny_web::run(
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION"),
        env!("CARGO_PKG_DESCRIPTION"),
        || { tiny_web_macro::addfn!(); },
    );
}
```

## Web site

A commercial website will be created soon. The project can still be viewed on [https://github.com/tryteex/tiny-shop](https://github.com/tryteex/tiny-shop).

## Dependencies

The `tiny-web` library depends on a number of other packages. A full list can be found in the Cargo.toml file.

## Contributing

If you'd like to contribute to tiny-web, check out our repository on GitHub.

## License

This project is licensed under the MIT License - see the LICENSE file for details.

## Community

Our project is in its infancy, if you want to join us, welcome!  
https://discord.gg/E8vZhjUDg8