# tiny-web

`tiny-web` is a tiny async library (backend web server) that allows you to write a Laravel-style or Django-style backend in Rust language.

> **Info**  
> Short documentation for the library is [here](https://github.com/tryteex/tiny-web/blob/main/doc/Index.md).

> **Note**  
> For security reasons, this library must be located behind the Primary web server, for example, under Nginx.

This library works only with FastCGI, SCGI, and UWSGI (modifier1=0) protocols.
> **Note**  
> Support for GRPC, HTTP (hiding behind a Reverse Proxy), and WebSocket is also under development. Check out our repository on GitHub.
>
> In addition, testing is performed exclusively under Nginx.

This library works with Postgresql 15+ database. But you can try the lower version.

The `install.sql` file is in the root of the project as a temporary solution so that the server can start. In the future, the `install` and `update` command will be added to the library to install and update the database.

## Contributing

If you'd like to contribute to tiny-web, check out our repository on GitHub.

## License

This project is licensed under the MIT License - see the LICENSE file for details.