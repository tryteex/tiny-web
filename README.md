# tiny-web

`tiny-web` is a tiny async library (backend web server) that allows you to write a Laravel-style or Django-style backend in Rust language.

## Installation

Add `tiny-web` to your `Cargo.toml` dependencies:

```toml
[dependencies]
tiny-web = "0.4.0"
```

## Usage

Just enter the following code to start the server

```rust
tiny_web::run(
    env!("CARGO_PKG_NAME"),
    env!("CARGO_PKG_VERSION"),
    env!("CARGO_PKG_DESCRIPTION"),
);
```

## Web site

A commercial website will be created soon. The project can still be viewed on [https://github.com/tryteex/tiny-shop](https://github.com/tryteex/tiny-shop).

## Dependencies

The `tiny-web` library depends on a number of other packages. A full list can be found in the Cargo.toml file.

## Contributing

If you'd like to contribute to tiny-web, check out our repository on GitHub.

## License

This project is licensed under the MIT License - see the LICENSE file for details.